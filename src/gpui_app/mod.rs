pub mod components;
pub use crate::editing;
mod input;
pub mod runtime;
pub mod theme;

use crate::{
    model::{
        Cancellation, ConfirmationIntent, DocumentInfo, EditCommand, LearningOutcome,
        MemoryCollection, Origin, ProjectSettings, QaIssue, Segment, SegmentState, TmMatch,
    },
    worker::{Data as WorkerData, Reply, Request, Task as WorkerTask},
};
use components::*;
use gpui::prelude::*;
use gpui::*;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc::SyncSender,
    time::{Duration, Instant},
};
use theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    TranslationMemory,
    QualityAssurance,
}

pub struct DraftState {
    pub segment: Segment,
    pub serial: u64,
    pub saved: u64,
    pub changed: Instant,
    pub first_dirty: Instant,
    pub saving: bool,
}

impl DraftState {
    pub fn new(segment: Segment) -> Self {
        Self {
            segment,
            serial: 0,
            saved: 0,
            changed: Instant::now(),
            first_dirty: Instant::now(),
            saving: false,
        }
    }

    pub fn acknowledge(&mut self, segment: &Segment, serial: u64) {
        if segment.id == self.segment.id {
            self.segment.revision = segment.revision;
            if self.serial == serial {
                self.segment = segment.clone();
            }
            self.saved = serial;
            self.saving = false;
        }
    }
}

#[allow(dead_code)]
enum PendingOp {
    Open,
    Operation,
    Page(u64, usize),
    Search(u64),
    Select(u64),
    Save(u64),
    Confirm(u64),
    Matches(u64),
    History,
    Close,
    Qa(u64, u64),
    Reload(u64),
}

enum PendingNav {
    Segment(i64),
    Ordinal(i64, usize),
}

pub struct LumenCatApp {
    sender: SyncSender<Request>,
    next_id: u64,
    pending: HashMap<u64, PendingOp>,
    generation: u64,
    selection_generation: u64,

    // Project & Documents
    pub project_path: String,
    pub opened: bool,
    pub documents: Vec<DocumentInfo>,
    pub current_document_id: Option<i64>,
    pub source_lang: String,
    pub target_lang: String,
    pub source_language_input: InputModel,
    pub target_language_input: InputModel,
    pub memory_name_input: InputModel,
    pub memories: Vec<MemoryCollection>,
    pub write_memory_id: Option<i64>,

    // Grid data
    pub rows: BTreeMap<usize, Segment>,
    requested_pages: Vec<usize>,

    // Editor data
    pub active_draft: Option<DraftState>,
    pub target_input: InputModel,
    navigate_after_save: Option<PendingNav>,
    confirmation_intent: Option<ConfirmationIntent>,

    // Intelligence
    pub matches: Vec<TmMatch>,
    pub qa_issues: Vec<QaIssue>,
    pub active_tab: RightTab,
    latest_matches_req: u64,

    // Search
    pub search_input: InputModel,
    pub replacement_input: InputModel,
    show_replace: bool,
    document_scope: bool,
    grid_scroll: UniformListScrollHandle,
    grid_anchor: usize,
    progress: (usize, usize),
    applied_tm: HashMap<i64, f64>,
    pub search_results: Vec<Segment>,
    pub is_searching: bool,

    // Status
    pub message: String,
    pub save_error: bool,
    pub closing_requested: bool,
    cancellations: HashMap<u64, Cancellation>,
    pub focus_handle: FocusHandle,
    show_memory_actions: bool,
}

impl Focusable for LumenCatApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn trim_grid_cache(rows: &mut BTreeMap<usize, Segment>, anchor: usize) {
    while rows.len() > 1024 {
        let Some((&first, _)) = rows.first_key_value() else {
            break;
        };
        let Some((&last, _)) = rows.last_key_value() else {
            break;
        };
        if anchor.abs_diff(first) > anchor.abs_diff(last) {
            rows.pop_first();
        } else {
            rows.pop_last();
        }
    }
}

impl LumenCatApp {
    pub fn new(sender: SyncSender<Request>, project: Option<String>, cx: &mut App) -> Self {
        let mut app = Self {
            sender,
            next_id: 0,
            pending: HashMap::new(),
            generation: 0,
            selection_generation: 0,
            project_path: project.unwrap_or_default(),
            opened: false,
            documents: Vec::new(),
            current_document_id: None,
            source_lang: "en".into(),
            target_lang: "es".into(),
            source_language_input: InputModel::new("Origen (fr, en...)", cx),
            target_language_input: InputModel::new("Destino (es, pt-BR...)", cx),
            memory_name_input: InputModel::new("Nombre de memoria", cx),
            memories: Vec::new(),
            write_memory_id: None,
            rows: BTreeMap::new(),
            requested_pages: Vec::new(),
            active_draft: None,
            target_input: InputModel::new("Type translation here...", cx),
            navigate_after_save: None,
            confirmation_intent: None,
            matches: Vec::new(),
            qa_issues: Vec::new(),
            active_tab: RightTab::TranslationMemory,
            latest_matches_req: 0,
            search_input: InputModel::new("Search source / target...", cx),
            replacement_input: InputModel::new("Reemplazar por...", cx),
            show_replace: false,
            document_scope: true,
            grid_scroll: UniformListScrollHandle::new(),
            grid_anchor: 0,
            progress: (0, 0),
            applied_tm: HashMap::new(),
            search_results: Vec::new(),
            is_searching: false,
            message: "Open or create a local project to get started · 100% Offline & Private"
                .into(),
            save_error: false,
            closing_requested: false,
            cancellations: HashMap::new(),
            focus_handle: cx.focus_handle(),
            show_memory_actions: false,
        };

        if !app.project_path.trim().is_empty() {
            let path = PathBuf::from(&app.project_path);
            app.send(WorkerTask::Open(path), PendingOp::Open);
        }

        app
    }

