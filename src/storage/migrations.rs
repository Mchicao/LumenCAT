use crate::model::*;
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::path::{Path, PathBuf};

pub(super) const VERSION: i64 = 3;
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
    tx.execute_batch(V3)?;
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
