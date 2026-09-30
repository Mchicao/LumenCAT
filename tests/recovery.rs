use lumencat::{formats, model::*, storage::ProjectStore};
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn killed_process_recovers_committed_work_discards_incomplete_transaction_and_exports_original_safe()
-> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_recovery_probe"))
        .arg(directory.path())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(20);
    while !directory.path().join("ready").exists() {
        if let Some(status) = child.try_wait()? {
            return Err(CatError::Invalid(format!("probe terminó {status}")));
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CatError::Invalid("timeout probe".into()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill()?;
    child.wait()?;
    let mut store = ProjectStore::open(&directory.path().join("project.db"))?;
    assert!(store.recovered);
    let docs = store.documents()?;
    let segment = store.page(docs[0].id, 0, 1, "")?.remove(0);
    assert_eq!(segment.target, "Hola 42.");
    assert_eq!(segment.state, SegmentState::Confirmed);
    assert_eq!(store.undo()?.map(|s| s.target), Some(String::new()));
    assert_eq!(store.redo()?.map(|s| s.target), Some("Hola 42.".into()));
    let doc = store.load_document(docs[0].id)?;
    let targets: Vec<_> = doc.segments.iter().map(|s| s.target.clone()).collect();
    let export = directory.path().join("translated.txt");
    formats::export_document(&doc, &targets, &export, &Cancellation::default())?;
    assert_eq!(std::fs::read(export)?, b"Hola 42.\r\n");
    assert_eq!(
        std::fs::read(directory.path().join("original.txt"))?,
        b"Hello 42.\r\n"
    );
    store.close()?;
    drop(store);
    let mut clean = ProjectStore::open(&directory.path().join("project.db"))?;
    assert!(!clean.recovered);
    clean.close()?;
    Ok(())
}