    fn send(&mut self, task: WorkerTask, pending: PendingOp) -> bool {
        self.next_id += 1;
        match self.sender.try_send(Request {
            id: self.next_id,
            task,
        }) {
            Ok(()) => {
                self.pending.insert(self.next_id, pending);
                true
            }
            Err(_) => {
                self.message = "Background worker busy. Text preserved in editor.".into();
                false
            }
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.confirmation_intent.is_some()
            || self
                .active_draft
                .as_ref()
                .is_some_and(|a| a.serial != a.saved || a.saving)
    }

    pub fn is_busy(&self) -> bool {
        self.pending.values().any(|p| {
            matches!(
                p,
                PendingOp::Operation
                    | PendingOp::Open
                    | PendingOp::History
                    | PendingOp::Close
                    | PendingOp::Select(_)
                    | PendingOp::Reload(_)
                    | PendingOp::Search(_)
            )
        })
    }

    pub fn invalidate(&mut self) {
        self.cancel_lookups();
        self.generation += 1;
        self.rows.clear();
        self.requested_pages.clear();
        self.grid_anchor = 0;
        self.search_results.clear();
        self.is_searching = false;
        self.selection_generation += 1;
    }

    fn cancel_lookups(&self) {
        for (id, pending) in &self.pending {
            if matches!(pending, PendingOp::Matches(_))
                && let Some(cancel) = self.cancellations.get(id)
            {
                cancel.cancel();
            }
        }
    }

    pub fn select_segment(&mut self, id: i64) {
        self.navigate_to(PendingNav::Segment(id));
    }

    fn navigate_to(&mut self, nav: PendingNav) {
        self.cancel_lookups();
        if self.is_dirty() {
            self.navigate_after_save = Some(nav);
            self.save(true);
        } else {
            self.selection_generation += 1;
            self.matches.clear();
            self.qa_issues.clear();
            let task = match nav {
                PendingNav::Segment(id) => WorkerTask::Select(id),
                PendingNav::Ordinal(doc, ord) => WorkerTask::AtOrdinal(doc, ord),
            };
            self.send(task, PendingOp::Select(self.selection_generation));
        }
    }

    pub fn on_target_text_changed(&mut self, origin: Origin) {
        if self.active_draft.as_ref().is_some_and(|a| a.segment.locked) {
            return;
        }
        let text = self.target_input.text.clone();
        self.confirmation_intent = None;
        if let Some(active) = &mut self.active_draft {
            active.segment.target = text;
            if active.serial == active.saved {
                active.first_dirty = Instant::now();
            }
            active.serial += 1;
            active.changed = Instant::now();
            active.segment.origin = origin;
            active.segment.state = SegmentState::Draft;
            self.request_qa();
        }
    }

    pub fn close_when_saved(&mut self) {
        if self.closing_requested && self.opened && !self.is_dirty() && !self.is_busy() {
            self.send(WorkerTask::Close, PendingOp::Close);
        }
    }

    pub fn save(&mut self, force: bool) {
        if self.save_error {
            return;
        }
        let Some(active) = &self.active_draft else {
            return;
        };
        if self.confirmation_intent.is_some_and(|intent| {
            intent.segment_id != active.segment.id || intent.serial != active.serial
        }) {
            self.confirmation_intent = None;
        }
        if active.segment.target.len() > 1_048_576 {
            self.save_error = true;
            self.message = "Target exceeds 1 MiB limit. Content is safely kept in editor.".into();
            return;
        }
        let confirming = self.confirmation_intent.is_some_and(|intent| {
            intent.segment_id == active.segment.id && intent.serial == active.serial
        });
        if active.saving || (active.serial == active.saved && !confirming) {
            return;
        }
        if !force
            && !confirming
            && active.changed.elapsed() < Duration::from_millis(250)
            && active.first_dirty.elapsed() < Duration::from_millis(500)
        {
            return;
        }
        let serial = active.serial;
        let cmd = EditCommand {
            segment_id: active.segment.id,
            expected_revision: active.segment.revision,
            target: active.segment.target.clone(),
            state: active.segment.state,
            locked: active.segment.locked,
            origin: active.segment.origin,
        };
        let (task, pending) = if confirming {
            (WorkerTask::Confirm(cmd), PendingOp::Confirm(serial))
        } else {
            (WorkerTask::Edit(cmd), PendingOp::Save(serial))
        };
        if self.send(task, pending)
            && let Some(active) = &mut self.active_draft
        {
            active.saving = true;
        }
    }

    pub fn confirm_active(&mut self) {
        self.queue_confirmation(false);
    }

    pub fn confirm_and_next(&mut self) {
        if self.active_draft.as_ref().is_none_or(|a| a.segment.locked) {
            return;
        }
        self.queue_confirmation(true);
    }

    fn queue_confirmation(&mut self, advance: bool) {
        let Some(active) = &self.active_draft else {
            return;
        };
        if active.segment.locked || active.segment.target.trim().is_empty() {
            self.message =
                "Introduce una traducción y desbloquea el segmento antes de confirmar".into();
            return;
        }
        self.confirmation_intent = Some(ConfirmationIntent {
            segment_id: active.segment.id,
            serial: active.serial,
            advance,
        });
        self.save(true);
    }

    pub fn move_segment(&mut self, delta: isize) {
        let Some(active) = &self.active_draft else {
            return;
        };
        let doc_id = active.segment.document_id;
        let count = self
            .documents
            .iter()
            .find(|d| d.id == doc_id)
            .map_or(0, |d| d.segment_count);
        let target_ord = active.segment.ordinal.saturating_add_signed(delta);
        if target_ord >= count {
            return;
        }
        if let Some(row) = self.rows.get(&target_ord) {
            self.select_segment(row.id);
        } else {
            self.navigate_to(PendingNav::Ordinal(doc_id, target_ord));
        }
    }

    pub fn copy_source_to_target(&mut self) {
        if let Some(active) = &self.active_draft {
            if active.segment.locked {
                return;
            }
            let src = active.segment.source.clone();
            self.target_input.set_text(src);
            self.on_target_text_changed(Origin::Human);
        }
    }

    pub fn toggle_active_lock(&mut self) {
        if let Some(active) = &mut self.active_draft {
            active.segment.locked = !active.segment.locked;
            active.serial += 1;
            active.changed = Instant::now();
            self.save(true);
        }
    }

    pub fn insert_tm_match(&mut self, index: usize) {
        if self.active_draft.as_ref().is_some_and(|a| a.segment.locked) {
            return;
        }
        if let Some(tm) = self.matches.get(index) {
            if let Some(active) = &self.active_draft {
                self.applied_tm.insert(active.segment.id, tm.score);
            }
            let target = tm.target.clone();
            self.target_input.set_text(target);
            self.on_target_text_changed(Origin::TranslationMemory);
        }
    }

    pub fn lookup(&mut self, query: String, sl: String, tl: String, concordance: bool) {
        self.cancel_lookups();
        self.matches.clear();
        let cancel = Cancellation::default();
        let task = if concordance {
            WorkerTask::Concordance(query, sl, tl, cancel.clone())
        } else {
            WorkerTask::Matches(query, sl, tl, cancel.clone())
        };
        if self.send(task, PendingOp::Matches(self.selection_generation)) {
            self.cancellations.insert(self.next_id, cancel);
            self.latest_matches_req = self.next_id;
        }
    }

    pub fn request_qa(&mut self) {
        if let Some(a) = &self.active_draft {
            let task = WorkerTask::Qa(
                a.segment.source.clone(),
                a.segment.target.clone(),
                a.segment.state == SegmentState::Confirmed,
            );
            let serial = a.serial;
            self.send(task, PendingOp::Qa(self.selection_generation, serial));
        }
    }

    fn refresh_progress(&mut self) {
        if let Some(id) = self.current_document_id {
            self.send(WorkerTask::Progress(id), PendingOp::Operation);
        }
    }
    pub fn insert_next_tag(&mut self) {
        if let Some(active) = &self.active_draft {
            if active.segment.locked {
                return;
            }
            if let Some(tag) = editing::next_tag(&active.segment.source, &self.target_input.text) {
                self.target_input.insert(tag);
                self.on_target_text_changed(Origin::Human);
            } else {
                self.message = "Todas las etiquetas ya están insertadas".into();
            }
        }
    }
    pub fn replace_targets(&mut self) {
        if self.search_input.text.is_empty() || self.is_dirty() || self.is_busy() {
            return;
        }
        if self.document_scope {
            if let Some(id) = self.current_document_id {
                self.send(
                    WorkerTask::ReplaceTargets(
                        id,
                        self.search_input.text.clone(),
                        self.replacement_input.text.clone(),
                    ),
                    PendingOp::Operation,
                );
            }
        } else if self
            .active_draft
            .as_ref()
            .is_some_and(|a| !a.segment.locked)
        {
            let text = editing::replace_text(
                &self.target_input.text,
                &self.search_input.text,
                &self.replacement_input.text,
            );
            self.target_input.set_text(text);
            self.on_target_text_changed(Origin::Human);
            self.save(true);
        }
    }

    pub fn perform_search(&mut self) {
        if !self.document_scope && !self.search_input.text.is_empty() {
            self.is_searching = true;
            self.search_results = self
                .active_draft
                .as_ref()
                .filter(|a| {
                    a.segment.source.contains(&self.search_input.text)
                        || a.segment.target.contains(&self.search_input.text)
                })
                .map(|a| vec![a.segment.clone()])
                .unwrap_or_default();
            return;
        }
        let query = self.search_input.text.trim().to_string();
        if query.is_empty() {
            self.is_searching = false;
            self.search_results.clear();
            return;
        }
        let Some(doc) = self.current_document_id else {
            return;
        };
        self.cancel_lookups();
        self.is_searching = true;
        let cancel = Cancellation::default();
        if self.send(
            WorkerTask::Search(doc, 0, query, cancel.clone()),
            PendingOp::Search(self.generation),
        ) {
            self.cancellations.insert(self.next_id, cancel);
            self.message = "Searching segments...".into();
        }
    }

    pub fn perform_concordance(&mut self) {
        let query = self.search_input.text.trim().to_string();
        if query.is_empty() {
            return;
        }
        let (sl, tl) = self.document_languages();
        self.active_tab = RightTab::TranslationMemory;
        self.lookup(query, sl, tl, true);
    }

    pub fn history(&mut self, redo: bool) {
        if self.is_dirty() {
            self.save(true);
            self.message = "Saving changes before undo/redo...".into();
        } else if !self.is_busy() {
            self.send(
                if redo {
                    WorkerTask::Redo
                } else {
                    WorkerTask::Undo
                },
                PendingOp::History,
            );
        }
    }

    pub fn document_languages(&self) -> (String, String) {
        self.documents
            .iter()
            .find(|d| Some(d.id) == self.current_document_id)
            .map(|d| (d.source_lang.clone(), d.target_lang.clone()))
            .unwrap_or_else(|| (self.source_lang.clone(), self.target_lang.clone()))
    }

    pub fn open_project_dialog(&mut self) {
        if self.is_dirty() || self.is_busy() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LumenCAT Project", &["lcat", "db", "sqlite"])
            .save_file()
        {
            self.project_path = path.to_string_lossy().to_string();
            self.send(WorkerTask::Open(path), PendingOp::Open);
        }
    }

    pub fn import_document_dialog(&mut self) {
        if !self.opened || self.is_busy() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Supported Documents", &["docx", "xlf", "xliff", "txt"])
            .add_filter("Word Documents", &["docx"])
            .add_filter("XLIFF", &["xlf", "xliff"])
            .add_filter("Plain Text", &["txt"])
            .pick_file()
        {
            let (sl, tl) = (self.source_lang.clone(), self.target_lang.clone());
            let cancel = Cancellation::default();
            if self.send(
                WorkerTask::Import(path, sl, tl, cancel.clone()),
                PendingOp::Operation,
            ) {
                self.cancellations.insert(self.next_id, cancel);
                self.message = "Importing document...".into();
            }
        }
    }

    pub fn save_project_languages(&mut self) {
        if !self.opened || self.is_busy() || self.is_dirty() {
            return;
        }
        self.send(
            WorkerTask::SetSettings(ProjectSettings {
                source_lang: self.source_language_input.text.clone(),
                target_lang: self.target_language_input.text.clone(),
            }),
            PendingOp::Operation,
        );
    }

    fn refresh_memories(&mut self) {
        self.send(WorkerTask::MemoryResources, PendingOp::Operation);
    }
    fn create_project_memory(&mut self) {
        if self.is_dirty() || self.is_busy() {
            return;
        }
        self.send(
            WorkerTask::CreateMemory(
                self.memory_name_input.text.clone(),
                self.source_lang.clone(),
                self.target_lang.clone(),
            ),
            PendingOp::Operation,
        );
    }

    pub fn import_tmx_dialog(&mut self) {
        if !self.opened || self.is_busy() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("TMX Translation Memory", &["tmx"])
            .pick_file()
        {
            let (sl, tl) = self.document_languages();
            let cancel = Cancellation::default();
            if self.send(
                WorkerTask::ImportTm(path, sl, tl, cancel.clone()),
                PendingOp::Operation,
            ) {
                self.cancellations.insert(self.next_id, cancel);
                self.message = "Importing TMX memory...".into();
            }
        }
    }

    pub fn export_document_dialog(&mut self) {
        if self.is_dirty() || self.is_busy() {
            self.save(true);
            return;
        }
        let Some(doc_id) = self.current_document_id else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Translated Document", &["docx", "xlf", "xliff", "txt"])
            .save_file()
        {
            let cancel = Cancellation::default();
            if self.send(
                WorkerTask::Export(doc_id, path, cancel.clone()),
                PendingOp::Operation,
            ) {
                self.cancellations.insert(self.next_id, cancel);
                self.message = "Exporting document safely...".into();
            }
        }
    }

    pub fn export_tmx_dialog(&mut self) {
        if !self.opened || self.is_busy() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("TMX Translation Memory", &["tmx"])
            .save_file()
        {
            let cancel = Cancellation::default();
            if self.send(
                WorkerTask::ExportTm(path, cancel.clone()),
                PendingOp::Operation,
            ) {
                self.cancellations.insert(self.next_id, cancel);
                self.message = "Exporting project translation memory...".into();
            }
        }
    }

    pub fn on_reply(&mut self, reply: Reply) {
        let Some(pending) = self.pending.remove(&reply.id) else {
            return;
        };
        self.cancellations.remove(&reply.id);
        let refresh_progress = matches!(
            &reply.result,
            Ok(WorkerData::Opened(..)
                | WorkerData::Documents(..)
                | WorkerData::Saved(..)
                | WorkerData::Confirmed(..)
                | WorkerData::History(..)
                | WorkerData::Done(..))
        );

        if matches!(pending, PendingOp::Matches(g) if g != self.selection_generation || reply.id != self.latest_matches_req)
            || matches!(pending, PendingOp::Search(g) if g != self.generation)
        {
            return;
        }

        if let PendingOp::Page(g, start) = pending
            && g == self.generation
        {
            self.requested_pages.retain(|p| *p != start);
        }

        match reply.result {
            Err(error) => {
                if matches!(pending, PendingOp::Save(_) | PendingOp::Confirm(_)) {
                    self.save_error = !(matches!(pending, PendingOp::Confirm(_))
                        && matches!(error, crate::model::CatError::Format(_)));
                    if let Some(a) = &mut self.active_draft {
                        a.saving = false;
                    }
                    self.confirmation_intent = None;
                }
                self.message = format!("{error}. Unsaved text remains in the editor.");
            }
            Ok(data) => {
                match data {
                    WorkerData::Opened(recovered, docs, settings, backup) => {
                        self.opened = true;
                        self.save_error = false;
                        self.active_draft = None;
                        self.confirmation_intent = None;
                        self.target_input.set_text("");
                        self.current_document_id = None;
                        self.progress = (0, 0);
                        self.applied_tm.clear();
                        self.invalidate();
                        self.documents = docs;
                        self.source_lang = settings.source_lang;
                        self.target_lang = settings.target_lang;
                        self.source_language_input
                            .set_text(self.source_lang.clone());
                        self.target_language_input
                            .set_text(self.target_lang.clone());
                        if let Some(first_doc) = self.documents.first().cloned() {
                            self.current_document_id = Some(first_doc.id);
                            self.invalidate();
                            self.navigate_to(PendingNav::Ordinal(first_doc.id, 0));
                        }
                        self.message = if recovered {
                            "Clean recovery from previous session. SQLite verified database safely."
                        } else {
                            "Local project opened. Changes are saved transactionally."
                        }
                        .into();
                        if let Some(backup) = backup {
                            self.message =
                                format!("Proyecto migrado con respaldo en {}", backup.display());
                        }
                        self.refresh_memories();
                    }
                    WorkerData::Settings(settings) => {
                        self.source_lang = settings.source_lang;
                        self.target_lang = settings.target_lang;
                        self.source_language_input
                            .set_text(self.source_lang.clone());
                        self.target_language_input
                            .set_text(self.target_lang.clone());
                        self.message = "Idiomas guardados para próximas importaciones; los documentos existentes no cambian".into();
                        self.refresh_memories();
                    }
                    WorkerData::Documents(docs) => {
                        self.documents = docs;
                        if self.current_document_id.is_none()
                            && let Some(first_doc) = self.documents.first().cloned()
                        {
                            self.current_document_id = Some(first_doc.id);
                            self.invalidate();
                            self.navigate_to(PendingNav::Ordinal(first_doc.id, 0));
                        }
                        self.message = "Documents updated successfully".into();
                    }
                    WorkerData::Page(rows) => {
                        if matches!(pending, PendingOp::Search(g) if g == self.generation) {
                            self.message = format!("{} segmentos encontrados", rows.len());
                            self.search_results = rows;
                        } else if matches!(pending, PendingOp::Page(g, _) if g == self.generation) {
                            for row in rows {
                                self.rows.insert(row.ordinal, row);
                            }
                            trim_grid_cache(&mut self.rows, self.grid_anchor);
                        }
                    }
                    WorkerData::Selected(segment) => {
                        if matches!(pending, PendingOp::Select(g) | PendingOp::Reload(g) if g == self.selection_generation)
                        {
                            let source = segment.source.clone();
                            self.target_input.set_text(segment.target.clone());
                            let index = if self.is_searching {
                                self.search_results
                                    .iter()
                                    .position(|s| s.id == segment.id)
                                    .unwrap_or(0)
                            } else {
                                segment.ordinal
                            };
                            self.grid_scroll
                                .scroll_to_item(index, ScrollStrategy::Center);
                            self.active_draft = Some(DraftState::new(segment));
                            self.matches.clear();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                            self.request_qa();
                        }
                    }
                    WorkerData::Saved(segment) => {
                        if let (PendingOp::Save(serial), Some(active)) =
                            (pending, &mut self.active_draft)
                            && segment.id == active.segment.id
                        {
                            active.acknowledge(&segment, serial);
                            self.rows.insert(segment.ordinal, segment);
                            trim_grid_cache(&mut self.rows, self.grid_anchor);
                            self.message = "All changes saved safely to SQLite disk".into();
                        }
                        self.request_qa();
                        if let Some(active) = &self.active_draft {
                            let source = active.segment.source.clone();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                        }
                    }
                    WorkerData::Confirmed(result) => {
                        let segment = result.segment;
                        if let (PendingOp::Confirm(serial), Some(active)) =
                            (pending, &mut self.active_draft)
                            && segment.id == active.segment.id
                        {
                            active.acknowledge(&segment, serial);
                            self.rows.insert(segment.ordinal, segment);
                            trim_grid_cache(&mut self.rows, self.grid_anchor);
                            let intent = self.confirmation_intent.filter(|intent| {
                                intent.serial == serial && intent.segment_id == active.segment.id
                            });
                            if intent.is_some() {
                                self.confirmation_intent = None;
                            }
                            let advance = intent.is_some_and(|intent| intent.advance)
                                && active.serial == serial;
                            self.message = match result.learning {
                            LearningOutcome::Learned => "Confirmado y aprendido en la memoria de escritura",
                            LearningOutcome::Disabled => "Confirmado sin aprendizaje: no hay memoria de escritura compatible",
                            LearningOutcome::UnsupportedCodes => "Confirmado; aprendizaje de códigos DOCX todavía no soportado",
                        }.into();
                            if advance {
                                self.move_segment(1);
                            }
                        }
                        self.request_qa();
                        if let Some(active) = &self.active_draft {
                            let source = active.segment.source.clone();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                        }
                    }
                    WorkerData::Memories(memories, selected) => {
                        self.memories = memories;
                        self.write_memory_id = selected;
                        if let Some(active) = &self.active_draft {
                            let source = active.segment.source.clone();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                        }
                    }
                    WorkerData::History(segment) => {
                        self.invalidate();
                        if let Some(segment) = segment {
                            self.current_document_id = Some(segment.document_id);
                            self.grid_scroll
                                .scroll_to_item(segment.ordinal, ScrollStrategy::Center);
                            self.target_input.set_text(segment.target.clone());
                            self.active_draft = Some(DraftState::new(segment));
                            if let Some(a) = &self.active_draft {
                                let source = a.segment.source.clone();
                                let (sl, tl) = self.document_languages();
                                self.lookup(source, sl, tl, false);
                            }
                        }
                        self.message = "History state restored and committed".into();
                        self.request_qa();
                    }
                    WorkerData::Matches(matches) => {
                        if reply.id == self.latest_matches_req
                            && matches!(pending, PendingOp::Matches(g) if g == self.selection_generation)
                        {
                            self.matches = matches;
                        }
                    }
                    WorkerData::Progress(id, total, translated) => {
                        if Some(id) == self.current_document_id {
                            self.progress = (total, translated);
                        }
                    }
                    WorkerData::Done(msg) => {
                        if matches!(pending, PendingOp::Operation) {
                            let selected = self.active_draft.as_ref().map(|a| a.segment.id);
                            self.invalidate();
                            if let Some(id) = selected {
                                self.select_segment(id);
                            }
                        }
                        self.message = msg;
                    }
                    WorkerData::Closed => {
                        self.opened = false;
                    }
                    WorkerData::Qa(issues) => {
                        if matches!(pending, PendingOp::Qa(g, s) if g == self.selection_generation && self.active_draft.as_ref().is_some_and(|a| a.serial == s))
                        {
                            self.qa_issues = issues;
                        }
                    }
                }
            }
        }

        if refresh_progress {
            self.refresh_progress();
        }
        if self.confirmation_intent.is_some() {
            self.save(true);
        }
        self.close_when_saved();
        if !self.is_dirty()
            && let Some(nav) = self.navigate_after_save.take()
        {
            self.navigate_to(nav);
        }
    }
}

impl Render for LumenCatApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Debounce auto-save if dirty
        if self.is_dirty() {
            self.save(false);
        }

