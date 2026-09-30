//! One owning worker holds the connection. Every edit and its undo record commit together.
use crate::{formats, model::*, tm};
use rusqlite::{Connection, params};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::Path,
};

const APPLICATION_ID: i64 = 0x4c434154;
const SCHEMA:&str="
CREATE TABLE session(id INTEGER PRIMARY KEY CHECK(id=1),clean INTEGER NOT NULL,cursor INTEGER NOT NULL DEFAULT 0);
INSERT INTO session VALUES(1,0,0);
CREATE TABLE documents(id INTEGER PRIMARY KEY,name TEXT NOT NULL,format TEXT NOT NULL,original BLOB NOT NULL,original_path TEXT,source_lang TEXT NOT NULL,target_lang TEXT NOT NULL);
CREATE TABLE segments(id INTEGER PRIMARY KEY,document_id INTEGER NOT NULL REFERENCES documents(id),ordinal INTEGER NOT NULL,external_id TEXT NOT NULL,source TEXT NOT NULL,target TEXT NOT NULL,state TEXT NOT NULL,locked INTEGER NOT NULL,origin TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 0,UNIQUE(document_id,ordinal));
CREATE TABLE history(id INTEGER PRIMARY KEY,segment_id INTEGER NOT NULL REFERENCES segments(id),before_target TEXT NOT NULL,before_state TEXT NOT NULL,before_locked INTEGER NOT NULL,before_origin TEXT NOT NULL,after_target TEXT NOT NULL,after_state TEXT NOT NULL,after_locked INTEGER NOT NULL,after_origin TEXT NOT NULL);
CREATE TABLE tm(id INTEGER PRIMARY KEY,source TEXT NOT NULL,target TEXT NOT NULL,source_lang TEXT NOT NULL,target_lang TEXT NOT NULL,normalized TEXT NOT NULL,chars INTEGER NOT NULL,raw_xml TEXT NOT NULL);
CREATE INDEX tm_exact ON tm(source_lang,target_lang,source);
CREATE INDEX tm_normalized ON tm(source_lang,target_lang,normalized);
CREATE INDEX tm_length ON tm(source_lang,target_lang,chars);
CREATE VIRTUAL TABLE tm_fts USING fts5(source,source_lang,target_lang,content=tm,content_rowid=id,tokenize='unicode61');
CREATE TRIGGER tm_insert AFTER INSERT ON tm BEGIN INSERT INTO tm_fts(rowid,source,source_lang,target_lang) VALUES(new.id,new.source,new.source_lang,new.target_lang); END;
";

pub struct ProjectStore {
    connection: Connection,
    _lock: File,
    pub recovered: bool,
    closed: bool,
}

