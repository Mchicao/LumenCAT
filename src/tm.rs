//! Bounded lexical retrieval; fuzzy scores are approximate, never context matches.
use crate::model::{CatError, Result, TmMatch, TmUnit};
use rusqlite::{Connection, params};
use unicode_normalization::UnicodeNormalization;

pub fn normalized(text: &str) -> String {
    text.nfc().collect()
}

pub fn insert(connection: &Connection, unit: &TmUnit) -> Result<usize> {
    if unit.source.len() > 1_048_576
        || unit.target.len() > 1_048_576
        || unit.raw_xml.len() > 4_194_304
    {
        return Err(CatError::Invalid(
            "Unidad TM demasiado grande para este MVP".into(),
        ));
    }
    // Un escritor; deduplicar idénticos preserva variantes con metadata diferente.
    Ok(connection.prepare_cached("INSERT INTO tm(source,target,source_lang,target_lang,normalized,chars,raw_xml) SELECT ?1,?2,?3,?4,?5,?6,?7 WHERE NOT EXISTS(SELECT 1 FROM tm WHERE source_lang=?3 AND target_lang=?4 AND source=?1 AND target=?2 AND raw_xml=?7)")?.execute(params![unit.source,unit.target,unit.source_lang,unit.target_lang,normalized(&unit.source),unit.source.chars().count(),unit.raw_xml])?)
}

fn collect(
    connection: &Connection,
    sql: &str,
    query: &str,
    sl: &str,
    tl: &str,
) -> Result<Vec<TmMatch>> {
    let mut statement = connection.prepare(sql)?;
    Ok(statement
        .query_map(params![query, sl, tl], |row| {
            Ok(TmMatch {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                score: 0.0,
                exact: false,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn lexical_query(query: &str) -> String {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .take(12)
        .map(|token| format!("\"{token}\""))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn selective_terms(connection: &Connection, source: &str) -> Result<Vec<String>> {
    let mut ranked = Vec::new();
    // Saturar frecuencia evita decodificar millones de postings de palabras comunes.
    // Solo decide rutas léxicas: no pretende contar frecuencia exacta de documento.
    let mut vocabulary = connection.prepare_cached("SELECT count(*) FROM (SELECT 1 FROM temp.tm_vocab WHERE term=?1 AND col='source' LIMIT 512)")?;
    for token in source
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .take(12)
    {
        let token = token.to_lowercase();
        let frequency = vocabulary.query_row([&token], |row| row.get::<_, i64>(0))?;
        if frequency > 0 {
            ranked.push((frequency, token));
        }
    }
    ranked.sort_unstable();
    ranked.dedup();
    // ponytail: two indexed lexical routes cap work; corpus recall remains approximate.
    Ok(ranked
        .into_iter()
        .take(2)
        .map(|(_, token)| format!("\"{token}\""))
        .collect())
}

fn language_query(query: &str, sl: &str, tl: &str) -> String {
    format!(
        "source_lang : \"{}\" AND target_lang : \"{}\" AND source : ({query})",
        sl.replace('"', "\"\""),
        tl.replace('"', "\"\"")
    )
}

pub fn matches(connection: &Connection, source: &str, sl: &str, tl: &str) -> Result<Vec<TmMatch>> {
    let mut results = collect(
        connection,
        "SELECT id,source,target FROM tm WHERE source=?1 AND source_lang=?2 AND target_lang=?3 ORDER BY id DESC LIMIT 8",
        source,
        sl,
        tl,
    )?;
    for item in &mut results {
        item.score = 100.0;
        item.exact = true;
    }
    if !results.is_empty() {
        return Ok(results);
    }
    let norm = normalized(source);
    let mut candidates = collect(
        connection,
        "SELECT id,source,target FROM tm WHERE normalized=?1 AND source_lang=?2 AND target_lang=?3 ORDER BY id DESC LIMIT 256",
        &norm,
        sl,
        tl,
    )?;
    for fts in selective_terms(connection, source)? {
        candidates.extend(collect(connection,"SELECT tm.id,tm.source,tm.target FROM tm_fts CROSS JOIN tm ON tm.id=tm_fts.rowid WHERE tm_fts MATCH ?1 AND tm.source_lang=?2 AND tm.target_lang=?3 LIMIT 128",&language_query(&fts,sl,tl),sl,tl)?);
    }
    // Indexed length range supplies bounded fallback for short text and CJK without word boundaries.
    let chars = source.chars().count();
    if candidates.len() < 64 {
        let mut statement=connection.prepare("SELECT id,source,target FROM tm WHERE source_lang=?1 AND target_lang=?2 AND chars BETWEEN ?3 AND ?4 ORDER BY chars LIMIT 256")?;
        candidates.extend(
            statement
                .query_map(
                    params![
                        sl,
                        tl,
                        chars.saturating_sub(chars / 4 + 2),
                        chars + chars / 4 + 2
                    ],
                    |row| {
                        Ok(TmMatch {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            target: row.get(2)?,
                            score: 0.0,
                            exact: false,
                        })
                    },
                )?
                .collect::<std::result::Result<Vec<_>, _>>()?,
        );
    }
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|item| seen.insert(item.id));
    // ponytail: quadratic scorer has a 16M cell budget; long queries have fewer candidates.
    candidates.truncate(
        256.min(16_000_000 / chars.saturating_mul(chars).max(1))
            .max(1),
    );
    for mut item in candidates {
        // Avoid quadratic edit distance on pathological segments; exact lookup remains available.
        if chars > 4096 || item.source.chars().count() > 4096 {
            continue;
        }
        item.score = 100.0 * strsim::normalized_levenshtein(&norm, &normalized(&item.source));
        if item.score >= 50.0 {
            results.push(item);
        }
    }
    results.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| b.id.cmp(&a.id)));
    results.truncate(8);
    Ok(results)
}

pub fn concordance(
    connection: &Connection,
    query: &str,
    sl: &str,
    tl: &str,
) -> Result<Vec<TmMatch>> {
    let fts = lexical_query(query);
    if fts.is_empty() {
        return Ok(Vec::new());
    }
    collect(
        connection,
        "SELECT tm.id,tm.source,tm.target FROM tm_fts CROSS JOIN tm ON tm.id=tm_fts.rowid WHERE tm_fts MATCH ?1 AND tm.source_lang=?2 AND tm.target_lang=?3 LIMIT 100",
        &language_query(&fts, sl, tl),
        sl,
        tl,
    )
}
