use crate::worker::{Data, Request, Task, Worker};
use eframe::egui::{self, Color32, Key, RichText};
use lumencat::model::*;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    time::{Duration, Instant},
};

enum Pending {
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
    Reload(u64),
}
enum Navigation {
    Segment(i64),
    Ordinal(i64, usize),
}
struct Draft {
    segment: Segment,
    serial: u64,
    saved: u64,
    changed: Instant,
    first_dirty: Instant,
    saving: bool,
}
impl Draft {
    // An acknowledgement advances persistence, never replaces newer editor text.
    fn acknowledge(&mut self, segment: &Segment, serial: u64) {
        if segment.id == self.segment.id {
            self.segment.revision = segment.revision;
            self.saved = serial;
            self.saving = false;
        }
    }
}
pub struct CatApp {
    worker: Worker,
    next_id: u64,
    pending: HashMap<u64, Pending>,
    generation: u64,
    selection_generation: u64,
    project_path: String,
    file_path: String,
    export_path: String,
    source_lang: String,
    target_lang: String,
    opened: bool,
    documents: Vec<DocumentInfo>,
    document: Option<i64>,
    rows: BTreeMap<usize, Segment>,
    requested_pages: Vec<usize>,
    active: Option<Draft>,
    navigate: Option<Navigation>,
    matches: Vec<TmMatch>,
    qa: Vec<QaIssue>,
    query: String,
    search_rows: Vec<Segment>,
    message: String,
    cancellations: HashMap<u64, Cancellation>,
    latest_matches: u64,
    closing: bool,
    closed: bool,
    save_error: bool,
    recovery_dialog: bool,
    recovery_path: String,
    discard_confirmed: bool,
}
impl CatApp {
    pub fn new(cc: &eframe::CreationContext<'_>, project: Option<String>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.selection.bg_fill = Color32::from_rgb(183, 222, 215);
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        cc.egui_ctx.set_style(style);
        let auto_open = project.is_some();
        let mut app = Self {
            worker: Worker::start(cc.egui_ctx.clone()),
            next_id: 0,
            pending: HashMap::new(),
            generation: 0,
            selection_generation: 0,
            project_path: project.unwrap_or_default(),
            file_path: String::new(),
            export_path: String::new(),
            source_lang: "en".into(),
            target_lang: "es".into(),
            opened: false,
            documents: Vec::new(),
            document: None,
            rows: BTreeMap::new(),
            requested_pages: Vec::new(),
            active: None,
            navigate: None,
            matches: Vec::new(),
            qa: Vec::new(),
            query: String::new(),
            search_rows: Vec::new(),
            message: "Abre o crea un proyecto local para empezar. No requiere cuenta ni conexión."
                .into(),
            cancellations: HashMap::new(),
            latest_matches: 0,
            closing: false,
            closed: false,
            save_error: false,
            recovery_dialog: false,
            recovery_path: String::new(),
            discard_confirmed: false,
        };
        if auto_open {
            app.send(Task::Open(PathBuf::from(&app.project_path)), Pending::Open);
        }
        app
    }
    fn send(&mut self, task: Task, pending: Pending) -> bool {
        self.next_id += 1;
        match self.worker.sender.try_send(Request {
            id: self.next_id,
            task,
        }) {
            Ok(()) => {
                self.pending.insert(self.next_id, pending);
                true
            }
            Err(_) => {
                self.message = "El trabajador está ocupado. El texto pendiente se conserva; se reintentará el guardado.".into();
                false
            }
        }
    }
    fn busy(&self) -> bool {
        self.pending.values().any(|p| {
            matches!(
                p,
                Pending::Operation
                    | Pending::Open
                    | Pending::History
                    | Pending::Close
                    | Pending::Select(_)
                    | Pending::Reload(_)
                    | Pending::Search(_)
            )
        })
    }
    fn dirty(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.serial != a.saved || a.saving)
    }
    fn invalidate(&mut self) {
        self.cancel_lookups();
        self.generation += 1;
        self.rows.clear();
        self.requested_pages.clear();
        self.search_rows.clear();
        self.selection_generation += 1;
    }
    fn select(&mut self, id: i64) {
        self.navigate_to(Navigation::Segment(id));
    }
    fn navigate_to(&mut self, navigation: Navigation) {
        self.cancel_lookups();
        if self.dirty() {
            self.navigate = Some(navigation);
            self.save(true);
        } else {
            self.selection_generation += 1;
            self.matches.clear();
            self.qa.clear();
            let task = match navigation {
                Navigation::Segment(id) => Task::Select(id),
                Navigation::Ordinal(doc, ordinal) => Task::AtOrdinal(doc, ordinal),
            };
            self.send(task, Pending::Select(self.selection_generation));
        }
    }
    fn changed(&mut self, origin: Origin) {
        if let Some(active) = &mut self.active {
            if active.serial == active.saved {
                active.first_dirty = Instant::now();
            }
            active.serial += 1;
            active.changed = Instant::now();
            active.segment.origin = origin;
            active.segment.state = SegmentState::Draft;
        }
    }
    fn save(&mut self, force: bool) {
        if self.save_error {
            return;
        }
        let Some(active) = &self.active else { return };
        if active.segment.target.len() > 1_048_576 {
            self.save_error = true;
            self.message="El target supera el límite de 1 MiB por segmento del MVP. El texto completo permanece en el editor; redúcelo o recupera un borrador en un archivo nuevo.".into();
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
        let command = EditCommand {
            segment_id: active.segment.id,
            expected_revision: active.segment.revision,
            target: active.segment.target.clone(),
            state: active.segment.state,
            locked: active.segment.locked,
            origin: active.segment.origin,
        };
        if self.send(Task::Edit(command), Pending::Save(serial))
            && let Some(active) = &mut self.active
        {
            active.saving = true;
        }
    }
    fn poll(&mut self) {
        while let Ok(reply) = self.worker.receiver.try_recv() {
            let Some(pending) = self.pending.remove(&reply.id) else {
                continue;
            };
            self.cancellations.remove(&reply.id);
            if matches!(pending,Pending::Matches(g) if g!=self.selection_generation || reply.id!=self.latest_matches)
                || matches!(pending,Pending::Search(g) if g!=self.generation)
            {
                continue;
            }
            if let Pending::Page(g, start) = pending
                && g == self.generation
            {
                self.requested_pages.retain(|p| *p != start);
            }
            match reply.result {
                Err(error) => {
                    if matches!(pending, Pending::Save(_)) {
                        self.save_error = true;
                        if let Some(a) = &mut self.active {
                            a.saving = false;
                        }
                    }
                    if matches!(pending, Pending::Close) {
                        self.closing = false;
                    }
                    self.message = format!(
                        "{error}. Si hay cambios pendientes, permanecen en el editor. Corrige el problema antes de cerrar."
                    );
                }
                Ok(data) => match data {
                    Data::Opened(recovered, docs) => {
                        self.opened = true;
                        self.documents = docs;
                        self.message = if recovered { "Se recuperó un cierre no limpio. SQLite validó la base; revisa el último segmento." } else { "Proyecto local abierto. Los cambios se guardan de forma transaccional." }.into();
                    }
                    Data::Documents(docs) => {
                        self.documents = docs;
                        self.message = "Documentos actualizados".into();
                    }
                    Data::Page(rows) => {
                        if matches!(pending, Pending::Search(g) if g == self.generation) {
                            self.search_rows = rows;
                        } else if matches!(pending, Pending::Page(g, _) if g == self.generation) {
                            for row in rows {
                                self.rows.insert(row.ordinal, row);
                            }
                            while self.rows.len() > 1024 {
                                // Keep pages near the most recently requested viewport.
                                let anchor = match pending {
                                    Pending::Page(_, start) => start,
                                    _ => 0,
                                };
                                let first = self
                                    .rows
                                    .first_key_value()
                                    .map(|(key, _)| *key)
                                    .unwrap_or(0);
                                let last =
                                    self.rows.last_key_value().map(|(key, _)| *key).unwrap_or(0);
                                if anchor.abs_diff(first) > anchor.abs_diff(last) {
                                    self.rows.pop_first();
                                } else {
                                    self.rows.pop_last();
                                }
                            }
                        }
                    }
                    Data::Selected(segment) => {
                        if matches!(pending, Pending::Select(g) | Pending::Reload(g) if g == self.selection_generation)
                        {
                            if matches!(pending, Pending::Reload(_)) {
                                self.save_error = false;
                                self.recovery_dialog = false;
                                self.navigate = None;
                                self.closing = false;
                            }
                            let source = segment.source.clone();
                            self.active = Some(Draft {
                                segment,
                                serial: 0,
                                saved: 0,
                                changed: Instant::now(),
                                first_dirty: Instant::now(),
                                saving: false,
                            });
                            self.matches.clear();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                            self.request_qa();
                        }
                    }
                    Data::Saved(segment) => {
                        if let (Pending::Save(serial), Some(active)) = (pending, &mut self.active)
                            && segment.id == active.segment.id
                        {
                            active.acknowledge(&segment, serial);
                            self.rows.insert(segment.ordinal, segment);
                            self.message = "Cambios guardados en disco".into();
                        }
                        self.request_qa();
                    }
                    Data::History(segment) => {
                        self.invalidate();
                        if let Some(segment) = segment {
                            self.document = Some(segment.document_id);
                            self.query.clear();
                            self.active = Some(Draft {
                                segment,
                                serial: 0,
                                saved: 0,
                                changed: Instant::now(),
                                first_dirty: Instant::now(),
                                saving: false,
                            });
                        }
                        self.message = "Historial aplicado y guardado".into();
                        if let Some(a) = &self.active {
                            let source = a.segment.source.clone();
                            let (sl, tl) = self.document_languages();
                            self.lookup(source, sl, tl, false);
                        }
                        self.request_qa();
                    }
                    Data::Matches(matches) => {
                        if reply.id == self.latest_matches
                            && matches!(pending, Pending::Matches(g) if g == self.selection_generation)
                        {
                            self.matches = matches;
                        }
                    }
                    Data::Done(message) => {
                        self.message = message;
                    }
                    Data::Closed => self.closed = true,
                    Data::Qa(issues) => {
                        if matches!(pending, Pending::Qa(g,s) if g == self.selection_generation && self.active.as_ref().is_some_and(|a|a.serial==s))
                        {
                            self.qa = issues;
                        }
                    }
                },
            }
        }
        if !self.dirty()
            && let Some(id) = self.navigate.take()
        {
            self.navigate_to(id);
        }
    }
    fn operation(&mut self, task: impl FnOnce(Cancellation) -> Task) {
        self.cancel_lookups();
        let cancel = Cancellation::default();
        if self.send(task(cancel.clone()), Pending::Operation) {
            self.cancellations.insert(self.next_id, cancel);
            self.message = "Operación en curso… puedes cancelarla".into();
        }
    }
    fn search(&mut self, document: i64, start: usize) {
        self.cancel_lookups();
        let cancel = Cancellation::default();
        if self.send(
            Task::Search(document, start, self.query.clone(), cancel.clone()),
            Pending::Search(self.generation),
        ) {
            self.cancellations.insert(self.next_id, cancel);
            self.message = "Búsqueda en curso… puedes cancelarla".into();
        }
    }
    fn cancel_lookups(&self) {
        for (id, pending) in &self.pending {
            if matches!(pending, Pending::Matches(_))
                && let Some(cancel) = self.cancellations.get(id)
            {
                cancel.cancel();
            }
        }
    }
    fn lookup(&mut self, query: String, sl: String, tl: String, concordance: bool) {
        self.cancel_lookups();
        self.matches.clear();
        let cancel = Cancellation::default();
        let task = if concordance {
            Task::Concordance(query, sl, tl, cancel.clone())
        } else {
            Task::Matches(query, sl, tl, cancel.clone())
        };
        if self.send(task, Pending::Matches(self.selection_generation)) {
            self.cancellations.insert(self.next_id, cancel);
            self.latest_matches = self.next_id;
        }
    }
    fn request_qa(&mut self) {
        if let Some(a) = &self.active {
            let task = Task::Qa(
                a.segment.source.clone(),
                a.segment.target.clone(),
                a.segment.state == SegmentState::Confirmed,
            );
            let serial = a.serial;
            self.send(task, Pending::Qa(self.selection_generation, serial));
        }
    }
    fn document_languages(&self) -> (String, String) {
        self.documents
            .iter()
            .find(|d| Some(d.id) == self.document)
            .map(|d| (d.source_lang.clone(), d.target_lang.clone()))
            .unwrap_or_else(|| (self.source_lang.clone(), self.target_lang.clone()))
    }
    fn move_segment(&mut self, delta: isize) {
        let Some(active) = &self.active else { return };
        let ordinal = active.segment.ordinal.saturating_add_signed(delta);
        let count = self
            .documents
            .iter()
            .find(|d| d.id == active.segment.document_id)
            .map_or(0, |d| d.segment_count);
        if ordinal >= count {
            return;
        }
        if let Some(row) = self.rows.get(&ordinal) {
            self.select(row.id);
        } else {
            self.navigate_to(Navigation::Ordinal(active.segment.document_id, ordinal));
        }
    }
    fn history(&mut self, redo: bool) {
        if self.dirty() {
            self.save(true);
            self.message = "Espera a que termine el guardado y vuelve a deshacer/rehacer.".into();
        } else if !self.busy() {
            self.send(if redo { Task::Redo } else { Task::Undo }, Pending::History);
        }
    }
}

