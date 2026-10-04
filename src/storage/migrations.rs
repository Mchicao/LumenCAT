use crate::model::*;
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::path::{Path, PathBuf};

pub(super) const VERSION: i64 = 5;
pub(super) const V2: &str = "
CREATE TABLE operations(id INTEGER PRIMARY KEY);
INSERT INTO operations SELECT id FROM history ORDER BY id;
ALTER TABLE history ADD COLUMN operation_id INTEGER REFERENCES operations(id);
UPDATE history SET operation_id=id;
CREATE INDEX history_operation ON history(operation_id,id);
";
pub(super) const V3: &str = "
CREATE TABLE settings(id INTEGER PRIMARY KEY CHECK(id=1),source_lang TEXT NOT NULL,target_lang TEXT NOT NULL);
INSERT INTO settings SELECT 1,coalesce((SELECT source_lang FROM documents ORDER BY id LIMIT 1),'en'),coalesce((SELECT target_lang FROM documents ORDER BY id LIMIT 1),'es');
";
pub(super) const V4: &str = "
CREATE TABLE memories(id INTEGER PRIMARY KEY,name TEXT NOT NULL,source_lang TEXT NOT NULL COLLATE NOCASE,target_lang TEXT NOT NULL COLLATE NOCASE,writable INTEGER NOT NULL DEFAULT 1,enabled INTEGER NOT NULL DEFAULT 1,UNIQUE(name,source_lang,target_lang));
INSERT INTO memories(name,source_lang,target_lang) SELECT 'Memoria importada',source_lang,target_lang FROM tm GROUP BY source_lang COLLATE NOCASE,target_lang COLLATE NOCASE;
ALTER TABLE tm ADD COLUMN memory_id INTEGER REFERENCES memories(id);
ALTER TABLE tm ADD COLUMN learned_segment_id INTEGER REFERENCES segments(id);
ALTER TABLE tm ADD COLUMN active INTEGER NOT NULL DEFAULT 1;
UPDATE tm SET memory_id=(SELECT id FROM memories WHERE memories.source_lang=tm.source_lang AND memories.target_lang=tm.target_lang AND name='Memoria importada');
INSERT INTO memories(name,source_lang,target_lang) SELECT 'Memoria del proyecto',source_lang,target_lang FROM settings;
ALTER TABLE settings ADD COLUMN write_memory_id INTEGER REFERENCES memories(id);
UPDATE settings SET write_memory_id=(SELECT id FROM memories WHERE name='Memoria del proyecto');
CREATE INDEX tm_contribution ON tm(learned_segment_id,memory_id,active);
CREATE INDEX tm_active_exact ON tm(source_lang COLLATE NOCASE,target_lang COLLATE NOCASE,source,active);
CREATE INDEX tm_active_normalized ON tm(source_lang COLLATE NOCASE,target_lang COLLATE NOCASE,normalized,active);
CREATE INDEX tm_active_length ON tm(source_lang COLLATE NOCASE,target_lang COLLATE NOCASE,chars,active);
CREATE TABLE history_tm(history_id INTEGER NOT NULL REFERENCES history(id) ON DELETE CASCADE,tm_id INTEGER NOT NULL REFERENCES tm(id),before_active INTEGER NOT NULL,after_active INTEGER NOT NULL,PRIMARY KEY(history_id,tm_id));
CREATE TRIGGER tm_delete AFTER DELETE ON tm BEGIN INSERT INTO tm_fts(tm_fts,rowid,source,source_lang,target_lang) VALUES('delete',old.id,old.source,old.source_lang,old.target_lang); END;
CREATE TRIGGER tm_update AFTER UPDATE OF source,source_lang,target_lang ON tm BEGIN INSERT INTO tm_fts(tm_fts,rowid,source,source_lang,target_lang) VALUES('delete',old.id,old.source,old.source_lang,old.target_lang); INSERT INTO tm_fts(rowid,source,source_lang,target_lang) VALUES(new.id,new.source,new.source_lang,new.target_lang); END;
";
pub(super) const V5: &str = "
CREATE TABLE term_bases(id INTEGER PRIMARY KEY,name TEXT NOT NULL UNIQUE,enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)));
CREATE TABLE term_concepts(id INTEGER PRIMARY KEY,base_id INTEGER NOT NULL REFERENCES term_bases(id),domain TEXT NOT NULL,notes TEXT NOT NULL,provenance TEXT NOT NULL);
CREATE INDEX term_concept_base ON term_concepts(base_id);
CREATE TABLE term_expressions(id INTEGER PRIMARY KEY,concept_id INTEGER NOT NULL REFERENCES term_concepts(id),language TEXT NOT NULL COLLATE NOCASE,text TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('preferred','allowed','forbidden')),case_sensitive INTEGER NOT NULL CHECK(case_sensitive IN (0,1)),UNIQUE(concept_id,language,text,status,case_sensitive));
CREATE INDEX term_expression_language ON term_expressions(language,concept_id);
";

pub(super) fn integrity(connection: &Connection) -> Result<()> {
    let status: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    let foreign_keys: bool = connection.prepare("PRAGMA foreign_key_check")?.exists([])?;
    if status != "ok" || foreign_keys {
        return Err(CatError::Invalid(
            "Proyecto inconsistente: conserva el original y abre una copia de respaldo".into(),
        ));
    }
    Ok(())
}

pub(super) fn backup(
    connection: &Connection,
    parent: &Path,
    prefix: &str,
    cancel: &Cancellation,
) -> Result<tempfile::NamedTempFile> {
    cancel.check()?;
    let temporary = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".lcat")
        .tempfile_in(parent)?;
    {
        let mut destination = Connection::open(temporary.path())?;
        {
            let backup = Backup::new(connection, &mut destination)?;
            loop {
                cancel.check()?;
                match backup.step(128)? {
                    StepResult::Done => break,
                    StepResult::More => (),
                    _ => {
                        return Err(CatError::Invalid(
                            "Respaldo ocupado; no se modificó el proyecto".into(),
                        ));
                    }
                }
            }
        }
        integrity(&destination)?;
        destination.pragma_update(None, "journal_mode", "DELETE")?;
    }
    temporary.as_file().sync_all()?;
    cancel.check()?;
    Ok(temporary)
}

pub(super) fn migrate(connection: &mut Connection, path: &Path, version: i64) -> Result<PathBuf> {
    integrity(connection)?;
    let counts = counts(connection)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!(
        "{}.backup-v{version}-",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    let backup = backup(connection, parent, &prefix, &Cancellation::default())?;
    let (_, backup_path) = backup.keep().map_err(|error| CatError::Io(error.error))?;
    tracing::info!(event = "migration_backup_created", version, backup = %backup_path.display());
    let tx = connection.transaction()?;
    if version < 2 {
        tx.execute_batch(V2)?;
    }
    if version < 3 {
        tx.execute_batch(V3)?;
    }
    if version < 4 {
        tx.execute_batch(V4)?;
    }
    tx.execute_batch(V5)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    if counts != self::counts(&tx)? {
        return Err(CatError::Invalid(
            "La migración alteró conteos; se conserva el respaldo".into(),
        ));
    }
    integrity(&tx)?;
    tx.commit()?;
    Ok(backup_path)
}

fn counts(connection: &Connection) -> Result<[i64; 4]> {
    Ok(connection.query_row("SELECT (SELECT count(*) FROM documents),(SELECT count(*) FROM segments),(SELECT count(*) FROM history),(SELECT count(*) FROM tm)", [], |row| Ok([row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?]))?)
}

pub(super) fn read_only(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}
