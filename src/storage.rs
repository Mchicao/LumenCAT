//! One owning worker holds the connection. Every edit and its undo record commit together.
use crate::{formats, model::*, tm};
use rusqlite::{Connection, params};
pub(crate) mod migrations;
mod terminology;
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
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
    pub migration_backup: Option<PathBuf>,
    closed: bool,
}

impl ProjectStore {
    pub fn open_existing(path: &Path) -> Result<Self> {
        Self::open(&path.canonicalize()?)
    }

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
        let mut connection = Connection::open_with_flags(
            &canonical,
            if canonical.exists() {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            } else {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
            },
        )?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let app: i64 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let objects: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if version > migrations::VERSION
            || (app != APPLICATION_ID && (app != 0 || objects != 0 || version != 0))
            || (app == APPLICATION_ID && !(1..=migrations::VERSION).contains(&version))
        {
            return Err(CatError::Invalid(
                "Base ajena o versión de proyecto no soportada; no fue modificada".into(),
            ));
        }
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let migration_backup = if version > 0 && version < migrations::VERSION {
            Some(migrations::migrate(&mut connection, &canonical, version)?)
        } else {
            None
        };
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        if version == 0 {
            let tx = connection.transaction()?;
            tx.execute_batch(SCHEMA)?;
            tx.execute_batch(migrations::V2)?;
            tx.execute_batch(migrations::V3)?;
            tx.execute_batch(migrations::V4)?;
            tx.execute_batch(migrations::V5)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            tx.pragma_update(None, "user_version", migrations::VERSION)?;
            tx.commit()?;
        }
        let recovered =
            connection.query_row("SELECT clean=0 FROM session WHERE id=1", [], |r| {
                r.get::<_, bool>(0)
            })? && version != 0;
        if recovered {
            migrations::integrity(&connection)?;
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
            migration_backup,
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
    pub fn settings(&self) -> Result<ProjectSettings> {
        Ok(self.connection.query_row(
            "SELECT source_lang,target_lang FROM settings WHERE id=1",
            [],
            |row| {
                Ok(ProjectSettings {
                    source_lang: row.get(0)?,
                    target_lang: row.get(1)?,
                })
            },
        )?)
    }
    pub fn set_settings(&mut self, settings: &ProjectSettings) -> Result<ProjectSettings> {
        self.writable()?;
        settings.validate()?;
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE settings SET source_lang=?1,target_lang=?2 WHERE id=1",
            params![settings.source_lang, settings.target_lang],
        )?;
        tx.execute("UPDATE settings SET write_memory_id=NULL WHERE write_memory_id IN (SELECT id FROM memories WHERE source_lang<>?1 OR target_lang<>?2)", params![settings.source_lang,settings.target_lang])?;
        tx.commit()?;
        self.settings()
    }

