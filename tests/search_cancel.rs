use lumencat::{formats, model::*, storage::ProjectStore};
use std::time::Duration;

#[test]
fn cancellation_interrupts_a_large_substring_scan_without_poisoning_connection() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("large.txt");
    std::fs::write(&input, format!("{}\n", "a".repeat(1024)).repeat(50_000))?;
    let token = Cancellation::default();
    let document = formats::import_document(&input, "en", "es", &token)?;
    let mut store = ProjectStore::open(&directory.path().join("search.db"))?;
    let doc = store.import_document(&document, &token)?;
    let cancel = Cancellation::default();
    let request = cancel.clone();
    let result = std::thread::scope(|scope| {
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(2));
            request.cancel();
        });
        store.search_page(doc, 0, 128, "not-present", &cancel)
    });
    assert!(matches!(result, Err(CatError::Cancelled)));
    let segment = store.page(doc, 0, 1, "")?.remove(0);
    let saved = store.edit(&EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: "recuperado".into(),
        state: SegmentState::Draft,
        locked: false,
        origin: Origin::Human,
    })?;
    assert_eq!(saved.target, "recuperado");
    store.close()?;
    Ok(())
}