        let entity = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .bg(Theme::bg_app())
            .text_color(Theme::text_primary())
            .flex()
            .flex_col()
            .font_family("Segoe UI")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let ks = &event.keystroke;
                let search = this.search_input.focus_handle.is_focused(window);
                let replacement = this.replacement_input.focus_handle.is_focused(window);
                let source_language = this.source_language_input.focus_handle.is_focused(window);
                let target_language = this.target_language_input.focus_handle.is_focused(window);
                let memory_name = this.memory_name_input.focus_handle.is_focused(window);
                if memory_name {
                    if ks.key == "enter" {
                        this.create_project_memory();
                    } else {
                        this.memory_name_input.handle_key(event, cx);
                    }
                } else if source_language || target_language {
                    if ks.key == "enter" {
                        this.save_project_languages();
                    } else if source_language {
                        this.source_language_input.handle_key(event, cx);
                    } else {
                        this.target_language_input.handle_key(event, cx);
                    }
                } else if ks.modifiers.control && matches!(ks.key.as_str(), "f" | "h") {
                    this.show_replace = ks.key == "h";
                    this.search_input.focus_handle.focus(window);
                } else if ks.key == "escape" {
                    this.is_searching = false;
                    this.search_results.clear();
                    if let Some(active) = &this.active_draft {
                        this.grid_scroll
                            .scroll_to_item(active.segment.ordinal, ScrollStrategy::Center);
                    }
                    this.show_replace = false;
                    this.target_input.focus_handle.focus(window);
                } else if ks.modifiers.control && ks.key == "s" {
                    this.save(true);
                } else if ks.modifiers.shift && ks.key == "f12" {
                    this.export_document_dialog();
                } else if ks.key == "f3" && search && !ks.modifiers.shift {
                    this.perform_concordance();
                } else if search || replacement {
                    if ks.key == "enter" {
                        this.perform_search();
                    } else if replacement {
                        this.replacement_input.handle_key(event, cx);
                    } else {
                        this.search_input.handle_key(event, cx);
                    }
                } else if ks.modifiers.control
                    && (ks.key == ","
                        || (ks.modifiers.alt
                            && matches!(ks.key.as_str(), "up" | "down" | "left" | "right")))
                {
                    this.insert_next_tag();
                } else if ks.modifiers.control && ks.key == "z" {
                    this.history(ks.modifiers.shift);
                } else if ks.modifiers.control && ks.key == "y" {
                    this.history(true);
                } else if ks.modifiers.control && ks.key == "enter" {
                    if ks.modifiers.alt {
                        this.confirm_active();
                    } else {
                        this.confirm_and_next();
                    }
                } else if (ks.modifiers.control || ks.modifiers.alt) && ks.key == "insert" {
                    this.copy_source_to_target();
                } else if ks.modifiers.control && ks.key == "t" {
                    this.insert_tm_match(0);
                } else if ks.modifiers.control
                    && !ks.modifiers.alt
                    && let Ok(index) = ks.key.parse::<usize>()
                    && (1..=9).contains(&index)
                {
                    this.insert_tm_match(index - 1);
                } else if ks.key == "f3" && !ks.modifiers.shift {
                    let query = if this.target_input.selected_all {
                        this.target_input.text.clone()
                    } else {
                        this.active_draft
                            .as_ref()
                            .map_or(String::new(), |a| a.segment.source.clone())
                    };
                    this.search_input.set_text(query);
                    this.perform_concordance();
                } else if ks.modifiers.control && ks.key == "l" {
                    this.toggle_active_lock();
                } else if matches!(ks.key.as_str(), "down" | "arrowdown") {
                    this.move_segment(1);
                } else if matches!(ks.key.as_str(), "up" | "arrowup") {
                    this.move_segment(-1);
                } else if ks.modifiers.alt && ks.key == "c" {
                    this.copy_source_to_target();
                } else if this
                    .active_draft
                    .as_ref()
                    .is_some_and(|a| !a.segment.locked)
                {
                    let before = this.target_input.text.clone();
                    this.target_input.handle_key(event, cx);
                    if before != this.target_input.text {
                        this.on_target_text_changed(Origin::Human);
                    }
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(self.render_header(entity.clone()))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .child(self.render_sidebar(entity.clone()))
                    .child(self.render_center(entity.clone()))
                    .child(self.render_right_panel(entity.clone())),
            )
            .child(self.render_progress(entity.clone()))
            .child(self.render_status_bar())
    }
}