impl ProjectStore {
    pub fn open(path: &Path) -> Result<Self> {
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            parent.canonicalize()?.join(
                path.file_name()
                    .ok_or_else(|| CatError::Invalid("Ruta de proyecto inválida".into()))?,
            )
        };
        let lock_path = canonical.with_extension("lumencat-lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock().map_err(|error| {
            CatError::Invalid(format!(
                "Proyecto abierto en otra sesión o bloqueo no disponible: {error}"
            ))
        })?;
        let connection = Connection::open(canonical)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let app: i64 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let objects: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if version > 1
            || (app != APPLICATION_ID && (app != 0 || objects != 0 || version != 0))
            || (app == APPLICATION_ID && version != 1)
        {
            return Err(CatError::Invalid(
                "Base ajena o versión de proyecto no soportada; no fue modificada".into(),
            ));
        }
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        if version == 0 {
            connection.execute_batch(&format!("BEGIN IMMEDIATE;{SCHEMA} PRAGMA application_id={APPLICATION_ID}; PRAGMA user_version=1; COMMIT;"))?;
        }
        let recovered =
            connection.query_row("SELECT clean=0 FROM session WHERE id=1", [], |r| {
                r.get::<_, bool>(0)
            })? && version != 0;
        if recovered {
            let status: String = connection.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
            if status != "ok" {
                return Err(CatError::Invalid(
                    "Proyecto posiblemente corrupto: conserva el archivo y restaura un respaldo"
                        .into(),
                ));
            }
        }
        connection.execute_batch(
            "CREATE VIRTUAL TABLE temp.tm_vocab USING fts5vocab(main, 'tm_fts', 'instance');",
        )?;
        connection.execute("UPDATE session SET clean=0 WHERE id=1", [])?;
        tracing::info!(event = "project_opened", recovered);
        Ok(Self {
            connection,
            _lock: lock,
            recovered,
            closed: false,
        })
    }
    fn writable(&self) -> Result<()> {
        if self.closed {
            Err(CatError::Invalid("Proyecto cerrado".into()))
        } else {
            Ok(())
        }
    }
    pub fn documents(&self) -> Result<Vec<DocumentInfo>> {
        let mut stmt=self.connection.prepare("SELECT d.id,d.name,d.format,(SELECT count(*) FROM segments s WHERE s.document_id=d.id),d.source_lang,d.target_lang FROM documents d ORDER BY d.id")?;
        let records = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, String>(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(
                |(id, name, format, segment_count, source_lang, target_lang)| {
                    Ok(DocumentInfo {
                        id,
                        name,
                        format: DocumentFormat::parse(&format)?,
                        segment_count,
                        source_lang,
                        target_lang,
                    })
                },
            )
            .collect()
    }
    pub fn import_document(
        &mut self,
        document: &ImportedDocument,
        cancel: &Cancellation,
    ) -> Result<i64> {
        self.writable()?;
        cancel.check()?;
        let tx = self.connection.transaction()?;
        let original_path = document
            .original_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        tx.execute("INSERT INTO documents(name,format,original,original_path,source_lang,target_lang) VALUES(?1,?2,?3,?4,?5,?6)",params![document.name,document.format.as_str(),document.original,original_path,document.source_lang,document.target_lang])?;
        let id = tx.last_insert_rowid();
        {
            let mut stmt=tx.prepare("INSERT INTO segments(document_id,ordinal,external_id,source,target,state,locked,origin) VALUES(?1,?2,?3,?4,?5,?6,?7,'imported')")?;
            for (ordinal, segment) in document.segments.iter().enumerate() {
                cancel.check()?;
                if segment.source.len() > 1_048_576 || segment.target.len() > 1_048_576 {
                    return Err(CatError::Invalid(
                        "Segmento demasiado grande para este MVP".into(),
                    ));
                }
                stmt.execute(params![
                    id,
                    ordinal,
                    segment.external_id,
                    segment.source,
                    segment.target,
                    segment.state.as_str(),
                    segment.locked
                ])?;
            }
        }
        cancel.check()?;
        tx.commit()?;
        Ok(id)
    }
    pub fn page(
        &self,
        document_id: i64,
        start: usize,
        limit: usize,
        query: &str,
    ) -> Result<Vec<Segment>> {
        let mut stmt=self.connection.prepare("SELECT id,document_id,ordinal,external_id,source,target,state,locked,origin,revision FROM segments WHERE document_id=?1 AND ordinal>=?2 AND (?4='' OR instr(source,?4)>0 OR instr(target,?4)>0) ORDER BY ordinal LIMIT ?3")?;
        stmt.query_map(
            params![document_id, start, limit.min(256), query],
            read_segment,
        )?
        .map(|r| decode_segment(r?))
        .collect()
    }
    pub fn segment(&self, id: i64) -> Result<Segment> {
        get_segment(&self.connection, id)
    }
    /// Permite cancelar filtros substring sin dejar cambios ni hooks instalados.
    pub fn search_page(
        &self,
        document_id: i64,
        start: usize,
        limit: usize,
        query: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<Segment>> {
        self.cancellable(cancel, || self.page(document_id, start, limit, query))
    }
    fn cancellable<T>(
        &self,
        cancel: &Cancellation,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        cancel.check()?;
        let token = cancel.clone();
        self.connection
            .progress_handler(1000, Some(move || token.check().is_err()));
        let result = operation();
        self.connection.progress_handler(0, None::<fn() -> bool>);
        cancel.check()?;
        result
    }
    pub fn matches_cancel(
        &self,
        source: &str,
        sl: &str,
        tl: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<TmMatch>> {
        self.cancellable(cancel, || self.matches(source, sl, tl))
    }
    pub fn concordance_cancel(
        &self,
        query: &str,
        sl: &str,
        tl: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<TmMatch>> {
        self.cancellable(cancel, || self.concordance(query, sl, tl))
    }
    pub fn edit(&mut self, command: &EditCommand) -> Result<Segment> {
        self.writable()?;
        if command.target.len() > 1_048_576 {
            return Err(CatError::Invalid(
                "Traducción demasiado grande para este MVP".into(),
            ));
        }
        let tx = self.connection.transaction()?;
        let before = get_segment(&tx, command.segment_id)?;
        if before.revision != command.expected_revision {
            return Err(CatError::Invalid(
                "El segmento cambió; vuelve a cargarlo antes de editar".into(),
            ));
        }
        if before.locked && command.target != before.target {
            return Err(CatError::Invalid(
                "Desbloquea el segmento antes de editar su texto".into(),
            ));
        }
        if command.target == before.target
            && command.state == before.state
            && command.locked == before.locked
            && command.origin == before.origin
        {
            return Ok(before);
        }
        let cursor: i64 =
            tx.query_row("SELECT cursor FROM session WHERE id=1", [], |r| r.get(0))?;
        tx.execute("DELETE FROM history WHERE id>?1", [cursor])?;
        tx.execute(
            "INSERT INTO history VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                cursor + 1,
                before.id,
                before.target,
                before.state.as_str(),
                before.locked,
                before.origin.as_str(),
                command.target,
                command.state.as_str(),
                command.locked,
                command.origin.as_str()
            ],
        )?;
        tx.execute("UPDATE segments SET target=?1,state=?2,locked=?3,origin=?4,revision=revision+1 WHERE id=?5",params![command.target,command.state.as_str(),command.locked,command.origin.as_str(),before.id])?;
        tx.execute("UPDATE session SET cursor=?1 WHERE id=1", [cursor + 1])?;
        let after = get_segment(&tx, before.id)?;
        tx.commit()?;
        Ok(after)
    }
    pub fn undo(&mut self) -> Result<Option<Segment>> {
        self.history(false)
    }
    pub fn redo(&mut self) -> Result<Option<Segment>> {
        self.history(true)
    }
    fn history(&mut self, redo: bool) -> Result<Option<Segment>> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        let cursor: i64 =
            tx.query_row("SELECT cursor FROM session WHERE id=1", [], |r| r.get(0))?;
        let selected = if redo { cursor + 1 } else { cursor };
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM history WHERE id=?1)",
            [selected],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(None);
        }
        let prefix = if redo { "after" } else { "before" };
        let sql = format!(
            "SELECT segment_id,{prefix}_target,{prefix}_state,{prefix}_locked,{prefix}_origin FROM history WHERE id=?1"
        );
        let (id, target, state, locked, origin): (i64, String, String, bool, String) = tx
            .query_row(&sql, [selected], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?;
        tx.execute("UPDATE segments SET target=?1,state=?2,locked=?3,origin=?4,revision=revision+1 WHERE id=?5",params![target,state,locked,origin,id])?;
        tx.execute(
            "UPDATE session SET cursor=?1 WHERE id=1",
            [if redo { selected } else { cursor - 1 }],
        )?;
        let segment = get_segment(&tx, id)?;
        tx.commit()?;
        Ok(Some(segment))
    }
    pub fn load_document(&self, id: i64) -> Result<ImportedDocument> {
        let (name,format,original,original_path,source_lang,target_lang):(String,String,Vec<u8>,Option<String>,String,String)=self.connection.query_row("SELECT name,format,original,original_path,source_lang,target_lang FROM documents WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
        let mut segments = Vec::new();
        let mut start = 0;
        loop {
            let page = self.page(id, start, 256, "")?;
            if page.is_empty() {
                break;
            }
            for item in page {
                start = item.ordinal + 1;
                segments.push(ImportedSegment {
                    external_id: item.external_id,
                    source: item.source,
                    target: item.target,
                    state: item.state,
                    locked: item.locked,
                });
            }
        }
        Ok(ImportedDocument {
            name,
            format: DocumentFormat::parse(&format)?,
            original,
            original_path: original_path.map(Into::into),
            source_lang,
            target_lang,
            segments,
        })
    }
    pub fn import_tmx(
        &mut self,
        path: &Path,
        sl: &str,
        tl: &str,
        cancel: &Cancellation,
    ) -> Result<usize> {
        self.writable()?;
        let reader = BufReader::new(File::open(path)?);
        let tx = self.connection.transaction()?;
        let mut count = 0;
        formats::import_tmx(reader, sl, tl, cancel, |unit| {
            count += tm::insert(&tx, &unit)?;
            Ok(())
        })?;
        cancel.check()?;
        tx.commit()?;
        Ok(count)
    }
    pub fn insert_tm_units(
        &mut self,
        units: impl IntoIterator<Item = TmUnit>,
        cancel: &Cancellation,
    ) -> Result<usize> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        let mut count = 0;
        for unit in units {
            cancel.check()?;
            count += tm::insert(&tx, &unit)?;
        }
        cancel.check()?;
        tx.commit()?;
        Ok(count)
    }
    pub fn matches(&self, source: &str, sl: &str, tl: &str) -> Result<Vec<TmMatch>> {
        tm::matches(&self.connection, source, sl, tl)
    }
    pub fn concordance(&self, query: &str, sl: &str, tl: &str) -> Result<Vec<TmMatch>> {
        tm::concordance(&self.connection, query, sl, tl)
    }
    pub fn export_tm(&self, path: &Path, cancel: &Cancellation) -> Result<usize> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let mut stmt = self
            .connection
            .prepare("SELECT source,target,source_lang,target_lang,raw_xml FROM tm ORDER BY id")?;
        let units = stmt.query_map([], |r| {
            Ok(TmUnit {
                source: r.get(0)?,
                target: r.get(1)?,
                source_lang: r.get(2)?,
                target_lang: r.get(3)?,
                raw_xml: r.get(4)?,
            })
        })?;
        let count = {
            let mut writer = BufWriter::new(temporary.as_file_mut());
            let count = formats::export_tmx(
                &mut writer,
                units.map(|r| r.map_err(CatError::from)),
                cancel,
            )?;
            writer.flush()?;
            count
        };
        cancel.check()?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(path)
            .map_err(|error| CatError::Io(error.error))?;
        Ok(count)
    }
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            self.connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
            self.connection
                .execute("UPDATE session SET clean=1 WHERE id=1", [])?;
            self.closed = true;
            tracing::info!(event = "project_closed_cleanly");
        }
        Ok(())
    }
}

type SegmentRow = (
    i64,
    i64,
    usize,
    String,
    String,
    String,
    String,
    bool,
    String,
    i64,
);
fn read_segment(row: &rusqlite::Row<'_>) -> rusqlite::Result<SegmentRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}
fn decode_segment(
    (id,document_id,ordinal,external_id,source,target,state,locked,origin,revision):SegmentRow,
) -> Result<Segment> {
    Ok(Segment {
        id,
        document_id,
        ordinal,
        external_id,
        source,
        target,
        state: SegmentState::parse(&state)?,
        locked,
        origin: Origin::parse(&origin)?,
        revision,
    })
}
fn get_segment(connection: &Connection, id: i64) -> Result<Segment> {
    decode_segment(connection.query_row("SELECT id,document_id,ordinal,external_id,source,target,state,locked,origin,revision FROM segments WHERE id=?1",[id],read_segment)?)
}
