use lumencat::{formats, model::*, storage::ProjectStore};
use std::{
    io::Write,
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

pub enum Task {
    Open(PathBuf),
    Import(PathBuf, String, String, Cancellation),
    Page(i64, usize, String),
    Search(i64, usize, String, Cancellation),
    Select(i64),
    AtOrdinal(i64, usize),
    Edit(EditCommand),
    Undo,
    Redo,
    Matches(String, String, String, Cancellation),
    Concordance(String, String, String, Cancellation),
    ImportTm(PathBuf, String, String, Cancellation),
    ExportTm(PathBuf, Cancellation),
    Export(i64, PathBuf, Cancellation),
    Close,
    Qa(String, String, bool),
    RecoverText(PathBuf, String, Cancellation),
}
pub enum Data {
    Opened(bool, Vec<DocumentInfo>),
    Documents(Vec<DocumentInfo>),
    Page(Vec<Segment>),
    Selected(Segment),
    Saved(Segment),
    History(Option<Segment>),
    Matches(Vec<TmMatch>),
    Done(String),
    Closed,
    Qa(Vec<QaIssue>),
}
pub struct Request {
    pub id: u64,
    pub task: Task,
}
pub struct Reply {
    pub id: u64,
    pub result: Result<Data>,
}
pub struct Worker {
    pub sender: SyncSender<Request>,
    pub receiver: Receiver<Reply>,
}

impl Worker {
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (sender, input) = mpsc::sync_channel::<Request>(16);
        let (output, receiver) = mpsc::sync_channel(16);
        thread::spawn(move || {
            let mut store = None;
            while let Ok(request) = input.recv() {
                let started = std::time::Instant::now();
                let result = run(&mut store, request.task);
                if result.is_err() {
                    tracing::warn!(event = "worker_operation_failed", request_id = request.id);
                }
                tracing::debug!(
                    event = "worker_operation_completed",
                    request_id = request.id,
                    elapsed_us = started.elapsed().as_micros() as u64
                );
                if output
                    .send(Reply {
                        id: request.id,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { sender, receiver }
    }
}

fn run(store: &mut Option<ProjectStore>, task: Task) -> Result<Data> {
    if let Task::Open(path) = task {
        let opened = ProjectStore::open(&path)?;
        let recovered = opened.recovered;
        let documents = opened.documents()?;
        *store = Some(opened);
        return Ok(Data::Opened(recovered, documents));
    }
    let db = store
        .as_mut()
        .ok_or_else(|| CatError::Invalid("Abre un proyecto primero".into()))?;
    match task {
        Task::Open(_) => Err(CatError::Invalid("Solicitud interna inválida".into())),
        Task::Import(path, source, target, cancel) => {
            let document = formats::import_document(&path, &source, &target, &cancel)?;
            db.import_document(&document, &cancel)?;
            Ok(Data::Documents(db.documents()?))
        }
        Task::Page(document, start, query) => {
            Ok(Data::Page(db.page(document, start, 128, &query)?))
        }
        Task::Search(document, start, query, cancel) => Ok(Data::Page(
            db.search_page(document, start, 128, &query, &cancel)?,
        )),
        Task::Select(id) => Ok(Data::Selected(db.segment(id)?)),
        Task::AtOrdinal(document, ordinal) => db
            .page(document, ordinal, 1, "")?
            .into_iter()
            .next()
            .map(Data::Selected)
            .ok_or_else(|| CatError::Invalid("No existe ese segmento".into())),
        Task::Edit(command) => Ok(Data::Saved(db.edit(&command)?)),
        Task::Undo => Ok(Data::History(db.undo()?)),
        Task::Redo => Ok(Data::History(db.redo()?)),
        Task::Matches(source, sl, tl, cancel) => Ok(Data::Matches(
            db.matches_cancel(&source, &sl, &tl, &cancel)?,
        )),
        Task::Concordance(query, sl, tl, cancel) => Ok(Data::Matches(
            db.concordance_cancel(&query, &sl, &tl, &cancel)?,
        )),
        Task::ImportTm(path, sl, tl, cancel) => {
            let count = db.import_tmx(&path, &sl, &tl, &cancel)?;
            Ok(Data::Done(format!("TM importada: {count} unidades")))
        }
        Task::ExportTm(path, cancel) => {
            db.export_tm(&path, &cancel)?;
            Ok(Data::Done("Memoria exportada".into()))
        }
        Task::Export(id, path, cancel) => {
            let document = db.load_document(id)?;
            let targets: Vec<_> = document.segments.iter().map(|s| s.target.clone()).collect();
            formats::export_document(&document, &targets, &path, &cancel)?;
            Ok(Data::Done(
                "Documento exportado sin sobrescribir el original".into(),
            ))
        }
        Task::Close => {
            db.close()?;
            Ok(Data::Closed)
        }
        Task::Qa(source, target, confirmed) => {
            Ok(Data::Qa(lumencat::qa::check(&source, &target, confirmed)))
        }
        Task::RecoverText(path, text, cancel) => {
            cancel.check()?;
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."));
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            for chunk in text.as_bytes().chunks(16 * 1024) {
                cancel.check()?;
                temporary.write_all(chunk)?;
            }
            temporary.flush()?;
            temporary.as_file().sync_all()?;
            cancel.check()?;
            temporary
                .persist_noclobber(&path)
                .map_err(|e| CatError::Io(e.error))?;
            Ok(Data::Done("Borrador completo recuperado en un archivo nuevo. El proyecto aún conserva cambios pendientes.".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_preserves_oversize_unicode_draft_and_refuses_overwrite() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Some(ProjectStore::open(&directory.path().join("project.db"))?);
        let path = directory.path().join("recovery.txt");
        let text = "حفظ 👩🏽‍💻 漢字 e\u{301}\n".repeat(100_000);
        assert!(text.len() > 1_048_576);
        run(
            &mut store,
            Task::RecoverText(path.clone(), text.clone(), Cancellation::default()),
        )?;
        assert_eq!(std::fs::read_to_string(&path)?, text);
        assert!(
            run(
                &mut store,
                Task::RecoverText(path.clone(), "overwrite".into(), Cancellation::default())
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path)?, text);
        let cancelled_path = directory.path().join("cancelled.txt");
        let cancel = Cancellation::default();
        cancel.cancel();
        assert!(matches!(
            run(
                &mut store,
                Task::RecoverText(cancelled_path.clone(), text, cancel)
            ),
            Err(CatError::Cancelled)
        ));
        assert!(!cancelled_path.exists());
        run(&mut store, Task::Close)?;
        Ok(())
    }
}