impl eframe::App for CatApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll();
        self.save(self.closing);
        if ctx.input(|i| i.viewport().close_requested()) && !self.closed {
            self.closing = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.closed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.closing && !self.dirty() && self.pending.is_empty() {
            if self.opened {
                self.send(Task::Close, Pending::Close);
            } else {
                self.closed = true;
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, Key::Z)) {
            self.history(false);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, Key::Y)) {
            self.history(true);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, Key::ArrowDown)) {
            self.move_segment(1);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, Key::ArrowUp)) {
            self.move_segment(-1);
        }
        let confirm = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, Key::Enter));
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("LumenCAT").color(Color32::from_rgb(22, 102, 92)));
                ui.separator();
                ui.label("Proyecto local");
                ui.add_enabled(
                    !self.opened,
                    egui::TextEdit::singleline(&mut self.project_path)
                        .desired_width(350.0)
                        .hint_text("C:\\…\\proyecto.lcat"),
                );
                if ui
                    .add_enabled(
                        !self.opened && !self.busy() && !self.project_path.trim().is_empty(),
                        egui::Button::new("Abrir / crear"),
                    )
                    .clicked()
                {
                    self.send(Task::Open(PathBuf::from(&self.project_path)), Pending::Open);
                }
                ui.label("Sin nube · IA desactivada");
            });
            ui.horizontal(|ui| {
                ui.label("Origen");
                ui.add(egui::TextEdit::singleline(&mut self.source_lang).desired_width(45.0));
                ui.label("Destino");
                ui.add(egui::TextEdit::singleline(&mut self.target_lang).desired_width(45.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.file_path)
                        .desired_width(380.0)
                        .hint_text("Ruta de TXT / XLIFF 1.2 / DOCX / TMX"),
                );
                let ready = self.opened
                    && !self.busy()
                    && !self.dirty()
                    && !self.file_path.trim().is_empty();
                if ui
                    .add_enabled(ready, egui::Button::new("Importar documento"))
                    .clicked()
                {
                    let (p, s, t) = (
                        PathBuf::from(&self.file_path),
                        self.source_lang.clone(),
                        self.target_lang.clone(),
                    );
                    self.operation(|c| Task::Import(p, s, t, c));
                }
                if ui
                    .add_enabled(ready, egui::Button::new("Importar TMX"))
                    .clicked()
                {
                    let (p, s, t) = (
                        PathBuf::from(&self.file_path),
                        self.source_lang.clone(),
                        self.target_lang.clone(),
                    );
                    self.operation(|c| Task::ImportTm(p, s, t, c));
                }
                if !self.cancellations.is_empty() && ui.button("Cancelar operación").clicked() {
                    for cancel in self.cancellations.values() {
                        cancel.cancel();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.export_path)
                        .desired_width(380.0)
                        .hint_text("Ruta nueva para exportar (nunca el original)"),
                );
                let ready = self.opened
                    && !self.busy()
                    && !self.dirty()
                    && !self.export_path.trim().is_empty();
                if ui
                    .add_enabled(
                        ready && self.document.is_some(),
                        egui::Button::new("Exportar documento"),
                    )
                    .clicked()
                    && let Some(id) = self.document
                {
                    let path = PathBuf::from(&self.export_path);
                    self.operation(|c| Task::Export(id, path, c));
                }
                if ui
                    .add_enabled(ready, egui::Button::new("Exportar TMX"))
                    .clicked()
                {
                    let path = PathBuf::from(&self.export_path);
                    self.operation(|c| Task::ExportTm(path, c));
                }
                if ui.button("Deshacer · Ctrl+Z").clicked() {
                    self.history(false);
                }
                if ui.button("Rehacer · Ctrl+Y").clicked() {
                    self.history(true);
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong(if self.dirty() {
                    "● Guardado pendiente"
                } else {
                    "✓ Sin cambios pendientes"
                });
                ui.separator();
                ui.label(&self.message);
            });
            if self.closing {
                ui.horizontal(|ui| {
                    ui.label("Esperando guardado seguro para cerrar…");
                    if ui.button("Seguir trabajando").clicked() {
                        self.closing = false;
                    }
                });
            }
            if self.save_error && ui.button("Reintentar guardado pendiente").clicked() {
                self.save_error = false;
                self.save(true);
            }
            if self.save_error
                && ui
                    .button("Recuperar borrador / recargar desde disco…")
                    .clicked()
            {
                self.recovery_dialog = true;
                self.discard_confirmed = false;
                self.closing = false;
            }
        });
        egui::SidePanel::left("documents")
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.heading("Documentos");
                ui.separator();
                let mut selected = None;
                for doc in &self.documents {
                    if ui
                        .add_enabled(
                            !self.dirty() && !self.busy(),
                            egui::Button::new(format!(
                                "{}\n{} segmentos",
                                doc.name, doc.segment_count
                            ))
                            .selected(self.document == Some(doc.id)),
                        )
                        .clicked()
                    {
                        selected = Some(doc.id);
                    }
                }
                if let Some(id) = selected {
                    if let Some(doc) = self.documents.iter().find(|d| d.id == id) {
                        self.source_lang = doc.source_lang.clone();
                        self.target_lang = doc.target_lang.clone();
                    }
                    self.document = Some(id);
                    self.active = None;
                    self.matches.clear();
                    self.invalidate();
                }
                ui.separator();
                ui.label("Ctrl+Enter  Confirmar\nAlt+↑ / ↓  Navegar\nCtrl+Z / Y  Historial");
            });
        egui::SidePanel::right("memory")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Memoria de traducción");
                ui.label("Referencias · inserción siempre explícita");
                ui.separator();
                let mut insert = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, tm) in self.matches.iter().enumerate() {
                        ui.strong(format!(
                            "{:.0}%{}",
                            tm.score,
                            if tm.exact { " · exacto" } else { " · fuzzy" }
                        ));
                        ui.label(preview(&tm.source));
                        ui.label(
                            RichText::new(preview(&tm.target))
                                .color(Color32::from_rgb(22, 102, 92)),
                        );
                        if ui
                            .add_enabled(
                                self.active.as_ref().is_some_and(|a| !a.segment.locked)
                                    && !self.busy()
                                    && !self.closing,
                                egui::Button::new("Insertar referencia TM"),
                            )
                            .clicked()
                        {
                            insert = Some(index);
                        }
                        ui.separator();
                    }
                });
                if let Some(index) = insert {
                    let target = self.matches[index].target.clone();
                    if let Some(a) = &mut self.active {
                        a.segment.target = target;
                    }
                    self.changed(Origin::TranslationMemory);
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Editor de traducción");
                let changed = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Buscar source / target").desired_width(240.0)).changed();
                if changed { for(id,pending)in &self.pending {if matches!(pending,Pending::Search(_)) && let Some(cancel)=self.cancellations.get(id){cancel.cancel();}} self.invalidate(); }
                if ui.add_enabled(self.document.is_some() && !self.busy(),egui::Button::new("Buscar")).clicked() && let Some(doc)=self.document { self.invalidate(); self.search(doc,0); }
                if ui.add_enabled(!self.search_rows.is_empty() && !self.busy(),egui::Button::new("Siguientes resultados")).clicked() && let (Some(doc),Some(last))=(self.document,self.search_rows.last()) { self.search(doc,last.ordinal+1); }
                if ui.add_enabled(self.opened && !self.busy(), egui::Button::new("Concordancia")).clicked() { let(sl,tl)=self.document_languages(); self.lookup(self.query.clone(), sl, tl,true); }
            });
            let count = self.documents.iter().find(|d| Some(d.id) == self.document).map_or(0, |d| d.segment_count);
            let mut select = None; let mut pages = Vec::new();
            if !self.query.is_empty() {
                ui.label("Resultados de búsqueda · máximo 128 por página · usa Buscar / Siguientes resultados");
                egui::ScrollArea::vertical().id_salt("search_results").max_height((ui.available_height()*0.42).max(120.0)).show_rows(ui,28.0,self.search_rows.len(),|ui,range| {
                    for index in range { let row=&self.search_rows[index]; if ui.selectable_label(self.active.as_ref().is_some_and(|a|a.segment.id==row.id),format!("{}  {} → {}",row.ordinal+1,preview(&row.source),preview(&row.target))).clicked(){select=Some(row.id);} }
                });
            } else { egui::ScrollArea::vertical().id_salt("segment_list").max_height((ui.available_height() * 0.42).max(120.0)).show_rows(ui, 28.0, count, |ui, range| {
                for ordinal in range {
                    if let Some(row) = self.rows.get(&ordinal) {
                        let label = format!("{:>5}  {}  {}  →  {}", ordinal + 1, if row.locked { "LOCK" } else if row.state == SegmentState::Confirmed { "OK" } else { "·" }, preview(&row.source), preview(&row.target));
                        if ui.selectable_label(self.active.as_ref().is_some_and(|a| a.segment.id == row.id), label).clicked() { select = Some(row.id); }
                    } else { ui.label(format!("{:>5}  …", ordinal + 1)); let page = ordinal / 128 * 128; if !pages.contains(&page) { pages.push(page); } }
                }
            }); }
            for page in pages { if !self.requested_pages.contains(&page) && let Some(doc) = self.document && self.send(Task::Page(doc,page,String::new()), Pending::Page(self.generation,page)) { self.requested_pages.push(page); } }
            if let Some(id) = select { self.select(id); }
            ui.separator();
            let mut changed = false; let mut copied = false; let mut locked = false; let mut confirmed = confirm;
            let editor_enabled=!self.busy() && !self.closing;
            if let Some(active) = &mut self.active {
                ui.horizontal(|ui| { ui.strong(format!("Segmento {}", active.segment.ordinal + 1)); ui.label(format!("Estado: {} · Origen: {}", active.segment.state.as_str(), active.segment.origin.as_str())); });
                ui.label("SOURCE · solo lectura");
                egui::ScrollArea::vertical().id_salt("source").max_height(90.0).show(ui, |ui| { ui.label(&active.segment.source); });
                ui.label("TARGET · traducción editable");
                egui::ScrollArea::vertical().id_salt("target").max_height(180.0).show(ui, |ui| {
                    changed = ui.add_enabled(!active.segment.locked && editor_enabled, egui::TextEdit::multiline(&mut active.segment.target).desired_width(f32::INFINITY).desired_rows(5)).changed();
                });
                ui.horizontal(|ui| {
                    copied = ui.add_enabled(!active.segment.locked && editor_enabled, egui::Button::new("Copiar source")).clicked();
                    confirmed |= ui.add_enabled(!active.segment.locked && editor_enabled, egui::Button::new("Confirmar · Ctrl+Enter")).clicked();
                    locked = ui.add_enabled(editor_enabled,egui::Checkbox::new(&mut active.segment.locked,"Bloqueado")).changed();
                });
                ui.separator(); ui.strong("Control de calidad local");
                for issue in &self.qa { ui.label(format!("{} · {}", issue.code, issue.message)); }
            } else { ui.label("Selecciona un segmento. El original se conserva y nunca se edita directamente."); }
            if copied { if let Some(a) = &mut self.active { a.segment.target = a.segment.source.clone(); } changed = true; }
            if changed { self.changed(Origin::Human); }
            if locked && let Some(a)=&mut self.active { if a.serial==a.saved {a.first_dirty=Instant::now();} a.serial+=1; a.changed=Instant::now(); self.save(true); }
            if confirmed && editor_enabled && let Some(a) = &mut self.active && !a.segment.locked { a.serial += 1; a.segment.state = SegmentState::Confirmed; a.changed = Instant::now(); self.save(true); self.move_segment(1); }
        });
        if self.recovery_dialog {
            let mut open = true;
            egui::Window::new("Recuperación del borrador pendiente").open(&mut open).collapsible(false).show(ctx,|ui| {
                if let Some(a)=&self.active {
                    ui.label(format!("Segmento {} · {} bytes de texto en el editor",a.segment.ordinal+1,a.segment.target.len()));
                    ui.label("Puedes guardar el texto completo en un TXT nuevo. El proyecto conserva sus cambios pendientes hasta guardarlos o descartarlos explícitamente.");
                    ui.label(format!("Vista del borrador: {}",preview(&a.segment.target)));
                }
                ui.add(egui::TextEdit::singleline(&mut self.recovery_path).hint_text("Ruta nueva para recuperar el borrador.txt").desired_width(450.0));
                if ui.add_enabled(!self.busy() && !self.recovery_path.trim().is_empty(),egui::Button::new("Guardar borrador completo en archivo nuevo")).clicked() && let Some(a)=&self.active {
                    let text=a.segment.target.clone();let path=PathBuf::from(&self.recovery_path);self.operation(|cancel|Task::RecoverText(path,text,cancel));
                }
                ui.separator();
                ui.label("Recargar reemplaza el borrador del editor por la última versión persistida del segmento. El texto pendiente que no hayas respaldado se perderá.");
                ui.checkbox(&mut self.discard_confirmed,"Entiendo y autorizo descartar este borrador pendiente");
                if ui.add_enabled(self.discard_confirmed && !self.busy(),egui::Button::new("Confirmar descarte y recargar desde disco")).clicked() && let Some(a)=&self.active {
                    let id=a.segment.id;self.selection_generation+=1;self.send(Task::Select(id),Pending::Reload(self.selection_generation));
                }
            });
            self.recovery_dialog = open;
        }
        if self.dirty() || !self.pending.is_empty() || self.closing {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
}
fn preview(text: &str) -> String {
    let mut chars = text.chars();
    let mut result: String = chars.by_ref().take(70).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result.replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_save_ack_preserves_newer_text_and_remains_dirty() {
        let segment = Segment {
            id: 1,
            document_id: 1,
            ordinal: 0,
            external_id: "1".into(),
            source: "source".into(),
            target: "más reciente 👩🏽‍💻".into(),
            state: SegmentState::Draft,
            locked: false,
            origin: Origin::Human,
            revision: 0,
        };
        let mut draft = Draft {
            segment: segment.clone(),
            serial: 2,
            saved: 0,
            changed: Instant::now(),
            first_dirty: Instant::now(),
            saving: true,
        };
        let mut committed = segment;
        committed.target = "texto anterior".into();
        committed.revision = 1;
        draft.acknowledge(&committed, 1);
        assert_eq!(draft.segment.target, "más reciente 👩🏽‍💻");
        assert_eq!(draft.segment.revision, 1);
        assert_ne!(draft.serial, draft.saved);
        assert!(!draft.saving);
        committed.target = draft.segment.target.clone();
        committed.revision = 2;
        draft.acknowledge(&committed, 2);
        assert_eq!(draft.serial, draft.saved);
    }
}
