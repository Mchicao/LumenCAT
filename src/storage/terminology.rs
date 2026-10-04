use super::ProjectStore;
use crate::{model::*, terminology};
use rusqlite::params;
use std::collections::BTreeMap;

impl ProjectStore {
    pub fn term_bases(&self) -> Result<Vec<TermBase>> {
        Ok(self
            .connection
            .prepare("SELECT id,name,enabled FROM term_bases ORDER BY id")?
            .query_map([], |r| {
                Ok(TermBase {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    enabled: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_term_base(&mut self, name: &str) -> Result<i64> {
        self.writable()?;
        if name.trim().is_empty() || name.len() > 256 {
            return Err(CatError::Invalid(
                "El nombre de base debe tener entre 1 y 256 bytes".into(),
            ));
        }
        self.connection
            .execute("INSERT INTO term_bases(name) VALUES(?1)", [name.trim()])?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn set_term_base_enabled(&mut self, id: i64, enabled: bool) -> Result<()> {
        self.writable()?;
        if self.connection.execute(
            "UPDATE term_bases SET enabled=?2 WHERE id=?1",
            params![id, enabled],
        )? != 1
        {
            return Err(CatError::Invalid("No existe esa base terminológica".into()));
        }
        Ok(())
    }

    pub fn add_term_concept(
        &mut self,
        concept: &NewTermConcept,
        cancel: &Cancellation,
    ) -> Result<i64> {
        self.writable()?;
        cancel.check()?;
        if concept.expressions.is_empty()
            || concept.expressions.len() > 64
            || concept.domain.len() > 256
            || concept.notes.len() > 8192
            || concept.provenance.len() > 1024
        {
            return Err(CatError::Invalid("Concepto inválido: 1–64 expresiones, dominio hasta 256 bytes, notas 8 KiB y procedencia 1 KiB".into()));
        }
        for expression in &concept.expressions {
            validate_language(&expression.language)?;
            if expression.text.trim().is_empty()
                || expression.text.len() > 512
                || !expression.text.chars().any(char::is_alphanumeric)
            {
                return Err(CatError::Invalid(
                    "Cada expresión debe contener palabras y no superar 512 bytes".into(),
                ));
            }
        }
        let tx = self.connection.transaction()?;
        let count: usize =
            tx.query_row("SELECT count(*) FROM term_expressions", [], |r| r.get(0))?;
        if count + concept.expressions.len() > 10_000 {
            return Err(CatError::Invalid(
                "Este corte admite hasta 10.000 expresiones por proyecto".into(),
            ));
        }
        tx.execute(
            "INSERT INTO term_concepts(base_id,domain,notes,provenance) VALUES(?1,?2,?3,?4)",
            params![
                concept.base_id,
                concept.domain,
                concept.notes,
                concept.provenance
            ],
        )?;
        let id = tx.last_insert_rowid();
        for expression in &concept.expressions {
            cancel.check()?;
            tx.execute("INSERT INTO term_expressions(concept_id,language,text,status,case_sensitive) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(concept_id,language,text,status,case_sensitive) DO NOTHING", params![id,expression.language,expression.text,expression.status.as_str(),expression.case_sensitive])?;
        }
        cancel.check()?;
        tx.commit()?;
        Ok(id)
    }

    pub fn term_concepts(&self, sl: &str, tl: &str) -> Result<Vec<TermConcept>> {
        validate_language(sl)?;
        validate_language(tl)?;
        let mut stmt = self.connection.prepare("SELECT c.id,c.base_id,b.name,c.domain,c.notes,c.provenance,e.language,e.text,e.status,e.case_sensitive FROM term_concepts c JOIN term_bases b ON b.id=c.base_id JOIN term_expressions e ON e.concept_id=c.id WHERE b.enabled=1 AND EXISTS(SELECT 1 FROM term_expressions s WHERE s.concept_id=c.id AND s.language=?1) AND EXISTS(SELECT 1 FROM term_expressions t WHERE t.concept_id=c.id AND t.language=?2) ORDER BY c.id,e.id")?;
        let mut rows = stmt.query(params![sl, tl])?;
        let mut concepts = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let id = row.get(0)?;
            let concept = match concepts.entry(id) {
                std::collections::btree_map::Entry::Vacant(entry) => entry.insert(TermConcept {
                    id,
                    base_id: row.get(1)?,
                    base_name: row.get(2)?,
                    domain: row.get(3)?,
                    notes: row.get(4)?,
                    provenance: row.get(5)?,
                    expressions: Vec::new(),
                }),
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            };
            concept.expressions.push(TermExpression {
                language: row.get(6)?,
                text: row.get(7)?,
                status: TermStatus::parse(&row.get::<_, String>(8)?)?,
                case_sensitive: row.get(9)?,
            });
        }
        Ok(concepts.into_values().collect())
    }

    pub fn terminology(
        &self,
        document_id: i64,
        source: &str,
        target: &str,
        cancel: &Cancellation,
    ) -> Result<TerminologyResult> {
        cancel.check()?;
        let (sl, tl, format): (String, String, String) = self.connection.query_row(
            "SELECT source_lang,target_lang,format FROM documents WHERE id=?1",
            [document_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        terminology::check(
            &self.term_concepts(&sl, &tl)?,
            source,
            target,
            &sl,
            &tl,
            DocumentFormat::parse(&format)?,
            cancel,
        )
    }
}