    pub fn memories(&self) -> Result<Vec<MemoryCollection>> {
        Ok(self
            .connection
            .prepare(
                "SELECT id,name,source_lang,target_lang,writable,enabled FROM memories ORDER BY id",
            )?
            .query_map([], |row| {
                Ok(MemoryCollection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_lang: row.get(2)?,
                    target_lang: row.get(3)?,
                    writable: row.get(4)?,
                    enabled: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn write_memory(&self) -> Result<Option<i64>> {
        Ok(self.connection.query_row(
            "SELECT write_memory_id FROM settings WHERE id=1",
            [],
            |row| row.get(0),
        )?)
    }
    pub fn create_memory(&mut self, name: &str, sl: &str, tl: &str) -> Result<i64> {
        self.writable()?;
        ProjectSettings {
            source_lang: sl.into(),
            target_lang: tl.into(),
        }
        .validate()?;
        if name.trim().is_empty() || name.len() > 256 {
            return Err(CatError::Invalid(
                "El nombre de memoria debe tener entre 1 y 256 bytes".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO memories(name,source_lang,target_lang) VALUES(?1,?2,?3)",
            params![name.trim(), sl, tl],
        )?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn select_write_memory(&mut self, memory: Option<i64>) -> Result<()> {
        self.writable()?;
        if let Some(id) = memory {
            let usable: bool = self.connection.query_row("SELECT m.writable AND m.enabled AND m.source_lang=s.source_lang AND m.target_lang=s.target_lang FROM memories m CROSS JOIN settings s WHERE m.id=?1", [id], |row| row.get(0))?;
            if !usable {
                return Err(CatError::Invalid(
                    "Elige una memoria habilitada de escritura para los idiomas del proyecto"
                        .into(),
                ));
            }
        }
        self.connection.execute(
            "UPDATE settings SET write_memory_id=?1 WHERE id=1",
            [memory],
        )?;
        Ok(())
    }
    pub fn configure_memory(&mut self, id: i64, writable: bool, enabled: bool) -> Result<()> {
        self.writable()?;
        if self.connection.execute(
            "UPDATE memories SET writable=?1,enabled=?2 WHERE id=?3",
            params![writable, enabled, id],
        )? == 0
        {
            return Err(CatError::Invalid("Memoria inexistente".into()));
        }
        Ok(())
    }
    pub fn import_document(
        &mut self,
        document: &ImportedDocument,
        cancel: &Cancellation,
    ) -> Result<i64> {
        self.writable()?;
        ProjectSettings {
            source_lang: document.source_lang.clone(),
            target_lang: document.target_lang.clone(),
        }
        .validate()?;
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
        let tx = self.connection.transaction()?;
        let after = apply_edit(&tx, command, &mut None, None)?;
        tx.commit()?;
        Ok(after)
    }
    pub fn confirm(&mut self, command: &EditCommand) -> Result<ConfirmationResult> {
        self.writable()?;
        if command.target.len() > 1_048_576 {
            return Err(CatError::Invalid("Traducción demasiado grande".into()));
        }
        if command.target.trim().is_empty() {
            return Err(CatError::Invalid(
                "No se puede confirmar una traducción vacía".into(),
            ));
        }
        let tx = self.connection.transaction()?;
        let before = get_segment(&tx, command.segment_id)?;
        if before.locked {
            return Err(CatError::Invalid(
                "Desbloquea el segmento antes de confirmar".into(),
            ));
        }
        let (format, sl, tl): (String, String, String) = tx.query_row(
            "SELECT format,source_lang,target_lang FROM documents WHERE id=?1",
            [before.document_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let has_codes = if format == "docx" {
            formats::docx::validate_target(&before.source, &command.target)?;
            crate::editing::parts(&before.source)
                .iter()
                .any(|(_, tag)| *tag)
        } else if format == "xliff12" {
            formats::inline::validate_target(&before.source, &command.target)?;
            crate::editing::parts(&before.source)
                .iter()
                .any(|(_, tag)| *tag)
        } else {
            false
        };
        let memory: Option<i64> = tx.query_row("SELECT m.id FROM settings s LEFT JOIN memories m ON m.id=s.write_memory_id AND m.writable=1 AND m.enabled=1 AND m.source_lang=?1 AND m.target_lang=?2 WHERE s.id=1", params![sl,tl], |row| row.get(0))?;
        let learning = if has_codes {
            LearningOutcome::UnsupportedCodes
        } else if memory.is_some() {
            LearningOutcome::Learned
        } else {
            LearningOutcome::Disabled
        };
        let mut confirmation = command.clone();
        confirmation.state = SegmentState::Confirmed;
        let segment = apply_edit(
            &tx,
            &confirmation,
            &mut None,
            if has_codes { None } else { memory },
        )?;
        tx.commit()?;
        Ok(ConfirmationResult { segment, learning })
    }
    pub fn edit_batch(
        &mut self,
        commands: &[EditCommand],
        cancel: &Cancellation,
    ) -> Result<Vec<Segment>> {
        self.writable()?;
        if commands.len() > 10_000 {
            return Err(CatError::Invalid(
                "La operación supera el límite de 10 000 segmentos".into(),
            ));
        }
        let tx = self.connection.transaction()?;
        let mut operation = None;
        let mut changed = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for command in commands {
            cancel.check()?;
            if !ids.insert(command.segment_id) {
                return Err(CatError::Invalid(
                    "Segmento duplicado en la operación".into(),
                ));
            }
            changed.push(apply_edit(&tx, command, &mut operation, None)?);
        }
        cancel.check()?;
        tx.commit()?;
        Ok(changed)
    }
    pub fn progress(&self, document: i64) -> Result<(usize, usize)> {
        Ok(self.connection.query_row(
            "SELECT count(*), coalesce(sum(trim(target)<>''),0) FROM segments WHERE document_id=?1",
            [document],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
    pub fn replace_targets(
        &mut self,
        document: i64,
        query: &str,
        replacement: &str,
    ) -> Result<usize> {
        self.writable()?;
        if query.is_empty() {
            return Err(CatError::Invalid("La búsqueda está vacía".into()));
        }
        let tx = self.connection.transaction()?;
        let segments = {
            let mut statement = tx.prepare("SELECT id,document_id,ordinal,external_id,source,target,state,locked,origin,revision FROM segments WHERE document_id=?1 AND locked=0 AND instr(target,?2)>0 ORDER BY ordinal")?;
            let rows = statement.query_map(params![document, query], read_segment)?;
            rows.map(|r| decode_segment(r?))
                .collect::<Result<Vec<_>>>()?
        };
        let mut count = 0;
        let mut operation = None;
        for segment in segments {
            let target = crate::editing::replace_text(&segment.target, query, replacement);
            if target != segment.target {
                apply_edit(
                    &tx,
                    &EditCommand {
                        segment_id: segment.id,
                        expected_revision: segment.revision,
                        target,
                        state: SegmentState::Draft,
                        locked: false,
                        origin: Origin::Human,
                    },
                    &mut operation,
                    None,
                )?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }
    pub fn undo(&mut self) -> Result<Option<Segment>> {
        Ok(self.undo_group()?.pop())
    }
    pub fn redo(&mut self) -> Result<Option<Segment>> {
        Ok(self.redo_group()?.pop())
    }
    pub fn undo_group(&mut self) -> Result<Vec<Segment>> {
        self.history(false)
    }
    pub fn redo_group(&mut self) -> Result<Vec<Segment>> {
        self.history(true)
    }
    fn history(&mut self, redo: bool) -> Result<Vec<Segment>> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        let cursor: i64 =
            tx.query_row("SELECT cursor FROM session WHERE id=1", [], |r| r.get(0))?;
        let selected = if redo { cursor + 1 } else { cursor };
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1)",
            [selected],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(Vec::new());
        }
        let prefix = if redo { "after" } else { "before" };
        let sql = format!(
            "SELECT segment_id,{prefix}_target,{prefix}_state,{prefix}_locked,{prefix}_origin,id FROM history WHERE operation_id=?1 ORDER BY id"
        );
        let entries = tx
            .prepare(&sql)?
            .query_map([selected], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, bool>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut segments = Vec::new();
        for (id, target, state, locked, origin, history_id) in entries {
            tx.execute("UPDATE segments SET target=?1,state=?2,locked=?3,origin=?4,revision=revision+1 WHERE id=?5",params![target,state,locked,origin,id])?;
            tx.execute(&format!("UPDATE tm SET active=(SELECT {prefix}_active FROM history_tm h WHERE h.tm_id=tm.id AND h.history_id=?1) WHERE id IN (SELECT tm_id FROM history_tm WHERE history_id=?1)"), [history_id])?;
            segments.push(get_segment(&tx, id)?);
        }
        tx.execute(
            "UPDATE session SET cursor=?1 WHERE id=1",
            [if redo { selected } else { cursor - 1 }],
        )?;
        tx.commit()?;
        Ok(segments)
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
        self.import_memory(path, sl, tl, cancel)
    }
    pub fn import_memory(
        &mut self,
        path: &Path,
        sl: &str,
        tl: &str,
        cancel: &Cancellation,
    ) -> Result<usize> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        let mut count = 0;
        formats::import_memory(path, sl, tl, cancel, |unit| {
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
        self.export_tm_filtered(path, None, cancel)
    }
    fn export_tm_filtered(
        &self,
        path: &Path,
        languages: Option<(&str, &str)>,
        cancel: &Cancellation,
    ) -> Result<usize> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let mut stmt = self
            .connection
            .prepare("SELECT source,target,source_lang,target_lang,raw_xml FROM tm WHERE active=1 AND (?1 IS NULL OR source_lang=?1 COLLATE NOCASE) AND (?2 IS NULL OR target_lang=?2 COLLATE NOCASE) ORDER BY id")?;
        let units = stmt.query_map(
            params![languages.map(|pair| pair.0), languages.map(|pair| pair.1)],
            |r| {
                Ok(TmUnit {
                    source: r.get(0)?,
                    target: r.get(1)?,
                    source_lang: r.get(2)?,
                    target_lang: r.get(3)?,
                    raw_xml: r.get(4)?,
                })
            },
        )?;
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
    pub fn update_sdltm(
        &self,
        source: &Path,
        destination: &Path,
        sl: &str,
        tl: &str,
        cancel: &Cancellation,
    ) -> Result<formats::sdltm::Report> {
        let directory = tempfile::tempdir()?;
        let interchange = directory.path().join("updates.tmx");
        if self.export_tm_filtered(&interchange, Some((sl, tl)), cancel)? == 0 {
            return Err(CatError::Invalid(
                "No hay unidades activas del par seleccionado para actualizar SDLTM".into(),
            ));
        }
        formats::sdltm::update(source, destination, &interchange, sl, tl, cancel)
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
    pub fn backup_to(&self, path: &Path, cancel: &Cancellation) -> Result<()> {
        self.writable()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        migrations::backup(&self.connection, parent, ".lumencat-backup-", cancel)?
            .persist_noclobber(path)
            .map_err(|error| CatError::Io(error.error))?;
        Ok(())
    }
    pub fn recover_copy(backup: &Path, destination: &Path, cancel: &Cancellation) -> Result<()> {
        let source = migrations::read_only(backup)?;
        let app: i64 = source.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let version: i64 = source.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if app != APPLICATION_ID || !(1..=migrations::VERSION).contains(&version) {
            return Err(CatError::Invalid(
                "No es un respaldo de LumenCAT compatible".into(),
            ));
        }
        migrations::integrity(&source)?;
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        migrations::backup(&source, parent, ".lumencat-recovered-", cancel)?
            .persist_noclobber(destination)
            .map_err(|error| CatError::Io(error.error))?;
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

fn apply_edit(
    tx: &rusqlite::Transaction<'_>,
    command: &EditCommand,
    operation: &mut Option<i64>,
    learning_memory: Option<i64>,
) -> Result<Segment> {
    if command.target.len() > 1_048_576 {
        return Err(CatError::Invalid("Traducción demasiado grande".into()));
    }
    let before = get_segment(tx, command.segment_id)?;
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
    let active_before = active_contributions(tx, before.id)?;
    if command.target != before.target {
        tx.execute(
            "UPDATE tm SET active=0 WHERE learned_segment_id=?1 AND active=1",
            [before.id],
        )?;
    }
    if let Some(memory) = learning_memory {
        let (sl, tl): (String, String) = tx.query_row(
            "SELECT source_lang,target_lang FROM documents WHERE id=?1",
            [before.document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        tm::insert_in_memory(
            tx,
            &TmUnit {
                source: before.source.clone(),
                target: command.target.clone(),
                source_lang: sl,
                target_lang: tl,
                raw_xml: String::new(),
            },
            memory,
            Some(before.id),
        )?;
        tx.execute(
            "UPDATE tm SET active=(target=?1) WHERE memory_id=?2 AND learned_segment_id=?3",
            params![command.target, memory, before.id],
        )?;
    }
    let active_after = active_contributions(tx, before.id)?;
    if active_before == active_after
        && command.target == before.target
        && command.state == before.state
        && command.locked == before.locked
        && command.origin == before.origin
    {
        return Ok(before);
    }
    let operation_id = match *operation {
        Some(id) => id,
        None => {
            let cursor: i64 =
                tx.query_row("SELECT cursor FROM session WHERE id=1", [], |r| r.get(0))?;
            tx.execute("DELETE FROM history WHERE operation_id>?1", [cursor])?;
            tx.execute("DELETE FROM operations WHERE id>?1", [cursor])?;
            tx.execute("INSERT INTO operations(id) VALUES(?1)", [cursor + 1])?;
            tx.execute("UPDATE session SET cursor=?1 WHERE id=1", [cursor + 1])?;
            *operation = Some(cursor + 1);
            cursor + 1
        }
    };
    tx.execute(
        "INSERT INTO history(operation_id,segment_id,before_target,before_state,before_locked,before_origin,after_target,after_state,after_locked,after_origin) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            operation_id,
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
    let history_id = tx.last_insert_rowid();
    for id in active_before.union(&active_after) {
        let before_active = active_before.contains(id);
        let after_active = active_after.contains(id);
        if before_active != after_active {
            tx.execute(
                "INSERT INTO history_tm VALUES(?1,?2,?3,?4)",
                params![history_id, id, before_active, after_active],
            )?;
        }
    }
    tx.execute("UPDATE segments SET target=?1,state=?2,locked=?3,origin=?4,revision=revision+1 WHERE id=?5",params![command.target,command.state.as_str(),command.locked,command.origin.as_str(),before.id])?;
    let after = get_segment(tx, before.id)?;
    Ok(after)
}

fn active_contributions(
    connection: &Connection,
    segment: i64,
) -> Result<std::collections::BTreeSet<i64>> {
    Ok(connection
        .prepare("SELECT id FROM tm WHERE learned_segment_id=?1 AND active=1 ORDER BY id")?
        .query_map([segment], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    fn v1(path: &Path) -> Result<Connection> {
        let connection = Connection::open(path)?;
        connection.execute_batch(SCHEMA)?;
        connection.pragma_update(None, "application_id", APPLICATION_ID)?;
        connection.pragma_update(None, "user_version", 1)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; INSERT INTO documents VALUES(1,'viejo','txt',X'48656C6C6F',NULL,'en','es'); INSERT INTO segments VALUES(1,1,0,'0','Hello','Café 世界 🙂','draft',0,'human',2); INSERT INTO history VALUES(1,1,'','draft',0,'imported','Café 世界 🙂','draft',0,'human'); INSERT INTO history VALUES(2,1,'Café 世界 🙂','draft',0,'human','otra variante','confirmed',0,'human'); UPDATE session SET cursor=1;")?;
        connection.execute("INSERT INTO tm(source,target,source_lang,target_lang,normalized,chars,raw_xml) VALUES('Hello','Importada','en','es','Hello',5,'')", [])?;
        Ok(connection)
    }

    #[test]
    fn v1_migration_preserves_text_memory_cursor_and_previous_backup() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("v1.lcat");
        let original = v1(&path)?;
        let mut store = ProjectStore::open_existing(&path)?;
        let backup = store
            .migration_backup
            .as_ref()
            .ok_or_else(|| CatError::Invalid("sin respaldo".into()))?;
        let copy = migrations::read_only(backup)?;
        assert_eq!(
            copy.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            1
        );
        assert_eq!(
            copy.query_row("SELECT target FROM segments", [], |r| r.get::<_, String>(0))?,
            "Café 世界 🙂"
        );
        assert_eq!(store.segment(1)?.target, "Café 世界 🙂");
        assert_eq!(store.matches("Hello", "en", "es")?[0].target, "Importada");
        assert_eq!(
            store.redo()?.map(|s| s.target),
            Some("otra variante".into())
        );
        assert_eq!(store.undo()?.map(|s| s.target), Some("Café 世界 🙂".into()));
        assert_eq!(store.undo()?.map(|s| s.target), Some(String::new()));
        drop(original);
        store.close()?;
        Ok(())
    }

    #[test]
    fn failed_migration_rolls_back_and_keeps_backup() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("broken-v1.lcat");
        let connection = v1(&path)?;
        connection.execute_batch("ALTER TABLE history ADD COLUMN operation_id INTEGER;")?;
        assert!(ProjectStore::open_existing(&path).is_err());
        assert_eq!(
            connection.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            1
        );
        assert!(!connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='operations')",
            [],
            |r| r.get::<_, bool>(0)
        )?);
        assert_eq!(
            connection.query_row("SELECT target FROM segments", [], |r| r.get::<_, String>(0))?,
            "Café 世界 🙂"
        );
        assert!(
            std::fs::read_dir(directory.path())?
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().contains("backup-v1-"))
        );
        Ok(())
    }

    #[test]
    fn v2_v3_and_v4_migrate_language_variants_without_losing_units() -> Result<()> {
        let directory = tempfile::tempdir()?;
        for version in [2, 3, 4] {
            let path = directory.path().join(format!("v{version}.lcat"));
            let connection = v1(&path)?;
            connection.execute_batch(migrations::V2)?;
            if version >= 3 {
                connection.execute_batch(migrations::V3)?;
            }
            connection.pragma_update(None, "user_version", version)?;
            connection.execute("INSERT INTO tm(source,target,source_lang,target_lang,normalized,chars,raw_xml) VALUES('Hello','Otra importada','EN','ES','Hello',5,'')", [])?;
            if version == 4 {
                connection.execute_batch(migrations::V4)?;
            }
            let mut store = ProjectStore::open_existing(&path)?;
            let backup = store
                .migration_backup
                .as_ref()
                .ok_or_else(|| CatError::Invalid("sin respaldo".into()))?;
            assert_eq!(
                migrations::read_only(backup)?
                    .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
                version
            );
            assert_eq!(store.matches("Hello", "en", "es")?.len(), 2);
            assert_eq!(store.settings()?.source_lang, "en");
            assert_eq!(store.segment(1)?.target, "Café 世界 🙂");
            assert!(store.term_bases()?.is_empty());
            store.redo()?;
            assert_eq!(store.segment(1)?.target, "otra variante");
            store.close()?;
        }
        Ok(())
    }
}
