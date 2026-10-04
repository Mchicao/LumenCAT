pub mod components;
pub use crate::editing;
mod input;
pub mod runtime;
pub mod theme;

use crate::{
    model::{
        Cancellation, DocumentInfo, EditCommand, Origin, QaIssue, Segment, SegmentState, TmMatch,
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
            self.saved = serial;
            self.saving = false;
        }
    }
}

enum PendingOp {
    Open,
    Operation,
    Page(u64, usize),
    Search(u64),
    Select(u64),
    Save(u64),
    Matches(u64),
    History,
    Close,
    Qa(u64, u64),
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

    // Grid data
    pub rows: BTreeMap<usize, Segment>,
    requested_pages: Vec<usize>,

    // Editor data
    pub active_draft: Option<DraftState>,
    pub target_input: InputModel,
    navigate_after_save: Option<PendingNav>,

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
    grid_list: ListState,
    pending_reveal: Option<usize>,
    progress: (usize, usize),
    applied_tm: HashMap<i64, f64>,
    pub search_results: Vec<Segment>,
    pub is_searching: bool,

    // Status
    pub message: String,
    pub save_error: bool,
    message_error: bool,
    pub closing_requested: bool,
    cancellations: HashMap<u64, Cancellation>,
    pub focus_handle: FocusHandle,
    show_memory_actions: bool,
    native_dialog_open: bool,
}