impl LumenCatApp {
    fn render_header(&self, entity: Entity<Self>) -> impl IntoElement {
        let is_ready = self.opened && !self.is_busy() && !self.is_dirty();
        let doc_selected = self.current_document_id.is_some();

        div()
            .h(px(52.))
            .w_full()
            .bg(Theme::bg_surface())
            .border_b_1()
            .border_color(Theme::border_subtle())
            .px_4()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(26.))
                                    .h(px(26.))
                                    .rounded_md()
                                    .bg(Theme::sky())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(rgb(0x0a101d))
                                    .font_weight(FontWeight::BOLD)
                                    .text_sm()
                                    .child("✦"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .text_sm()
                                            .text_color(Theme::text_primary())
                                            .child("LumenCAT"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::text_muted())
                                            .child("Local Translation Studio"),
                                    ),
                            ),
                    )
                    .child(div().h(px(20.)).w(px(1.)).bg(Theme::border_subtle()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(Theme::bg_subtle())
                                    .border_1()
                                    .border_color(Theme::border_subtle())
                                    .text_xs()
                                    .text_color(if self.opened {
                                        Theme::text_primary()
                                    } else {
                                        Theme::text_muted()
                                    })
                                    .child(if self.opened {
                                        let p = PathBuf::from(&self.project_path);
                                        p.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| self.project_path.clone())
                                    } else {
                                        "No project loaded".into()
                                    }),
                            )
                            .child(custom_button(
                                if self.opened {
                                    "Switch Project"
                                } else {
                                    "Open / Create Project"
                                },
                                ButtonVariant::Secondary,
                                !self.is_busy(),
                                {
                                    let entity = entity.clone();
                                    move |_event, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.open_project_dialog();
                                        });
                                    }
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(custom_button(
                        "Importar documento",
                        ButtonVariant::Secondary,
                        is_ready,
                        {
                            let entity = entity.clone();
                            move |_event, _window, cx| {
                                entity.update(cx, |this, _cx| {
                                    this.import_document_dialog();
                                });
                            }
                        },
                    ))
                    .when(self.show_memory_actions, |bar| {
                        bar.child(custom_button(
                            "Importar TMX",
                            ButtonVariant::Secondary,
                            is_ready,
                            {
                                let entity = entity.clone();
                                move |_event, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.import_tmx_dialog();
                                    });
                                }
                            },
                        ))
                    })
                    .child(custom_button(
                        "Exportar documento",
                        ButtonVariant::Secondary,
                        is_ready && doc_selected,
                        {
                            let entity = entity.clone();
                            move |_event, _window, cx| {
                                entity.update(cx, |this, _cx| {
                                    this.export_document_dialog();
                                });
                            }
                        },
                    ))
                    .when(self.show_memory_actions, |bar| {
                        bar.child(custom_button(
                            "Exportar TMX",
                            ButtonVariant::Secondary,
                            is_ready,
                            {
                                let entity = entity.clone();
                                move |_event, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.export_tmx_dialog();
                                    });
                                }
                            },
                        ))
                    })
                    .child(custom_button("Memoria ▾", ButtonVariant::Ghost, true, {
                        let entity = entity.clone();
                        move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.show_memory_actions = !this.show_memory_actions;
                                cx.notify();
                            });
                        }
                    }))
                    .child(div().h(px(20.)).w(px(1.)).bg(Theme::border_subtle()))
                    .child(history_button(false, !self.is_busy(), {
                        let entity = entity.clone();
                        move |_event, _window, cx| {
                            entity.update(cx, |this, _cx| {
                                this.history(false);
                            });
                        }
                    }))
                    .child(history_button(true, !self.is_busy(), {
                        let entity = entity.clone();
                        move |_event, _window, cx| {
                            entity.update(cx, |this, _cx| {
                                this.history(true);
                            });
                        }
                    })),
            )
    }

    fn render_sidebar(&self, entity: Entity<Self>) -> impl IntoElement {
        let docs = self.documents.clone();
        let curr_id = self.current_document_id;
        let is_busy = self.is_busy();
        let is_dirty = self.is_dirty();

        div()
            .w(px(210.))
            .h_full()
            .bg(Theme::bg_sidebar())
            .border_r_1()
            .border_color(Theme::border_subtle())
            .flex()
            .flex_col()
            .child(
                div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().child("Idiomas de importación"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(Theme::text_secondary())
                            .child("Origen"),
                    )
                    .child(input_field(&self.source_language_input, entity.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(Theme::text_secondary())
                            .child("Destino"),
                    )
                    .child(input_field(&self.target_language_input, entity.clone()))
                    .child(custom_button(
                        "Guardar idiomas",
                        ButtonVariant::Secondary,
                        self.opened && !is_busy && !is_dirty,
                        {
                            let entity = entity.clone();
                            move |_, _, cx| {
                                entity.update(cx, |this, _| this.save_project_languages());
                            }
                        },
                    )),
            )
            .when(self.show_memory_actions, |sidebar| {
                sidebar.child(self.render_memory_resources(entity.clone()))
            })
            .child(
                div()
                    .px_3()
                    .py_2p5()
                    .border_b_1()
                    .border_color(Theme::border_subtle())
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(Theme::text_secondary())
                            .child("DOCUMENTS"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_full()
                            .bg(Theme::bg_subtle())
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .child(format!("{}", docs.len())),
                    ),
            )
            .child(
                div()
                    .id("sidebar_docs")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(if docs.is_empty() {
                        vec![
                            div()
                                .p_4()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .text_center()
                                .child(
                                    div().text_xs().text_color(Theme::text_muted()).child(
                                        "No documents yet.\nImport DOCX, XLIFF, or TXT above.",
                                    ),
                                )
                                .into_any_element(),
                        ]
                    } else {
                        docs.into_iter()
                            .map(|doc| {
                                let doc_id = doc.id;
                                let doc_name = doc.name;
                                let doc_sl = doc.source_lang;
                                let doc_tl = doc.target_lang;
                                let seg_count = doc.segment_count;
                                let is_selected = curr_id == Some(doc_id);
                                let fmt = PathBuf::from(&doc_name)
                                    .extension()
                                    .map(|e| e.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "doc".into());

                                let entity_cb = entity.clone();
                                div()
                                    .p_2p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .bg(if is_selected {
                                        Theme::bg_selected()
                                    } else {
                                        Theme::bg_surface()
                                    })
                                    .border_1()
                                    .border_color(if is_selected {
                                        Theme::border_selected()
                                    } else {
                                        Theme::border_subtle()
                                    })
                                    .hover(|s| s.bg(Theme::bg_hover()))
                                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                        if !is_busy && !is_dirty {
                                            entity_cb.update(cx, |this, _cx| {
                                                this.current_document_id = Some(doc_id);
                                                this.invalidate();
                                                this.refresh_progress();
                                                this.navigate_to(PendingNav::Ordinal(doc_id, 0));
                                            });
                                        }
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .items_center()
                                            .mb_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(if is_selected {
                                                        Theme::sky()
                                                    } else {
                                                        Theme::text_primary()
                                                    })
                                                    .child(doc_name),
                                            )
                                            .child(format_badge(&fmt)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .text_xs()
                                            .text_color(Theme::text_muted())
                                            .child(format!("{} → {}", doc_sl, doc_tl))
                                            .child(format!("{} segs", seg_count)),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
    }

    fn render_memory_resources(&self, entity: Entity<Self>) -> impl IntoElement {
        let ready = self.opened && !self.is_busy() && !self.is_dirty();
        div()
            .id("memory-resources")
            .max_h(px(260.))
            .overflow_y_scroll()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().child("Memorias del proyecto"))
            .child(input_field(&self.memory_name_input, entity.clone()))
            .child(custom_button(
                "Crear memoria",
                ButtonVariant::Secondary,
                ready,
                {
                    let entity = entity.clone();
                    move |_, _, cx| {
                        entity.update(cx, |this, _| this.create_project_memory());
                    }
                },
            ))
            .child(custom_button(
                "Desactivar aprendizaje",
                ButtonVariant::Ghost,
                ready,
                {
                    let entity = entity.clone();
                    move |_, _, cx| {
                        entity.update(cx, |this, _| {
                            this.send(WorkerTask::SelectWriteMemory(None), PendingOp::Operation);
                        });
                    }
                },
            ))
            .children(
                self.memories
                    .iter()
                    .filter(|m| {
                        m.source_lang.eq_ignore_ascii_case(&self.source_lang)
                            && m.target_lang.eq_ignore_ascii_case(&self.target_lang)
                    })
                    .map(|memory| {
                        let id = memory.id;
                        let selected = self.write_memory_id == Some(id);
                        let writable = memory.writable;
                        let enabled = memory.enabled;
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .py_2()
                            .child(div().text_xs().child(memory.name.clone()))
                            .child(custom_button(
                                if selected {
                                    "Memoria de escritura"
                                } else {
                                    "Usar para aprender"
                                },
                                if selected {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                },
                                ready && writable && enabled,
                                {
                                    let entity = entity.clone();
                                    move |_, _, cx| {
                                        entity.update(cx, |this, _| {
                                            this.send(
                                                WorkerTask::SelectWriteMemory(Some(id)),
                                                PendingOp::Operation,
                                            );
                                        });
                                    }
                                },
                            ))
                            .child(custom_button(
                                if writable {
                                    "Pasar a solo lectura"
                                } else {
                                    "Permitir escritura"
                                },
                                ButtonVariant::Ghost,
                                ready,
                                {
                                    let entity = entity.clone();
                                    move |_, _, cx| {
                                        entity.update(cx, |this, _| {
                                            this.send(
                                                WorkerTask::ConfigureMemory(id, !writable, enabled),
                                                PendingOp::Operation,
                                            );
                                        });
                                    }
                                },
                            ))
                            .child(custom_button(
                                if enabled {
                                    "Excluir de búsquedas"
                                } else {
                                    "Incluir en búsquedas"
                                },
                                ButtonVariant::Ghost,
                                ready,
                                {
                                    let entity = entity.clone();
                                    move |_, _, cx| {
                                        entity.update(cx, |this, _| {
                                            this.send(
                                                WorkerTask::ConfigureMemory(id, writable, !enabled),
                                                PendingOp::Operation,
                                            );
                                        });
                                    }
                                },
                            ))
                    }),
            )
    }

    fn render_center(&mut self, entity: Entity<Self>) -> impl IntoElement {
        let is_searching = self.is_searching;
        let search_count = self.search_results.len();

        let doc_count = self
            .documents
            .iter()
            .find(|d| Some(d.id) == self.current_document_id)
            .map_or(0, |d| d.segment_count);

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(Theme::bg_app())
            .overflow_hidden()
            .child(self.render_search_toolbar(entity.clone()))
            .child(self.render_bilingual_grid(
                entity.clone(),
                doc_count,
                is_searching,
                search_count,
            ))
            .child(self.render_translation_studio(entity))
    }

    fn render_search_toolbar(&self, entity: Entity<Self>) -> impl IntoElement {
        let search = entity.clone();
        let replace = entity.clone();
        let scope = entity.clone();
        let clear = entity.clone();
        let toggle = entity.clone();
        div()
            .w_full()
            .bg(Theme::bg_surface())
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(input_field(&self.search_input, entity.clone()))
                    .child(custom_button(
                        if self.document_scope {
                            "Documento"
                        } else {
                            "Segmento"
                        },
                        ButtonVariant::Ghost,
                        true,
                        move |_, _, cx| {
                            scope.update(cx, |this, cx| {
                                this.document_scope = !this.document_scope;
                                cx.notify();
                            });
                        },
                    ))
                    .child(custom_button(
                        "Buscar",
                        ButtonVariant::Secondary,
                        self.opened,
                        move |_, _, cx| {
                            search.update(cx, |this, cx| {
                                this.perform_search();
                                cx.notify();
                            });
                        },
                    ))
                    .child(custom_button(
                        "Reemplazar",
                        ButtonVariant::Ghost,
                        self.opened,
                        move |_, window, cx| {
                            toggle.update(cx, |this, cx| {
                                this.show_replace = !this.show_replace;
                                this.search_input.focus_handle.focus(window);
                                cx.notify();
                            });
                        },
                    ))
                    .child(custom_button(
                        "Limpiar",
                        ButtonVariant::Ghost,
                        self.is_searching,
                        move |_, _, cx| {
                            clear.update(cx, |this, cx| {
                                this.is_searching = false;
                                if let Some(active) = &this.active_draft {
                                    this.grid_scroll.scroll_to_item(
                                        active.segment.ordinal,
                                        ScrollStrategy::Center,
                                    );
                                }
                                this.search_input.set_text("");
                                cx.notify();
                            });
                        },
                    )),
            )
            .when(self.show_replace, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(input_field(&self.replacement_input, entity.clone()))
                        .child(custom_button(
                            "Aplicar al destino",
                            ButtonVariant::Secondary,
                            self.opened && !self.is_dirty() && !self.is_busy(),
                            move |_, _, cx| {
                                replace.update(cx, |this, cx| {
                                    this.replace_targets();
                                    cx.notify();
                                });
                            },
                        )),
                )
            })
    }

    fn render_bilingual_grid(
        &mut self,
        entity: Entity<Self>,
        doc_count: usize,
        is_searching: bool,
        search_count: usize,
    ) -> impl IntoElement {
        let active_id = self.active_draft.as_ref().map(|d| d.segment.id);

        let count = if is_searching {
            search_count
        } else {
            doc_count
        };

        div()
            .h(px(260.))
            .w_full()
            .border_b_1()
            .border_color(Theme::border_subtle())
            .bg(Theme::bg_card())
            .flex()
            .flex_col()
            .child(
                // Table Header
                div()
                    .h(px(28.))
                    .w_full()
                    .bg(Theme::bg_subtle())
                    .border_b_1()
                    .border_color(Theme::border_subtle())
                    .flex()
                    .items_center()
                    .px_3()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::text_secondary())
                    .child(div().w(px(50.)).child("#"))
                    .child(div().flex_1().child("SOURCE SEGMENT"))
                    .child(div().w(px(110.)).child("ESTADO / TM"))
                    .child(div().flex_1().child("TARGET TRANSLATION")),
            )
            .child(
                div()
                    .id("grid_rows")
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(if count == 0 {
                        div()
                            .p_6()
                            .text_center()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .child(if is_searching {
                                "No matching segments found."
                            } else {
                                "No segments in this document."
                            })
                            .into_any_element()
                    } else {
                        let generation = self.generation;
                        uniform_list("virtual_grid_rows", count, move |range, _, cx| {
                            if !is_searching {
                                let request_entity = entity.clone();
                                let request_range = range.clone();
                                cx.defer(move |cx| {
                                    request_entity.update(cx, |this, _| {
                                        if this.generation == generation && !this.is_searching {
                                            this.request_grid_pages(request_range);
                                        }
                                    })
                                });
                            }
                            entity.update(cx, |this, _| {
                                range
                                    .map(|index| {
                                        let (seg_id, ord, state, locked, src_preview, tgt_preview) =
                                            if is_searching {
                                                if let Some(row) = this.search_results.get(index) {
                                                    (
                                                        row.id,
                                                        row.ordinal,
                                                        row.state,
                                                        row.locked,
                                                        row.source.clone(),
                                                        row.target.clone(),
                                                    )
                                                } else {
                                                    (
                                                        0,
                                                        index,
                                                        SegmentState::Draft,
                                                        false,
                                                        "...".into(),
                                                        "...".into(),
                                                    )
                                                }
                                            } else {
                                                if let Some(row) = this.rows.get(&index) {
                                                    (
                                                        row.id,
                                                        row.ordinal,
                                                        row.state,
                                                        row.locked,
                                                        row.source.clone(),
                                                        row.target.clone(),
                                                    )
                                                } else {
                                                    (
                                                        0,
                                                        index,
                                                        SegmentState::Draft,
                                                        false,
                                                        "Loading segment...".into(),
                                                        String::new(),
                                                    )
                                                }
                                            };

                                        let is_active = Some(seg_id) == active_id && seg_id != 0;
                                        let entity_click = entity.clone();

                                        div()
                                            .h(px(36.))
                                            .flex_shrink_0()
                                            .w_full()
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .border_b_1()
                                            .border_color(Theme::border_subtle())
                                            .cursor_pointer()
                                            .bg(if is_active {
                                                Theme::bg_selected()
                                            } else {
                                                Theme::bg_card()
                                            })
                                            .hover(|s| s.bg(Theme::bg_hover()))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                move |_event, _window, cx| {
                                                    if seg_id != 0 {
                                                        entity_click.update(cx, |this, cx| {
                                                            this.target_input
                                                                .focus_handle
                                                                .focus(_window);
                                                            this.select_segment(seg_id);
                                                            cx.notify();
                                                        });
                                                    }
                                                },
                                            )
                                            .child(
                                                div()
                                                    .w(px(50.))
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(if is_active {
                                                        Theme::sky()
                                                    } else {
                                                        Theme::text_muted()
                                                    })
                                                    .child(format!("{:>4}", ord + 1)),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_ellipsis()
                                                    .text_xs()
                                                    .text_color(Theme::text_primary())
                                                    .overflow_hidden()
                                                    .child(if src_preview.len() > 70 {
                                                        format!(
                                                            "{}…",
                                                            src_preview
                                                                .chars()
                                                                .take(67)
                                                                .collect::<String>()
                                                        )
                                                    } else {
                                                        src_preview
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .w(px(110.))
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(
                                                                if state == SegmentState::Confirmed
                                                                {
                                                                    Theme::emerald()
                                                                } else {
                                                                    Theme::amber()
                                                                },
                                                            )
                                                            .child(if locked {
                                                                "🔒"
                                                            } else if state
                                                                == SegmentState::Confirmed
                                                            {
                                                                "✓"
                                                            } else {
                                                                "✎"
                                                            }),
                                                    )
                                                    .when(
                                                        this.applied_tm.contains_key(&seg_id),
                                                        |d| {
                                                            d.child(format!(
                                                                "TM {:.0}%",
                                                                this.applied_tm[&seg_id]
                                                            ))
                                                        },
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_ellipsis()
                                                    .text_xs()
                                                    .text_color(if tgt_preview.is_empty() {
                                                        Theme::text_muted()
                                                    } else {
                                                        Theme::text_accent()
                                                    })
                                                    .overflow_hidden()
                                                    .child(if tgt_preview.is_empty() {
                                                        "— empty —".into()
                                                    } else if tgt_preview.len() > 70 {
                                                        format!(
                                                            "{}…",
                                                            tgt_preview
                                                                .chars()
                                                                .take(67)
                                                                .collect::<String>()
                                                        )
                                                    } else {
                                                        tgt_preview
                                                    }),
                                            )
                                            .into_any_element()
                                    })
                                    .collect::<Vec<_>>()
                            })
                        })
                        .with_width_from_item(Some(self.grid_anchor))
                        .track_scroll(self.grid_scroll.clone())
                        .size_full()
                        .into_any_element()
                    }),
            )
    }

    fn request_grid_pages(&mut self, range: std::ops::Range<usize>) {
        let Some(document) = self.current_document_id else {
            return;
        };
        let count = self
            .documents
            .iter()
            .find(|d| d.id == document)
            .map_or(0, |d| d.segment_count);
        self.grid_anchor = range.start;
        let start = range.start.saturating_sub(128) / 128 * 128;
        let end = range.end.saturating_add(128).min(count);
        for page in (start..end).step_by(128) {
            let page_end = page.saturating_add(128).min(count);
            if !self.requested_pages.contains(&page)
                && (page..page_end).any(|index| !self.rows.contains_key(&index))
                && self.send(
                    WorkerTask::Page(document, page, String::new()),
                    PendingOp::Page(self.generation, page),
                )
            {
                self.requested_pages.push(page);
            }
        }
        trim_grid_cache(&mut self.rows, self.grid_anchor);
    }

    fn render_translation_studio(&self, entity: Entity<Self>) -> impl IntoElement {
        let (has_active, ord, state, locked, origin_str, src_text) =
            if let Some(active) = &self.active_draft {
                (
                    true,
                    active.segment.ordinal + 1,
                    active.segment.state,
                    active.segment.locked,
                    active.segment.origin.as_str().to_string(),
                    active.segment.source.clone(),
                )
            } else {
                (
                    false,
                    0,
                    SegmentState::Draft,
                    false,
                    "None".into(),
                    "Select a segment above to begin translating.".into(),
                )
            };

        let entity_confirm = entity.clone();
        let entity_copy = entity.clone();
        let entity_lock = entity.clone();
        let entity_prev = entity.clone();
        let entity_next = entity.clone();

        let target_text = self.target_input.text.clone();
        let cursor_pos = self.target_input.cursor;

        div()
            .flex_1()
            .p_4()
            .bg(Theme::bg_app())
            .flex()
            .flex_col()
            .gap_3()
            .child(
                // Studio Header & Badges
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_base()
                                    .text_color(Theme::text_primary())
                                    .child(if has_active {
                                        format!("Segment #{ord}")
                                    } else {
                                        "Translation Studio".into()
                                    }),
                            )
                            .child(if has_active {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(status_badge(state, locked))
                                    .child(
                                        div()
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(Theme::bg_subtle())
                                            .text_xs()
                                            .text_color(Theme::text_muted())
                                            .child(format!("Origin: {origin_str}")),
                                    )
                            } else {
                                div()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(custom_button(
                                "Anterior",
                                ButtonVariant::Ghost,
                                has_active,
                                move |_event, _window, cx| {
                                    entity_prev.update(cx, |this, _cx| {
                                        this.move_segment(-1);
                                    });
                                },
                            ))
                            .child(custom_button(
                                "Siguiente",
                                ButtonVariant::Ghost,
                                has_active,
                                move |_event, _window, cx| {
                                    entity_next.update(cx, |this, _cx| {
                                        this.move_segment(1);
                                    });
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .gap_3()
                    .child(
                        // Source Card (Read-Only)
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .rounded_md()
                            .bg(Theme::bg_card())
                            .border_1()
                            .border_color(Theme::border_subtle())
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(Theme::text_secondary())
                                            .child("ORIGEN"),
                                    )
                                    .child(custom_button(
                                        "Copiar origen",
                                        ButtonVariant::Ghost,
                                        has_active && !locked,
                                        move |_event, _window, cx| {
                                            entity_copy.update(cx, |this, _cx| {
                                                this.copy_source_to_target();
                                            });
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .line_height(px(22.))
                                    .text_color(Theme::text_primary())
                                    .child(inline_text(&src_text)),
                            ),
                    )
                    .child(
                        // Target Editor Card
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .rounded_md()
                            .bg(Theme::bg_card())
                            .border_1()
                            .border_color(if locked {
                                Theme::border_subtle()
                            } else {
                                Theme::border_accent()
                            })
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(Theme::text_accent())
                                                    .child("DESTINO"),
                                            )
                                            .child(if locked {
                                                div().text_xs().text_color(Theme::slate()).child(
                                                    "(Locked - Click Lock button below to unlock)",
                                                )
                                            } else {
                                                div()
                                            }),
                                    )
                                    .child(div().text_xs().text_color(Theme::text_muted()).child(
                                        format!(
                                            "{} palabras · {} caracteres",
                                            editing::words(&target_text),
                                            target_text.chars().count(),
                                        ),
                                    )),
                            )
                            .child(
                                // Target Text Area with Cursor
                                div()
                                    .id("target_editor_scroll")
                                    .relative()
                                    .child(input::native_input(
                                        self.target_input.focus_handle.clone(),
                                        entity.clone(),
                                    ))
                                    .track_focus(&self.target_input.focus_handle)
                                    .on_mouse_down(MouseButton::Left, {
                                        let focus = self.target_input.focus_handle.clone();
                                        move |_, window, _| focus.focus(window)
                                    })
                                    .flex_1()
                                    .p_2()
                                    .rounded_md()
                                    .bg(Theme::bg_app())
                                    .border_1()
                                    .border_color(Theme::border_subtle())
                                    .overflow_y_scroll()
                                    .text_sm()
                                    .line_height(px(24.))
                                    .child(if target_text.is_empty() {
                                        div().text_color(Theme::text_muted()).child(if locked {
                                    "This segment is locked from editing."
                                } else {
                                    "Type translation here... (or insert from TM on the right)"
                                })
                                    } else {
                                        let before =
                                            &target_text[..cursor_pos.min(target_text.len())];
                                        let after =
                                            &target_text[cursor_pos.min(target_text.len())..];
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .flex_wrap()
                                            .child(inline_text(before))
                                            .child(div().w(px(2.)).h(px(18.)).bg(Theme::sky()))
                                            .child(inline_text(after))
                                    }),
                            )
                            .child(
                                // Action buttons
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().flex().items_center().gap_2().child(
                                        custom_button(
                                            if locked { "Desbloquear" } else { "Bloquear" },
                                            ButtonVariant::Secondary,
                                            has_active,
                                            move |_event, _window, cx| {
                                                entity_lock.update(cx, |this, _cx| {
                                                    this.toggle_active_lock();
                                                });
                                            },
                                        ),
                                    ))
                                    .child(div().flex().items_center().gap_2().child(
                                        custom_button(
                                            "Confirmar y avanzar",
                                            ButtonVariant::Success,
                                            has_active && !locked,
                                            move |_event, _window, cx| {
                                                entity_confirm.update(cx, |this, _cx| {
                                                    this.confirm_and_next();
                                                });
                                            },
                                        ),
                                    )),
                            ),
                    ),
            )
    }

    fn render_right_panel(&self, entity: Entity<Self>) -> impl IntoElement {
        let active_tab = self.active_tab;
        let matches = self.matches.clone();
        let qa_issues = self.qa_issues.clone();
        let is_locked = self.active_draft.as_ref().is_some_and(|d| d.segment.locked);

        let entity_tm_tab = entity.clone();
        let entity_qa_tab = entity.clone();

        div()
            .w(px(310.))
            .h_full()
            .bg(Theme::bg_sidebar())
            .border_l_1()
            .border_color(Theme::border_subtle())
            .flex()
            .flex_col()
            .child(
                // Tab switcher
                div()
                    .h(px(40.))
                    .w_full()
                    .border_b_1()
                    .border_color(Theme::border_subtle())
                    .flex()
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .bg(if active_tab == RightTab::TranslationMemory {
                                Theme::bg_surface()
                            } else {
                                Theme::bg_sidebar()
                            })
                            .text_color(if active_tab == RightTab::TranslationMemory {
                                Theme::sky()
                            } else {
                                Theme::text_muted()
                            })
                            .border_b_2()
                            .border_color(if active_tab == RightTab::TranslationMemory {
                                Theme::sky()
                            } else {
                                rgba(0x00000000)
                            })
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                entity_tm_tab.update(cx, |this, _cx| {
                                    this.active_tab = RightTab::TranslationMemory;
                                });
                            })
                            .child(format!("TM MATCHES ({})", matches.len())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .bg(if active_tab == RightTab::QualityAssurance {
                                Theme::bg_surface()
                            } else {
                                Theme::bg_sidebar()
                            })
                            .text_color(if active_tab == RightTab::QualityAssurance {
                                if !qa_issues.is_empty() {
                                    Theme::rose()
                                } else {
                                    Theme::sky()
                                }
                            } else {
                                Theme::text_muted()
                            })
                            .border_b_2()
                            .border_color(if active_tab == RightTab::QualityAssurance {
                                if !qa_issues.is_empty() {
                                    Theme::rose()
                                } else {
                                    Theme::sky()
                                }
                            } else {
                                rgba(0x00000000)
                            })
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                entity_qa_tab.update(cx, |this, _cx| {
                                    this.active_tab = RightTab::QualityAssurance;
                                });
                            })
                            .child(format!("QA ALERTS ({})", qa_issues.len())),
                    ),
            )
            .child(
                div()
                    .id("right_panel_scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(if active_tab == RightTab::TranslationMemory {
                        if matches.is_empty() {
                            vec![
                                div()
                                    .p_6()
                                    .text_center()
                                    .text_xs()
                                    .text_color(Theme::text_muted())
                                    .child("Sin coincidencias para este segmento.\nImporta una memoria TMX para consultar sugerencias locales.")
                                    .into_any_element(),
                            ]
                        } else {
                            matches
                                .iter()
                                .enumerate()
                                .map(|(idx, tm)| {
                                    let entity_insert = entity.clone();
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(Theme::bg_card())
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .flex()
                                                .justify_between()
                                                .items_center()
                                                .child(tm_badge(tm.score, tm.exact))
                                                .child(custom_button(
                                                    "Apply Match",
                                                    ButtonVariant::Secondary,
                                                    !is_locked,
                                                    move |_event, _window, cx| {
                                                        entity_insert.update(cx, |this, _cx| {
                                                            this.insert_tm_match(idx);
                                                        });
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_secondary())
                                                .child(format!("{} · {} · Src: {}", tm.memory_name, if tm.learned_from.is_some() { "Aprendida" } else { "Importada" }, tm.source)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(Theme::emerald())
                                                .child(format!("Tgt: {}", tm.target)),
                                        )
                                        .into_any_element()
                                })
                                .collect()
                        }
                    } else {
                        if qa_issues.is_empty() {
                            vec![
                                div()
                                    .p_6()
                                    .text_center()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(Theme::emerald())
                                            .font_weight(FontWeight::BOLD)
                                            .text_sm()
                                            .child("✓ All QA Checks Passed"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::text_muted())
                                            .child("No number mismatches, punctuation discrepancies, or length anomalies detected."),
                                    )
                                    .into_any_element(),
                            ]
                        } else {
                            qa_issues
                                .iter()
                                .map(|issue| {
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(Theme::rose_bg())
                                        .border_1()
                                        .border_color(Theme::rose())
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::rose())
                                                .child(format!("⚠️ {}", issue.code)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_primary())
                                                .child(issue.message.clone()),
                                        )
                                        .into_any_element()
                                })
                                .collect()
                        }
                    }),
            )
    }

    fn render_progress(&self, entity: Entity<Self>) -> impl IntoElement {
        let (total, mut translated) = self.progress;
        if let Some(active) = &self.active_draft {
            let was_translated = self
                .rows
                .get(&active.segment.ordinal)
                .is_some_and(|s| !s.target.trim().is_empty());
            let is_translated = !active.segment.target.trim().is_empty();
            if was_translated != is_translated {
                translated = if is_translated {
                    translated + 1
                } else {
                    translated.saturating_sub(1)
                };
            }
        }
        div()
            .px_4()
            .py_1()
            .bg(Theme::bg_surface())
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                entity.update(cx, |this, cx| {
                    if let Some(row) = this.rows.values().find(|s| s.target.trim().is_empty()) {
                        this.select_segment(row.id);
                    }
                    cx.notify();
                });
            })
            .child(progress_bar(translated, total))
            .child(
                div()
                    .text_xs()
                    .text_color(Theme::text_muted())
                    .child(format!(
                        "{} pendientes · clic para ir a un pendiente",
                        total.saturating_sub(translated)
                    )),
            )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let is_dirty = self.is_dirty();

        div()
            .h(px(30.))
            .w_full()
            .bg(Theme::bg_surface())
            .border_t_1()
            .border_color(Theme::border_subtle())
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(
                                if self.save_error {
                                    Theme::rose()
                                } else if is_dirty {
                                    Theme::amber()
                                } else {
                                    Theme::emerald()
                                },
                            ))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if self.save_error {
                                        Theme::rose()
                                    } else if is_dirty {
                                        Theme::amber()
                                    } else {
                                        Theme::emerald()
                                    })
                                    .child(if self.save_error {
                                        "Save Error"
                                    } else if is_dirty {
                                        "Saving transaction..."
                                    } else {
                                        "All changes saved to disk"
                                    }),
                            ),
                    )
                    .child(div().h(px(14.)).w(px(1.)).bg(Theme::border_subtle()))
                    .child(
                        div()
                            .text_color(Theme::text_secondary())
                            .child(self.message.clone()),
                    ),
            )
            .child(
                div().flex().items_center().gap_3().child(
                    div()
                        .text_color(Theme::text_muted())
                        .child("100% Local · SQLite ACID · No Cloud · AI Disabled"),
                ),
            )
    }
}

#[cfg(test)]
mod cache_tests {
    use super::trim_grid_cache;
    use crate::model::{Origin, Segment, SegmentState};

    fn segment(ordinal: usize) -> Segment {
        Segment {
            id: ordinal as i64 + 1,
            document_id: 1,
            ordinal,
            external_id: ordinal.to_string(),
            source: "café 世界".into(),
            target: String::new(),
            state: SegmentState::Draft,
            locked: false,
            origin: Origin::Imported,
            revision: 0,
        }
    }

    #[test]
    fn viewport_cache_keeps_nearby_rows_and_evicts_previous_regions() {
        let mut rows = (0..2048)
            .map(|ordinal| (ordinal, segment(ordinal)))
            .collect();
        trim_grid_cache(&mut rows, 1500);
        assert_eq!(rows.len(), 1024);
        assert!((1200..1800).all(|ordinal| rows.contains_key(&ordinal)));
        assert!(!rows.contains_key(&0));
        rows.extend((99_000..100_000).map(|ordinal| (ordinal, segment(ordinal))));
        trim_grid_cache(&mut rows, 99_500);
        assert_eq!(rows.len(), 1024);
        assert!(rows.contains_key(&99_500));
        assert!(!rows.contains_key(&1500));
    }
}
