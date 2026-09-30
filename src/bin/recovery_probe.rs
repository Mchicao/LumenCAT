//! Proceso auxiliar de verificación: no forma parte del flujo de aplicación.
use lumencat::{formats, model::*, storage::ProjectStore};
use std::path::PathBuf;

fn main() -> Result<()> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| CatError::Invalid("probe requiere directorio".into()))?;
    let txt = directory.join("original.txt");
    std::fs::write(&txt, "Hello 42.\r\n")?;
    let mut store = ProjectStore::open(&directory.join("project.db"))?;
    let doc = formats::import_document(&txt, "en", "es", &Cancellation::default())?;
    let id = store.import_document(&doc, &Cancellation::default())?;
    let segment = store.page(id, 0, 1, "")?.remove(0);
    store.edit(&EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: "Hola 42.".into(),
        state: SegmentState::Confirmed,
        locked: false,
        origin: Origin::Human,
    })?;
    // Dejamos una escritura NO confirmada en WAL; el padre mata este proceso.
    let unfinished = rusqlite::Connection::open(directory.join("project.db"))?;
    unfinished.execute_batch("BEGIN IMMEDIATE; UPDATE segments SET target='UNCOMMITTED';")?;
    std::fs::write(directory.join("ready"), b"committed + transaction pending")?;
    loop {
        std::thread::park_timeout(std::time::Duration::from_secs(60));
    }
}
