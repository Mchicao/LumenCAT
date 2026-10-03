use lumencat::{model::*, storage::ProjectStore};
use std::path::Path;

fn document() -> ImportedDocument {
    ImportedDocument {
        name: "Ejemplo".into(),
        format: DocumentFormat::Txt,
        original: b"Hello".to_vec(),
        original_path: None,
        source_lang: "en".into(),
        target_lang: "es".into(),
        segments: vec![ImportedSegment {
            external_id: "1".into(),
            source: "Hello".into(),
            target: String::new(),
            state: SegmentState::Draft,
            locked: false,
        }],
    }
}
fn edit(segment: &Segment, target: &str, locked: bool) -> EditCommand {
    EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: target.into(),
        state: SegmentState::Draft,
        locked,
        origin: Origin::Human,
    }
}

#[test]
fn durable_undo_redo_recovery_revision_and_lock() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("project.db");
    let mut store = ProjectStore::open(&path)?;
    assert!(!store.recovered);
    assert!(ProjectStore::open(&path).is_err());
    let document_id = store.import_document(&document(), &Cancellation::default())?;
    let original = store.page(document_id, 0, 1000, "")?.remove(0);
    let translated = store.edit(&edit(&original, "Hola", false))?;
    assert!(store.edit(&edit(&original, "stale", false)).is_err());
    let locked = store.edit(&edit(&translated, "Hola", true))?;
    assert!(store.edit(&edit(&locked, "locked edit", false)).is_err());
    drop(store); // No clean marker: models a terminated worker/process.
    let mut store = ProjectStore::open(&path)?;
    assert!(store.recovered);
    let unlocked = store
        .undo()?
        .ok_or_else(|| CatError::Invalid("sin undo".into()))?;
    assert_eq!(unlocked.target, "Hola");
    assert!(!unlocked.locked);
    let empty = store
        .undo()?
        .ok_or_else(|| CatError::Invalid("sin undo".into()))?;
    assert_eq!(empty.target, "");
    assert!(empty.revision > locked.revision);
    assert_eq!(store.redo()?.map(|s| s.target), Some("Hola".into()));
    let current = store.segment(original.id)?;
    store.edit(&edit(&current, "Buenas", false))?;
    assert!(store.redo()?.is_none());
    store.close()?;
    drop(store);
    let mut store = ProjectStore::open(&path)?;
    assert!(!store.recovered);
    assert_eq!(
        store.load_document(document_id)?.segments[0].target,
        "Buenas"
    );
    store.close()?;
    Ok(())
}

#[test]
fn cancel_rolls_back_partial_memory_and_document_import() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("p.db"))?;
    let cancel = Cancellation::default();
    let units = (0..4).map(|index| {
        if index == 2 {
            cancel.cancel();
        }
        TmUnit {
            source: format!("Source {index}"),
            target: "Destino".into(),
            source_lang: "en".into(),
            target_lang: "es".into(),
            raw_xml: String::new(),
        }
    });
    assert!(matches!(
        store.insert_tm_units(units, &cancel),
        Err(CatError::Cancelled)
    ));
    assert!(store.matches("Source 0", "en", "es")?.is_empty());
    assert!(matches!(
        store.import_document(&document(), &cancel),
        Err(CatError::Cancelled)
    ));
    assert!(store.documents()?.is_empty());
    store.close()?;
    Ok(())
}

fn foreign_database(path: &Path, future: bool) -> Result<()> {
    let connection = rusqlite::Connection::open(path)?;
    if future {
        connection.execute_batch("PRAGMA application_id=1279476052; PRAGMA user_version=99;")?;
    } else {
        connection.execute_batch(
            "CREATE TABLE precious(data TEXT); INSERT INTO precious VALUES('preserve');",
        )?;
    }
    drop(connection);
    assert!(ProjectStore::open(path).is_err());
    let connection = rusqlite::Connection::open(path)?;
    if future {
        assert_eq!(
            connection.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            99
        );
    } else {
        assert_eq!(
            connection.query_row("SELECT data FROM precious", [], |r| r.get::<_, String>(0))?,
            "preserve"
        );
    }
    Ok(())
}
#[test]
fn refuses_foreign_and_future_schema_without_modification() -> Result<()> {
    let directory = tempfile::tempdir()?;
    foreign_database(&directory.path().join("foreign.db"), false)?;
    foreign_database(&directory.path().join("future.db"), true)
}

