use lumencat::{model::*, storage::ProjectStore};

#[test]
fn replacement_preserves_tags_locks_history_and_rolls_back_all_edits() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("replace.db");
    let mut store = ProjectStore::open(&path)?;
    let document = ImportedDocument {
        name: "Reemplazos".into(),
        format: DocumentFormat::Txt,
        original: Vec::new(),
        original_path: None,
        source_lang: "en".into(),
        target_lang: "es".into(),
        segments: ["café <g id=\"café\">café</g>", "café", "café café"]
            .into_iter()
            .enumerate()
            .map(|(i, target)| ImportedSegment {
                external_id: i.to_string(),
                source: "coffee".into(),
                target: target.into(),
                state: SegmentState::Confirmed,
                locked: i == 1,
            })
            .collect(),
    };
    let id = store.import_document(&document, &Cancellation::default())?;
    assert_eq!(store.progress(id)?, (3, 3));
    assert_eq!(store.replace_targets(id, "café", "té")?, 2);
    let rows = store.page(id, 0, 10, "")?;
    assert_eq!(rows[0].target, "té <g id=\"café\">té</g>");
    assert_eq!(rows[1].target, "café");
    assert_eq!(rows[2].target, "té té");
    assert_eq!(rows[0].state, SegmentState::Draft);
    assert_eq!(store.undo()?.map(|s| s.target), Some("café café".into()));
    assert_eq!(
        store.page(id, 0, 10, "")?[0].target,
        "café <g id=\"café\">café</g>"
    );
    assert_eq!(store.redo()?.map(|s| s.target), Some("té té".into()));
    assert_eq!(
        store.page(id, 0, 10, "")?[0].target,
        "té <g id=\"café\">té</g>"
    );
    let before = store.page(id, 0, 10, "")?;
    assert!(
        store
            .replace_targets(id, "té", &"x".repeat(600_000))
            .is_err()
    );
    let after = store.page(id, 0, 10, "")?;
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.target, b.target);
        assert_eq!(a.revision, b.revision);
    }
    assert_eq!(store.undo()?.map(|s| s.target), Some("café café".into()));
    assert!(store.replace_targets(id, "", "ignored").is_err());
    store.close()?;
    drop(store);
    let mut reopened = ProjectStore::open(&path)?;
    assert!(!reopened.recovered);
    assert_eq!(reopened.page(id, 0, 10, "")?[1].target, "café");
    reopened.close()?;
    Ok(())
}