impl Focusable for LumenCatApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
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
            rows: BTreeMap::new(),
            requested_pages: Vec::new(),
            active_draft: None,
            target_input: InputModel::new("Type translation here...", cx),
            navigate_after_save: None,
            matches: Vec::new(),
            qa_issues: Vec::new(),
            active_tab: RightTab::TranslationMemory,
            latest_matches_req: 0,
            search_input: InputModel::new("Search source / target...", cx),
            replacement_input: InputModel::new("Reemplazar por...", cx),
            show_replace: false,
            document_scope: true,
            grid_list: ListState::new(0, ListAlignment::Top, px(600.)),
            pending_reveal: None,
            progress: (0, 0),
            applied_tm: HashMap::new(),
            search_results: Vec::new(),
            is_searching: false,
            message: "Open or create a local project to get started · 100% Offline & Private"
                .into(),
            save_error: false,
            message_error: false,
            closing_requested: false,
            cancellations: HashMap::new(),
            focus_handle: cx.focus_handle(),
            show_memory_actions: false,
            native_dialog_open: false,
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
                if matches!(
                    pending,
                    PendingOp::Operation
                        | PendingOp::History
                        | PendingOp::Search(_)
                        | PendingOp::Open
                ) {
                    self.message_error = false;
                }
                self.pending.insert(self.next_id, pending);
                true
            }
            Err(_) => {
                self.message_error = true;
                self.message = "Background worker busy. Text preserved in editor.".into();
                false
            }
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.active_draft
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
                    | PendingOp::Search(_)
            )
        })
    }

    pub fn invalidate(&mut self) {
        self.cancel_lookups();
        self.generation += 1;
        self.rows.clear();
        self.requested_pages.clear();
        self.search_results.clear();
        self.is_searching = false;
        self.selection_generation += 1;
        self.grid_list.reset(0);
    }

    fn remeasure_rows(&mut self, ordinals: Vec<usize>) {
        if self.is_searching || ordinals.is_empty() {
            return;
        }
        let min = ordinals.iter().min().copied().unwrap_or(0);
        let max = ordinals.iter().max().copied().unwrap_or(0);
        self.grid_list.remeasure_items(min..max + 1);
    }

    fn activate_document(&mut self, id: i64) {
        let Some(doc) = self.documents.iter().find(|d| d.id == id).cloned() else {
            return;
        };
        self.current_document_id = Some(id);
        self.source_lang = doc.source_lang;
        self.target_lang = doc.target_lang;
        self.active_draft = None;
        self.target_input.set_text("");
        self.matches.clear();
        self.qa_issues.clear();
        self.invalidate();
        self.refresh_progress();
        if doc.segment_count > 0 {
            self.navigate_to(PendingNav::Ordinal(id, 0));
        }
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
        if active.segment.target.len() > 1_048_576 {
            self.save_error = true;
            self.message = "Target exceeds 1 MiB limit. Content is safely kept in editor.".into();
            return;
        }
        if active.saving || active.serial == active.saved {
            return;
        }
        if !force
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
        if self.send(WorkerTask::Edit(cmd), PendingOp::Save(serial))
            && let Some(active) = &mut self.active_draft
        {
            active.saving = true;
        }
    }

    pub fn confirm_active(&mut self) {
        if let Some(active) = &mut self.active_draft
            && !active.segment.locked
        {
            active.segment.state = SegmentState::Confirmed;
            active.serial += 1;
            active.changed = Instant::now();
        }
        self.save(true);
    }

    pub fn confirm_and_next(&mut self) {
        if self.active_draft.as_ref().is_none_or(|a| a.segment.locked) {
            return;
        }
        self.confirm_active();
        self.move_segment(1);
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
                self.message_error = false;
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
        if self.native_dialog_open || self.is_dirty() || self.is_busy() {
            return;
        }
        self.native_dialog_open = true;
        let path = rfd::FileDialog::new()
            .add_filter("LumenCAT Project", &["lcat", "db", "sqlite"])
            .save_file();
        self.native_dialog_open = false;
        if let Some(path) = path {
            self.project_path = path.to_string_lossy().to_string();
            self.send(WorkerTask::Open(path), PendingOp::Open);
        }
    }

    pub fn import_document_dialog(&mut self) {
        if self.native_dialog_open || !self.opened || self.is_busy() {
            return;
        }
        self.native_dialog_open = true;
        let path = rfd::FileDialog::new()
            .add_filter("Supported Documents", &["docx", "xlf", "xliff", "txt"])
            .add_filter("Word Documents", &["docx"])
            .add_filter("XLIFF", &["xlf", "xliff"])
            .add_filter("Plain Text", &["txt"])
            .pick_file();
        self.native_dialog_open = false;
        if let Some(path) = path {
            let (sl, tl) = self.document_languages();
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

    pub fn import_tmx_dialog(&mut self) {
        if self.native_dialog_open || !self.opened || self.is_busy() {
            return;
        }
        self.native_dialog_open = true;
        let path = rfd::FileDialog::new()
            .add_filter("TMX Translation Memory", &["tmx"])
            .pick_file();
        self.native_dialog_open = false;
        if let Some(path) = path {
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
        if self.native_dialog_open {
            return;
        }
        if self.is_dirty() || self.is_busy() {
            self.save(true);
            return;
        }
        let Some(doc_id) = self.current_document_id else {
            return;
        };
        self.native_dialog_open = true;
        let path = rfd::FileDialog::new()
            .add_filter("Translated Document", &["docx", "xlf", "xliff", "txt"])
            .save_file();
        self.native_dialog_open = false;
        if let Some(path) = path {
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
        if self.native_dialog_open || !self.opened || self.is_busy() {
            return;
        }
        self.native_dialog_open = true;
        let path = rfd::FileDialog::new()
            .add_filter("TMX Translation Memory", &["tmx"])
            .save_file();
        self.native_dialog_open = false;
        if let Some(path) = path {
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
                self.message_error = true;
                if matches!(pending, PendingOp::Save(_)) {
                    self.save_error = true;
                    if let Some(a) = &mut self.active_draft {
                        a.saving = false;
                    }
                }
                self.message = error.to_string();
            }
            Ok(data) => match data {
                WorkerData::Opened(recovered, docs) => {
                    self.message_error = false;
                    self.opened = true;
                    self.save_error = false;
                    self.active_draft = None;
                    self.target_input.set_text("");
                    self.current_document_id = None;
                    self.progress = (0, 0);
                    self.applied_tm.clear();
                    self.invalidate();
                    self.documents = docs;
                    if let Some(first_doc) = self.documents.first().cloned() {
                        self.activate_document(first_doc.id);
                    }
                    self.message = if recovered {
                        "Clean recovery from previous session. SQLite verified database safely."
                    } else {
                        "Local project opened. Changes are saved transactionally."
                    }
                    .into();
                }
                WorkerData::Documents(docs) => {
                    self.message_error = false;
                    self.documents = docs;
                    if self.current_document_id.is_none()
                        && let Some(first_doc) = self.documents.first().cloned()
                    {
                        self.activate_document(first_doc.id);
                    }
                    self.message = "Documents updated successfully".into();
                }
                WorkerData::Page(rows) => {
                    if matches!(pending, PendingOp::Search(g) if g == self.generation) {
                        self.message_error = false;
                        self.message = format!("{} segmentos encontrados", rows.len());
                        self.search_results = rows;
                        self.grid_list.reset(0);
                    } else if matches!(pending, PendingOp::Page(g, _) if g == self.generation) {
                        let mut changed = Vec::new();
                        for row in rows {
                            changed.push(row.ordinal);
                            self.rows.insert(row.ordinal, row);
                        }
                        self.remeasure_rows(changed);
                    }
                }
                WorkerData::Selected(segment) => {
                    if matches!(pending, PendingOp::Select(g) if g == self.selection_generation) {
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
                        let previous = self
                            .active_draft
                            .as_ref()
                            .filter(|d| d.segment.document_id == segment.document_id)
                            .map(|d| d.segment.ordinal);
                        self.rows.insert(segment.ordinal, segment.clone());
                        self.active_draft = Some(DraftState::new(segment));
                        let mut changed = vec![index];
                        if let Some(p) = previous {
                            changed.push(p);
                        }
                        self.remeasure_rows(changed);
                        self.pending_reveal = Some(index);
                        self.matches.clear();
                        let (sl, tl) = self.document_languages();
                        self.lookup(source, sl, tl, false);
                        self.request_qa();
                    }
                }
                WorkerData::Saved(segment) => {
                    self.message_error = false;
                    let saved_ordinal = if let (PendingOp::Save(serial), Some(active)) =
                        (pending, &mut self.active_draft)
                        && segment.id == active.segment.id
                    {
                        active.acknowledge(&segment, serial);
                        self.rows.insert(segment.ordinal, segment);
                        self.message = "All changes saved safely to SQLite disk".into();
                        Some(active.segment.ordinal)
                    } else {
                        None
                    };
                    if let Some(ordinal) = saved_ordinal {
                        self.remeasure_rows(vec![ordinal]);
                    }
                    self.request_qa();
                }
                WorkerData::History(segment) => {
                    self.message_error = false;
                    self.invalidate();
                    if let Some(segment) = segment {
                        self.current_document_id = Some(segment.document_id);
                        self.target_input.set_text(segment.target.clone());
                        self.rows.insert(segment.ordinal, segment.clone());
                        self.active_draft = Some(DraftState::new(segment));
                        self.pending_reveal =
                            Some(self.active_draft.as_ref().map_or(0, |a| a.segment.ordinal));
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
                    self.message_error = false;
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
            },
        }

        if refresh_progress {
            self.refresh_progress();
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
            .id("lumencat")
            .accessibility_id("lumencat")
            .role(Role::Application)
            .aria_label("LumenCAT")
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
                if ks.key == "tab" {
                    if ks.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                } else if ks.modifiers.control && matches!(ks.key.as_str(), "f" | "h") {
                    this.show_replace = ks.key == "h";
                    this.search_input.focus_handle.focus(window, cx);
                } else if ks.key == "escape" {
                    this.is_searching = false;
                    this.search_results.clear();
                    if let Some(active) = &this.active_draft {
                        this.pending_reveal = Some(active.segment.ordinal);
                        cx.notify();
                    }
                    this.show_replace = false;
                    this.target_input.focus_handle.focus(window, cx);
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
                } else if this.target_input.focus_handle.is_focused(window)
                    && this
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
            .id("header")
            .role(Role::Toolbar)
            .aria_label("Proyecto y documentos")
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
                                    .id("project-name")
                                    .role(Role::Label)
                                    .aria_label(if self.opened {
                                        self.project_path.clone()
                                    } else {
                                        "Sin proyecto".into()
                                    })
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
                                "project-open",
                                if self.opened {
                                    "Switch Project"
                                } else {
                                    "Open / Create Project"
                                },
                                ButtonVariant::Secondary,
                                !self.is_busy() && !self.is_dirty(),
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
                        "document-import",
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
                            "tm-import",
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
                        "document-export",
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
                            "tm-export",
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
                    .child(
                        custom_button("memory-menu", "Memoria ▾", ButtonVariant::Ghost, true, {
                            let entity = entity.clone();
                            move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.show_memory_actions = !this.show_memory_actions;
                                    cx.notify();
                                });
                            }
                        })
                        .aria_expanded(self.show_memory_actions)
                        .on_a11y_action(AccessibleAction::Expand, {
                            let entity = entity.clone();
                            move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.show_memory_actions = true;
                                    cx.notify();
                                });
                            }
                        })
                        .on_a11y_action(AccessibleAction::Collapse, {
                            let entity = entity.clone();
                            move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.show_memory_actions = false;
                                    cx.notify();
                                });
                            }
                        }),
                    )
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
                    .accessibility_id("documents")
                    .role(Role::List)
                    .aria_label(format!("Documentos ({})", docs.len()))
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(if docs.is_empty() {
                        vec![
                            div()
                                .id("documents-empty")
                                .role(Role::Label)
                                .aria_label("Sin documentos. Importa DOCX, XLIFF o TXT.")
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
                                    .id(("document", doc_id as u64))
                                    .accessibility_id(format!("document-{doc_id}"))
                                    .role(Role::ListItem)
                                    .aria_label(doc_name.clone())
                                    .aria_description(format!(
                                        "{doc_sl} → {doc_tl}; {seg_count} segmentos"
                                    ))
                                    .aria_selected(is_selected)
                                    .focusable()
                                    .tab_stop(!is_busy && !is_dirty)
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
                                    .on_click(move |_event, _window, cx| {
                                        if !is_busy && !is_dirty {
                                            entity_cb.update(cx, |this, cx| {
                                                this.activate_document(doc_id);
                                                cx.notify();
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

    fn render_center(&mut self, entity: Entity<Self>) -> impl IntoElement {
        let is_searching = self.is_searching;
        let search_count = self.search_results.len();

        let doc_count = self
            .documents
            .iter()
            .find(|d| Some(d.id) == self.current_document_id)
            .map_or(0, |d| d.segment_count);

        // Request pages for segments that are not loaded yet
        if !is_searching && let Some(doc_id) = self.current_document_id {
            let active_ord = self.active_draft.as_ref().map_or(0, |d| d.segment.ordinal);
            let start_page = (active_ord / 128) * 128;
            if !self.requested_pages.contains(&start_page)
                && !self.rows.contains_key(&active_ord)
                && self.send(
                    WorkerTask::Page(doc_id, start_page, String::new()),
                    PendingOp::Page(self.generation, start_page),
                )
            {
                self.requested_pages.push(start_page);
            }
        }

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(Theme::bg_app())
            .overflow_hidden()
            .child(self.render_search_toolbar(entity.clone()))
            .child(self.render_editor_toolbar(entity.clone()))
            .child(self.render_bilingual_grid(entity, doc_count, is_searching, search_count))
    }

    fn render_search_toolbar(&self, entity: Entity<Self>) -> impl IntoElement {
        let search = entity.clone();
        let replace = entity.clone();
        let scope = entity.clone();
        let clear = entity.clone();
        let toggle = entity.clone();
        div()
            .id("search-toolbar")
            .role(Role::Toolbar)
            .aria_label("Buscar y reemplazar")
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
                    .child(input_field(
                        input::InputField::Search,
                        &self.search_input,
                        entity.clone(),
                    ))
                    .child(custom_button(
                        "search-scope",
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
                        "search-submit",
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
                        "concordance",
                        "Concordancia",
                        ButtonVariant::Ghost,
                        self.opened && !self.search_input.text.trim().is_empty(),
                        {
                            let entity = entity.clone();
                            move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.perform_concordance();
                                    cx.notify();
                                });
                            }
                        },
                    ))
                    .child(custom_button(
                        "replace-toggle",
                        "Reemplazar",
                        ButtonVariant::Ghost,
                        self.opened,
                        move |_, window, cx| {
                            toggle.update(cx, |this, cx| {
                                this.show_replace = !this.show_replace;
                                this.search_input.focus_handle.focus(window, cx);
                                cx.notify();
                            });
                        },
                    ))
                    .child(custom_button(
                        "search-clear",
                        "Limpiar",
                        ButtonVariant::Ghost,
                        self.is_searching,
                        move |_, _, cx| {
                            clear.update(cx, |this, cx| {
                                this.is_searching = false;
                                if let Some(active) = &this.active_draft {
                                    this.pending_reveal = Some(active.segment.ordinal);
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
                        .child(input_field(
                            input::InputField::Replacement,
                            &self.replacement_input,
                            entity.clone(),
                        ))
                        .child(custom_button(
                            "replace-apply",
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
        let count = if is_searching {
            search_count
        } else {
            doc_count
        };

        let old_count = self.grid_list.item_count();
        if old_count != count {
            self.grid_list.splice(0..old_count, count);
            self.grid_list.clone().with_uniform_item_height(px(44.));
        }
        if let Some(target) = self.pending_reveal.take() {
            self.grid_list.scroll_to_reveal_item(target);
        }

        let (source_label, target_label) = {
            let (sl, tl) = self.document_languages();
            (
                format!("SOURCE · {}", sl.to_uppercase()),
                format!("TARGET · {}", tl.to_uppercase()),
            )
        };

        let rows_entity = entity.clone();

        div()
            .id("bilingual-grid")
            .accessibility_id("bilingual-grid")
            .role(Role::Grid)
            .aria_label("Segmentos bilingües")
            .aria_row_count(count)
            .aria_column_count(4)
            .flex_1()
            .min_h(px(0.))
            .w_full()
            .bg(Theme::bg_card())
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(28.))
                    .w_full()
                    .bg(Theme::bg_subtle())
                    .border_b_1()
                    .border_color(Theme::border_subtle())
                    .flex()
                    .items_center()
                    .px_3()
                    .gap_2()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::text_secondary())
                    .child(div().w(px(50.)).child("#"))
                    .child(div().flex_1().min_w_0().child(source_label))
                    .child(div().flex_1().min_w_0().child(target_label))
                    .child(div().w(px(5.))),
            )
            .child(
                div()
                    .id("grid_rows")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .when(count == 0, |d| {
                        d.child(
                            div()
                                .id("segments-empty")
                                .role(Role::Label)
                                .aria_label(if is_searching {
                                    "Sin resultados"
                                } else {
                                    "Sin segmentos traducibles"
                                })
                                .p_6()
                                .text_center()
                                .text_xs()
                                .text_color(Theme::text_muted())
                                .child(if is_searching {
                                    "No matching segments found."
                                } else {
                                    "No segments in this document."
                                }),
                        )
                    })
                    .when(count > 0, |d| {
                        d.child(
                            list(self.grid_list.clone(), move |index, _window, cx| {
                                rows_entity.update(cx, |this, _cx| {
                                    this.render_grid_item(&rows_entity, index)
                                })
                            })
                            .size_full(),
                        )
                    }),
            )
    }

    fn render_grid_item(&mut self, entity: &Entity<Self>, index: usize) -> AnyElement {
        let active_id = self.active_draft.as_ref().map(|d| d.segment.id);
        if self.is_searching {
            return if let Some(row) = self.search_results.get(index) {
                self.render_segment_row(
                    entity,
                    index,
                    row.id,
                    row.ordinal,
                    row.state,
                    row.locked,
                    &row.source,
                    &row.target,
                    false,
                )
            } else {
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(Theme::text_muted())
                    .child("...")
                    .into_any_element()
            };
        }
        if let Some(row) = self.rows.get(&index) {
            let is_active = Some(row.id) == active_id && row.id != 0;
            return self.render_segment_row(
                entity,
                index,
                row.id,
                row.ordinal,
                row.state,
                row.locked,
                &row.source,
                &row.target,
                is_active,
            );
        }
        let page = (index / 128) * 128;
        if !self.requested_pages.contains(&page)
            && let Some(doc_id) = self.current_document_id
            && self.send(
                WorkerTask::Page(doc_id, page, String::new()),
                PendingOp::Page(self.generation, page),
            )
        {
            self.requested_pages.push(page);
        }
        self.render_segment_row(
            entity,
            index,
            0,
            index,
            SegmentState::Draft,
            false,
            "Loading segment...",
            "",
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_segment_row(
        &self,
        entity: &Entity<Self>,
        index: usize,
        seg_id: i64,
        ord: usize,
        state: SegmentState,
        locked: bool,
        src: &str,
        tgt: &str,
        is_active: bool,
    ) -> AnyElement {
        let entity_click = entity.clone();
        let strip_color = if locked {
            Theme::slate()
        } else if state == SegmentState::Confirmed {
            Theme::emerald()
        } else if tgt.trim().is_empty() {
            Theme::rose()
        } else {
            Theme::amber()
        };

        let number = div()
            .id("ordinal")
            .role(Role::Cell)
            .aria_label(format!("{}", ord + 1))
            .aria_column_index(0)
            .w(px(50.))
            .flex()
            .items_center()
            .gap_1()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .text_color(if is_active {
                Theme::sky()
            } else {
                Theme::text_muted()
            })
            .child(if locked {
                div().child("🔒")
            } else if is_active {
                div().text_color(Theme::sky()).child("▶")
            } else {
                div()
            })
            .child(format!("{:>4}", ord + 1));

        let source = if is_active {
            div()
                .id("active-source")
                .accessibility_id("active-source")
                .role(Role::TextInput)
                .aria_label("Origen del segmento activo")
                .aria_value(src.to_string())
                .a11y_synthetic_children(|tree| tree.parent_node().set_read_only())
        } else {
            div()
                .id("source")
                .role(Role::Cell)
                .aria_label("Origen")
                .aria_value(src.to_string())
                .a11y_synthetic_children(|tree| tree.parent_node().set_read_only())
        };
        let source = source
            .aria_column_index(1)
            .flex_1()
            .min_w_0()
            .text_sm()
            .line_height(px(22.))
            .text_color(Theme::text_primary())
            .overflow_hidden()
            .child(inline_text(src));

        let target_aria = if is_active {
            self.target_input.text.clone()
        } else {
            tgt.to_string()
        };
        let mut target = div()
            .id("target")
            .role(Role::Cell)
            .aria_label("Destino")
            .aria_value(target_aria)
            .a11y_synthetic_children(|tree| tree.parent_node().set_read_only())
            .aria_column_index(2)
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .overflow_hidden();

        if let Some(score) = self.applied_tm.get(&seg_id) {
            let badge = div()
                .px_1p5()
                .py_0p5()
                .mr_auto()
                .rounded_sm()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .bg(Theme::violet_bg())
                .text_color(Theme::violet())
                .child(format!("TM {:.0}%", score));
            target = target.child(badge);
        }

        let target = if is_active {
            target.child(
                input::accessible_input(
                    input::InputField::Target,
                    &self.target_input,
                    entity.clone(),
                    !locked,
                )
                .relative()
                .child(input::native_input(
                    self.target_input.focus_handle.clone(),
                    entity.clone(),
                ))
                .track_focus(&self.target_input.focus_handle)
                .on_mouse_down(MouseButton::Left, {
                    let focus = self.target_input.focus_handle.clone();
                    move |_, window, cx| focus.focus(window, cx)
                })
                .w_full()
                .min_h(px(56.))
                .p_2()
                .rounded_md()
                .bg(Theme::bg_app())
                .border_1()
                .border_color(if locked {
                    Theme::border_subtle()
                } else {
                    Theme::border_accent()
                })
                .text_sm()
                .line_height(px(24.))
                .child(if self.target_input.text.is_empty() {
                    div().text_color(Theme::text_muted()).child(if locked {
                        "Este segmento está bloqueado."
                    } else {
                        "Escriba la traducción aquí..."
                    })
                } else {
                    let cursor_pos = self.target_input.cursor.min(self.target_input.text.len());
                    let before = self.target_input.text[..cursor_pos].to_string();
                    let after = self.target_input.text[cursor_pos..].to_string();
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .flex_wrap()
                        .child(inline_text(&before))
                        .child(div().w(px(2.)).h(px(18.)).bg(Theme::sky()))
                        .child(inline_text(&after))
                }),
            )
        } else if tgt.is_empty() {
            target.child(div().text_sm().text_color(Theme::text_muted()).child("—"))
        } else {
            target.child(
                div()
                    .text_sm()
                    .line_height(px(22.))
                    .text_color(Theme::text_accent())
                    .child(inline_text(tgt)),
            )
        };

        let state_str = if state == SegmentState::Confirmed {
            "confirmado"
        } else {
            "borrador"
        };
        let lock_str = if locked { "bloqueado" } else { "editable" };
        let state_cell = div()
            .id("state")
            .role(Role::Cell)
            .aria_label("Estado")
            .aria_value(format!("{state_str}; {lock_str}"))
            .a11y_synthetic_children(|tree| tree.parent_node().set_read_only())
            .aria_column_index(3)
            .w(px(5.))
            .self_stretch()
            .rounded_full()
            .bg(strip_color);

        div()
            .id(("segment", ord))
            .accessibility_id(format!("segment-{seg_id}"))
            .role(Role::Row)
            .aria_label(format!("Segmento {}", ord + 1))
            .aria_description(format!(
                "{}; {}; {}; {}",
                state_str,
                lock_str,
                if is_active {
                    "seleccionado"
                } else {
                    "no seleccionado"
                },
                src,
            ))
            .aria_row_index(index)
            .focusable()
            .tab_stop(false)
            .w_full()
            .px_3()
            .py_2()
            .flex()
            .items_start()
            .gap_2()
            .border_b_1()
            .border_color(Theme::border_subtle())
            .cursor_pointer()
            .bg(if is_active {
                Theme::bg_selected()
            } else {
                Theme::bg_card()
            })
            .hover(|s| s.bg(Theme::bg_hover()))
            .when(is_active, |d| d.border_l_2().border_color(Theme::sky()))
            .on_click(move |_event, _window, cx| {
                if seg_id != 0 {
                    entity_click.update(cx, |this, cx| {
                        this.target_input.focus_handle.focus(_window, cx);
                        if this.is_searching {
                            this.is_searching = false;
                            this.search_results.clear();
                            this.search_input.set_text("");
                        }
                        this.select_segment(seg_id);
                        cx.notify();
                    });
                }
            })
            .child(number)
            .child(source)
            .child(target)
            .child(state_cell)
            .into_any_element()
    }

    fn render_editor_toolbar(&self, entity: Entity<Self>) -> impl IntoElement {
        let (has_active, ord, state, locked, origin_str) = if let Some(active) = &self.active_draft
        {
            (
                true,
                active.segment.ordinal + 1,
                active.segment.state,
                active.segment.locked,
                active.segment.origin.as_str().to_string(),
            )
        } else {
            (false, 0, SegmentState::Draft, false, "None".into())
        };

        let entity_confirm = entity.clone();
        let entity_copy = entity.clone();
        let entity_lock = entity.clone();
        let entity_prev = entity.clone();
        let entity_next = entity.clone();
        let entity_tag = entity.clone();

        let target_text = self.target_input.text.clone();

        div()
            .id("editor-toolbar")
            .role(Role::Toolbar)
            .aria_label("Acciones del segmento")
            .w_full()
            .bg(Theme::bg_surface())
            .border_b_1()
            .border_color(Theme::border_subtle())
            .px_3()
            .py_1p5()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_sm()
                    .text_color(Theme::text_primary())
                    .child(if has_active {
                        format!("Segmento #{ord}")
                    } else {
                        "Sin segmento activo".into()
                    }),
            )
            .when(has_active, |d| {
                d.child(status_badge(state, locked)).child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded_sm()
                        .bg(Theme::bg_subtle())
                        .text_xs()
                        .text_color(Theme::text_muted())
                        .child(format!("Origen: {origin_str}")),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_xs()
                    .text_color(Theme::text_muted())
                    .child(format!(
                        "{} palabras · {} caracteres",
                        editing::words(&target_text),
                        target_text.chars().count(),
                    )),
            )
            .child(custom_button(
                "segment-previous",
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
                "segment-next",
                "Siguiente",
                ButtonVariant::Ghost,
                has_active,
                move |_event, _window, cx| {
                    entity_next.update(cx, |this, _cx| {
                        this.move_segment(1);
                    });
                },
            ))
            .child(custom_button(
                "source-copy",
                "Copiar origen",
                ButtonVariant::Ghost,
                has_active && !locked,
                move |_event, _window, cx| {
                    entity_copy.update(cx, |this, _cx| {
                        this.copy_source_to_target();
                    });
                },
            ))
            .child(custom_button(
                "segment-lock",
                if locked { "Desbloquear" } else { "Bloquear" },
                ButtonVariant::Secondary,
                has_active,
                move |_event, _window, cx| {
                    entity_lock.update(cx, |this, _cx| {
                        this.toggle_active_lock();
                    });
                },
            ))
            .child(
                custom_button(
                    "insert-next-tag",
                    "Etiqueta",
                    ButtonVariant::Ghost,
                    has_active && !locked,
                    move |_, _, cx| {
                        entity_tag.update(cx, |this, cx| {
                            this.insert_next_tag();
                            cx.notify();
                        });
                    },
                )
                .aria_label("Insertar siguiente etiqueta protegida")
                .aria_keyshortcuts("Control+,"),
            )
            .child(custom_button(
                "segment-confirm-next",
                "Confirmar y avanzar",
                ButtonVariant::Success,
                has_active && !locked,
                move |_event, _window, cx| {
                    entity_confirm.update(cx, |this, _cx| {
                        this.confirm_and_next();
                    });
                },
            ))
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
                            .id("tm-tab")
                            .accessibility_id("tm-tab")
                            .role(Role::Tab)
                            .aria_label("Memoria de traducción")
                            .aria_selected(active_tab == RightTab::TranslationMemory)
                            .focusable()
                            .tab_stop(true)
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
                            .on_click(move |_event, _window, cx| {
                                entity_tm_tab.update(cx, |this, cx| {
                                    this.active_tab = RightTab::TranslationMemory;
                                    cx.notify();
                                });
                            })
                            .child(format!("TM MATCHES ({})", matches.len())),
                    )
                    .child(
                        div()
                            .id("qa-tab")
                            .accessibility_id("qa-tab")
                            .role(Role::Tab)
                            .aria_label("Control de calidad")
                            .aria_selected(active_tab == RightTab::QualityAssurance)
                            .focusable()
                            .tab_stop(true)
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
                            .on_click(move |_event, _window, cx| {
                                entity_qa_tab.update(cx, |this, cx| {
                                    this.active_tab = RightTab::QualityAssurance;
                                    cx.notify();
                                });
                            })
                            .child(format!("QA ALERTS ({})", qa_issues.len())),
                    ),
            )
            .child(
                div()
                    .id("right_panel_scroll")
                    .role(Role::TabPanel)
                    .aria_label(if active_tab == RightTab::TranslationMemory { "Coincidencias TM" } else { "Avisos QA" })
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
                                    .id("tm-empty")
                                    .role(Role::Label)
                                    .aria_label("Sin coincidencias TM")
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
                                        .id(("tm-match", idx))
                                        .accessibility_id(format!("tm-match-{}", idx + 1))
                                        .role(Role::Group)
                                        .aria_label(format!("Coincidencia {}: {:.0}%; origen: {}; destino: {}", idx + 1, tm.score, tm.source, tm.target))
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
                                                    format!("tm-apply-{idx}"),
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
                                                .child(format!("Src: {}", tm.source)),
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
                                    .id("qa-empty")
                                    .role(Role::Label)
                                    .aria_label("Sin avisos QA para el segmento activo")
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
                                            .child("No number mismatches, punctuation discrepancies, or identity issues detected."),
                                    )
                                    .into_any_element(),
                            ]
                        } else {
                            qa_issues
                                .iter()
                                .enumerate()
                                .map(|(index, issue)| {
                                    div()
                                        .id(("qa-issue", index))
                                        .role(Role::Alert)
                                        .aria_label(format!("{}: {}", issue.code, issue.message))
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
        let enabled = !self.is_busy()
            && !self.is_dirty()
            && self.rows.values().any(|row| row.target.trim().is_empty());
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
            .id("progress")
            .accessibility_id("translation-progress")
            .role(Role::Button)
            .aria_label("Ir al primer segmento pendiente")
            .aria_description(format!(
                "{translated} de {total} segmentos con destino; {} pendientes",
                total.saturating_sub(translated)
            ))
            .a11y_synthetic_children(move |tree| {
                if !enabled {
                    tree.parent_node().set_disabled();
                }
            })
            .when(enabled, |d| d.focusable().tab_stop(true))
            .px_4()
            .py_1()
            .bg(Theme::bg_surface())
            .when(enabled, |d| {
                d.cursor_pointer().on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        if let Some(row) = this.rows.values().find(|s| s.target.trim().is_empty()) {
                            this.select_segment(row.id);
                        }
                        cx.notify();
                    });
                })
            })
            .child(progress_bar(translated, total))
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let is_dirty = self.is_dirty();
        let status = if self.save_error {
            "Error de guardado"
        } else if self.message_error {
            "Error"
        } else if is_dirty {
            "Guardando…"
        } else {
            "Guardado"
        };
        let visible = self.save_error || self.message_error || is_dirty || self.is_busy();

        div()
            .id("status")
            .accessibility_id("application-status")
            .role(Role::Status)
            .aria_label(status)
            .aria_value(self.message.clone())
            .a11y_synthetic_children(|tree| {
                tree.parent_node().set_read_only();
                tree.parent_node().set_live(accesskit::Live::Polite);
            })
            .h(px(if visible { 30. } else { 0. }))
            .w_full()
            .bg(Theme::bg_surface())
            .px_3()
            .flex()
            .items_center()
            .text_xs()
            .overflow_hidden()
            .when(visible, |d| {
                d.text_color(if self.save_error || self.message_error {
                    Theme::rose()
                } else {
                    Theme::text_secondary()
                })
                .child(if is_dirty && !self.save_error && !self.message_error {
                    status.into()
                } else {
                    self.message.clone()
                })
            })
    }
}