#[test]
fn unicode_memory_language_isolation_and_atomic_export() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("p.db"))?;
    let unit = TmUnit {
        source: "Café 世界 🙂".into(),
        target: "Café mundo 🙂".into(),
        source_lang: "fr".into(),
        target_lang: "es".into(),
        raw_xml: String::new(),
    };
    store.insert_tm_units([unit], &Cancellation::default())?;
    assert!(store.matches("Café 世界 🙂", "en", "es")?.is_empty());
    let exact = store.matches("Café 世界 🙂", "fr", "es")?;
    assert!(exact[0].exact);
    assert_eq!(exact[0].score, 100.0);
    let normalized = store.matches("Cafe\u{301} 世界 🙂", "fr", "es")?;
    assert!(!normalized[0].exact);
    assert_eq!(normalized[0].score, 100.0);
    assert_eq!(store.concordance("Café", "fr", "es")?.len(), 1);
    let output = directory.path().join("memory.tmx");
    assert_eq!(store.export_tm(&output, &Cancellation::default())?, 1);
    let saved = std::fs::read(&output)?;
    assert!(store.export_tm(&output, &Cancellation::default()).is_err());
    assert_eq!(saved, std::fs::read(&output)?);
    let cancelled = Cancellation::default();
    cancelled.cancel();
    let incomplete = directory.path().join("incomplete.tmx");
    assert!(store.export_tm(&incomplete, &cancelled).is_err());
    assert!(!incomplete.exists());
    let mut imported = ProjectStore::open(&directory.path().join("imported.db"))?;
    assert_eq!(
        imported.import_tmx(&output, "fr", "es", &Cancellation::default())?,
        1
    );
    assert!(imported.matches("Café 世界 🙂", "fr", "es")?[0].exact);
    imported.close()?;
    store.close()?;
    Ok(())
}

#[test]
fn grouped_edits_survive_restart_and_conflicts_roll_back() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("group.lcat");
    let mut store = ProjectStore::open(&path)?;
    let first = store.import_document(&document(), &Cancellation::default())?;
    let second = store.import_document(&document(), &Cancellation::default())?;
    let a = store.page(first, 0, 1, "")?.remove(0);
    let b = store.page(second, 0, 1, "")?.remove(0);
    store.edit_batch(
        &[edit(&a, "Hola", false), edit(&b, "Buenas", false)],
        &Cancellation::default(),
    )?;
    store.close()?;
    drop(store);
    let mut store = ProjectStore::open_existing(&path)?;
    assert_eq!(store.undo_group()?.len(), 2);
    assert_eq!(store.segment(a.id)?.target, "");
    assert_eq!(store.segment(b.id)?.target, "");
    assert_eq!(store.redo_group()?.len(), 2);
    let current = store.segment(a.id)?;
    assert!(
        store
            .edit_batch(
                &[
                    edit(&current, "fallará", false),
                    edit(&b, "obsoleto", false)
                ],
                &Cancellation::default()
            )
            .is_err()
    );
    assert_eq!(store.segment(a.id)?.target, "Hola");
    assert_eq!(store.segment(a.id)?.revision, current.revision);
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(matches!(
        store.edit_batch(&[edit(&current, "cancelado", false)], &cancel),
        Err(CatError::Cancelled)
    ));
    store.undo_group()?;
    store.edit(&edit(&store.segment(a.id)?, "Nueva rama", false))?;
    assert!(store.redo_group()?.is_empty());
    assert_eq!(store.segment(b.id)?.target, "");
    store.close()?;
    Ok(())
}

#[test]
fn backup_includes_wal_and_recovery_never_overwrites() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("original.lcat");
    let backup = directory.path().join("backup.lcat");
    let recovered = directory.path().join("recovered.lcat");
    let mut store = ProjectStore::open(&path)?;
    let id = store.import_document(&document(), &Cancellation::default())?;
    let segment = store.page(id, 0, 1, "")?.remove(0);
    store.edit(&edit(&segment, "最新 café 🙂", false))?;
    assert!(path.with_extension("lcat-wal").exists());
    store.backup_to(&backup, &Cancellation::default())?;
    assert!(store.backup_to(&backup, &Cancellation::default()).is_err());
    let copy = rusqlite::Connection::open(&backup)?;
    assert_eq!(
        copy.query_row("SELECT target FROM segments", [], |r| r.get::<_, String>(0))?,
        "最新 café 🙂"
    );
    drop(copy);
    ProjectStore::recover_copy(&backup, &recovered, &Cancellation::default())?;
    assert!(ProjectStore::recover_copy(&backup, &path, &Cancellation::default()).is_err());
    let mut restored = ProjectStore::open_existing(&recovered)?;
    assert_eq!(restored.segment(segment.id)?.target, "最新 café 🙂");
    assert_eq!(restored.undo()?.map(|s| s.target), Some(String::new()));
    restored.close()?;
    assert_eq!(store.segment(segment.id)?.target, "最新 café 🙂");
    store.close()?;
    let missing = directory.path().join("missing.lcat");
    assert!(ProjectStore::open_existing(&missing).is_err());
    assert!(!missing.exists());
    Ok(())
}
