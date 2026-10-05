//! Lector SDLTM de solo lectura sin SDK (Fase A del plan de memorias):
//! nunca escribe, nunca toca el original y rechaza con diagnóstico lo que no
//! puede modelar. La escritura/actualización de SDLTM sigue requiriendo el
//! puente de Trados Studio (`sdltm`).

use super::{MAX_TEXT, attr, escaped, invalid, parse_tu, text_event};
use crate::model::*;
use quick_xml::{Reader, events::Event};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

const MEMORY_COLUMNS: &[&str] = &["source_language", "target_language", "tucount"];
const UNIT_COLUMNS: &[&str] = &[
    "id",
    "source_segment",
    "target_segment",
    "creation_date",
    "creation_user",
    "change_date",
    "change_user",
    "last_used_date",
    "usage_counter",
];

pub fn import(
    source: &Path,
    source_lang: &str,
    target_lang: &str,
    cancel: &Cancellation,
    mut sink: impl FnMut(TmUnit) -> Result<()>,
) -> Result<usize> {
    cancel.check()?;
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    check_structure(&connection)?;
    let (declared_source, declared_target): (String, String) = connection.query_row(
        "SELECT source_language, target_language FROM translation_memories",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if !declared_source.eq_ignore_ascii_case(source_lang)
        || !declared_target.eq_ignore_ascii_case(target_lang)
    {
        return Err(CatError::Invalid(format!(
            "La SDLTM declara {declared_source}→{declared_target}; se solicitó {source_lang}→{target_lang}. Configura el par exacto antes de importar o actualizar."
        )));
    }
    let memories: i64 =
        connection.query_row("SELECT count(*) FROM translation_memories", [], |row| {
            row.get(0)
        })?;
    if memories != 1 {
        return Err(CatError::Invalid(format!(
            "se esperaba una única memoria por archivo SDLTM; se encontraron {memories}"
        )));
    }
    let declared_tucount: i64 =
        connection.query_row("SELECT tucount FROM translation_memories", [], |row| {
            row.get(0)
        })?;
    let mut statement = connection.prepare(
        "SELECT id, source_segment, target_segment, creation_date, creation_user, change_date, change_user, last_used_date, usage_counter FROM translation_units ORDER BY id",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let id: i64 = row.get(0)?;
        let source_segment: Option<String> = row.get(1)?;
        let target_segment: Option<String> = row.get(2)?;
        let creation_date: Option<String> = row.get(3)?;
        let creation_user: Option<String> = row.get(4)?;
        let change_date: Option<String> = row.get(5)?;
        let change_user: Option<String> = row.get(6)?;
        let last_used_date: Option<String> = row.get(7)?;
        let usage_counter: Option<i64> = row.get(8)?;
        let Some(source_segment) = source_segment.filter(|s| !s.is_empty()) else {
            return Err(CatError::Invalid(format!(
                "la unidad {id} no tiene segmento de origen; SDLTM corrupta o no estándar"
            )));
        };
        let Some(target_segment) = target_segment.filter(|s| !s.is_empty()) else {
            return Err(CatError::Invalid(format!(
                "la unidad {id} no tiene segmento de destino; SDLTM corrupta o no estándar"
            )));
        };
        let mut unit = String::from("<tu");
        push_attribute(
            &mut unit,
            "creationdate",
            tmx_date(&creation_date).as_deref(),
        )?;
        push_attribute(&mut unit, "creationid", creation_user.as_deref())?;
        push_attribute(&mut unit, "changedate", tmx_date(&change_date).as_deref())?;
        push_attribute(&mut unit, "changeid", change_user.as_deref())?;
        push_attribute(
            &mut unit,
            "lastusagedate",
            tmx_date(&last_used_date).as_deref(),
        )?;
        if let Some(usage) = usage_counter {
            unit.push_str(&format!(" usagecount=\"{usage}\""));
        }
        unit.push('>');
        for (language, segment) in [
            (declared_source.as_str(), source_segment),
            (declared_target.as_str(), target_segment),
        ] {
            unit.push_str(&format!(
                "<tuv xml:lang=\"{}\"><seg>{}</seg></tuv>",
                escaped(language)?,
                segment_tmx(&segment, cancel)?
            ));
        }
        unit.push_str("</tu>");
        let parsed = parse_tu(&unit, source_lang, target_lang, cancel)?
            .ok_or_else(|| invalid("SDLTM con unidad fuera del par declarado"))?;
        sink(parsed)?;
        count += 1;
    }
    if count as i64 != declared_tucount {
        return Err(CatError::Invalid(format!(
            "la SDLTM declara {declared_tucount} unidades pero se leyeron {count}; memoria inconsistente, importación rechazada"
        )));
    }
    Ok(count)
}

fn check_structure(connection: &Connection) -> Result<()> {
    for table in ["translation_memories", "translation_units"] {
        let exists: bool = connection
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1")?
            .exists([table])?;
        if !exists {
            return Err(CatError::Format(
                "el archivo no es una memoria SDLTM reconocida".into(),
            ));
        }
    }
    for (table, columns) in [
        ("translation_memories", MEMORY_COLUMNS),
        ("translation_units", UNIT_COLUMNS),
    ] {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
        let present: std::collections::HashSet<String> = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<_, _>>()?;
        if let Some(missing) = columns.iter().find(|column| !present.contains(**column)) {
            return Err(CatError::Invalid(format!(
                "esquema SDLTM desconocido: falta la columna {missing} en {table}; importa con el puente de Trados Studio o exporta a TMX"
            )));
        }
    }
    Ok(())
}

fn push_attribute(unit: &mut String, name: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value.filter(|v| !v.is_empty()) {
        unit.push_str(&format!(" {name}=\"{}\"", escaped(value)?));
    }
    Ok(())
}

fn tmx_part(value: Option<&str>, len: usize) -> Option<&str> {
    let value = value?;
    (value.len() == len && value.bytes().all(|b| b.is_ascii_digit())).then_some(value)
}

fn tmx_date(raw: &Option<String>) -> Option<String> {
    let raw = raw.as_deref()?.trim();
    let (date, time) = raw.split_once(' ')?;
    let mut date = date.split('-');
    let mut time = time.split(':');
    let parts = [
        tmx_part(date.next(), 4)?,
        tmx_part(date.next(), 2)?,
        tmx_part(date.next(), 2)?,
        "T",
        tmx_part(time.next(), 2)?,
        tmx_part(time.next(), 2)?,
        tmx_part(time.next(), 2)?,
        "Z",
    ];
    if date.next().is_some() || time.next().is_some() {
        return None;
    }
    Some(parts.concat())
}

/// Convierte el XML `Segment` de SDLTM al dialecto TMX inline de SDL
/// (`bpt i type x` / `ept i` / `ph type x`), conservando el XML original de
/// cada `Tag` como contenido nativo para que `raw_xml` sea fiel.
fn segment_tmx(raw: &str, cancel: &Cancellation) -> Result<String> {
    let mut reader = Reader::from_str(raw);
    let mut output = String::new();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut text = String::new();
    let mut tag_start = 0;
    let mut root_seen = false;
    let mut root_closed = false;
    loop {
        cancel.check()?;
        let before = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(invalid)?;
        match &event {
            Event::Start(start) => {
                let name = start.name().as_ref().to_vec();
                let lname = local(&name);
                let _ = attr(start, b"__none")?;
                if stack.is_empty() {
                    if root_seen || root_closed || lname != b"Segment" {
                        return Err(invalid("se requiere un único Segment SDLTM"));
                    }
                    root_seen = true;
                } else {
                    let valid = match stack.as_slice() {
                        [s] if local(s) == b"Segment" => {
                            matches!(lname, b"Elements" | b"CultureName")
                        }
                        [.., e] if local(e) == b"Elements" => matches!(lname, b"Text" | b"Tag"),
                        [.., t] if local(t) == b"Text" => lname == b"Value",
                        [.., g] if local(g) == b"Tag" => true,
                        _ => false,
                    };
                    if !valid {
                        return Err(invalid(format!(
                            "elemento «{}» inesperado dentro del Segment SDLTM",
                            String::from_utf8_lossy(&name)
                        )));
                    }
                }
                match lname {
                    b"Text" => text.clear(),
                    b"Tag" => tag_start = before,
                    _ => (),
                }
                if stack.len() >= 128 {
                    return Err(invalid("Segment SDLTM demasiado profundo"));
                }
                stack.push(name);
            }
            Event::Empty(start) => {
                let name = start.name().as_ref().to_vec();
                let lname = local(&name);
                let _ = attr(start, b"__none")?;
                let accepted = match stack.as_slice() {
                    [] => lname == b"Segment" && !root_seen && !root_closed,
                    [s] if local(s) == b"Segment" => {
                        matches!(lname, b"Elements" | b"CultureName")
                    }
                    [.., e] if local(e) == b"Elements" => lname == b"Text",
                    [.., t] if local(t) == b"Text" => lname == b"Value",
                    [.., g] if local(g) == b"Tag" => true,
                    _ => false,
                };
                if !accepted {
                    return Err(invalid(format!(
                        "elemento «{}» inesperado dentro del Segment SDLTM",
                        String::from_utf8_lossy(&name)
                    )));
                }
                if stack.is_empty() {
                    root_seen = true;
                    root_closed = true;
                }
                if lname == b"Text" {
                    text.clear();
                }
            }
            Event::End(end) => {
                if stack.pop().as_deref() != Some(end.name().as_ref()) {
                    return Err(invalid("Segment SDLTM sin balance"));
                }
                match local(end.name().as_ref()) {
                    b"Text" => output.push_str(&escaped(&text)?),
                    b"Tag" => output.push_str(&tag_tmx(
                        &raw[tag_start..reader.buffer_position() as usize],
                        cancel,
                    )?),
                    _ => (),
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Eof => break,
            _ => {
                let Some(piece) = text_event(&event)? else {
                    return Err(invalid("evento XML no soportado en Segment SDLTM"));
                };
                let inside = |name: &[u8]| stack.last().is_some_and(|n| local(n) == name);
                let inside_tag = stack.iter().any(|n| local(n) == b"Tag");
                if inside(b"Value") {
                    text.push_str(&piece);
                } else if !(inside_tag
                    || inside(b"CultureName")
                    || (stack.is_empty() && piece.trim().is_empty()))
                {
                    return Err(invalid("texto fuera de lugar en Segment SDLTM"));
                }
            }
        }
        if output.len() > MAX_TEXT || text.len() > MAX_TEXT {
            return Err(invalid("Segment SDLTM supera el límite de tamaño"));
        }
    }
    if !root_seen || !root_closed || !stack.is_empty() {
        return Err(invalid("Segment SDLTM incompleto"));
    }
    Ok(output)
}

fn local(name: &[u8]) -> &[u8] {
    name.rsplit(|b| *b == b':').next().unwrap_or(name)
}

fn attr_fragment(name: &str, value: Option<&str>, skip: impl Fn(&str) -> bool) -> Result<String> {
    match value.filter(|v| !skip(v)) {
        Some(value) => Ok(format!(" {name}=\"{}\"", escaped(value)?)),
        None => Ok(String::new()),
    }
}

fn tag_tmx(raw: &str, cancel: &Cancellation) -> Result<String> {
    let mut kind = None;
    let mut anchor = None;
    let mut alignment_anchor = None;
    let mut tag_id = None;
    let mut reader = Reader::from_str(raw);
    loop {
        cancel.check()?;
        let event = reader.read_event().map_err(invalid)?;
        let (Event::Start(start) | Event::Empty(start)) = &event else {
            if matches!(event, Event::Eof) {
                break;
            }
            continue;
        };
        let field = match local(start.name().as_ref()) {
            b"Type" => &mut kind,
            b"Anchor" => &mut anchor,
            b"AlignmentAnchor" => &mut alignment_anchor,
            b"TagID" => &mut tag_id,
            _ => continue,
        };
        let mut value = String::new();
        if matches!(event, Event::Start(_)) {
            loop {
                cancel.check()?;
                let inner = reader.read_event().map_err(invalid)?;
                match &inner {
                    Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) => {
                        if let Some(piece) = text_event(&inner)? {
                            value.push_str(&piece);
                        }
                    }
                    Event::End(end) if end.name() == start.name() => break,
                    Event::Comment(_) => (),
                    _ => return Err(invalid("campo de Tag SDLTM con contenido estructurado")),
                }
            }
        }
        *field = Some(value);
    }
    let kind = kind
        .as_deref()
        .ok_or_else(|| invalid("Tag SDLTM sin Type"))?;
    let content = escaped(raw)?;
    let type_attr = attr_fragment("type", tag_id.as_deref(), |v| v.is_empty())?;
    let x_attr = attr_fragment("x", alignment_anchor.as_deref(), |v| v == "0")?;
    match kind {
        "Start" | "End" => {
            let anchor = anchor
                .as_deref()
                .filter(|a| !a.is_empty())
                .ok_or_else(|| invalid(format!("Tag SDLTM {kind} sin Anchor")))?;
            let anchor = escaped(anchor)?;
            Ok(if kind == "Start" {
                format!("<bpt i=\"{anchor}\"{type_attr}{x_attr}>{content}</bpt>")
            } else {
                format!("<ept i=\"{anchor}\">{content}</ept>")
            })
        }
        "Standalone" => Ok(format!("<ph{type_attr}{x_attr}>{content}</ph>")),
        other => Err(invalid(format!(
            "variante de Tag SDLTM «{other}» no soportada por el lector nativo (LockedContent, TextPlaceholder…); importa con Trados Studio instalado o exporta a TMX"
        ))),
    }
}
