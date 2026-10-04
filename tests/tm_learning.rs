use lumencat::{model::*, storage::ProjectStore};

fn document() -> ImportedDocument {
    ImportedDocument {
        name: "Repeticiones".into(),
        format: DocumentFormat::Txt,
        original: b"Hello world.\nHello world.".to_vec(),
        original_path: None,
        source_lang: "en".into(),
        target_lang: "es".into(),
        segments: (0..2)
            .map(|i| ImportedSegment {
                external_id: i.to_string(),
                source: "Hello world.".into(),
                target: String::new(),
                state: SegmentState::Draft,
                locked: false,
            })
            .collect(),
    }
}

fn command(segment: &Segment, target: &str) -> EditCommand {
    EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: target.into(),
        state: SegmentState::Draft,
        locked: false,
        origin: Origin::Human,
    }
}

#[test]
fn confirmation_learning_correction_and_history_are_durable() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("learning.lcat");
    let mut store = ProjectStore::open(&path)?;
    let doc = store.import_document(&document(), &Cancellation::default())?;
    let rows = store.page(doc, 0, 10, "")?;
    let a = &rows[0];
    let result = store.confirm(&command(a, "Hola mundo."))?;
    assert_eq!(result.learning, LearningOutcome::Learned);
    assert_eq!(result.segment.state, SegmentState::Confirmed);
    let repeated = store.matches(&rows[1].source, "en", "es")?;
    assert_eq!(repeated.len(), 1);
    assert_eq!(repeated[0].target, "Hola mundo.");
    assert_eq!(repeated[0].learned_from, Some(a.id));
    let repeated_confirm = store.confirm(&command(&result.segment, "Hola mundo."))?;
    assert_eq!(repeated_confirm.segment.revision, result.segment.revision);
    assert_eq!(
        store.export_tm(&dir.path().join("once.tmx"), &Cancellation::default())?,
        1
    );
    let corrected = store.edit(&command(&result.segment, "Saludos mundo."))?;
    assert!(store.matches("Hello world.", "en", "es")?.is_empty());
    assert!(store.concordance("Hello", "en", "es")?.is_empty());
    store.undo()?;
    assert_eq!(
        store.matches("Hello world.", "en", "es")?[0].target,
        "Hola mundo."
    );
    store.redo()?;
    assert!(store.concordance("Hello", "en", "es")?.is_empty());
    assert!(store.confirm(&command(&corrected, "obsoleto")).is_err());
    let corrected = store.segment(a.id)?;
    store.confirm(&command(&corrected, "Saludos mundo."))?;
    store.close()?;
    drop(store);
    let mut store = ProjectStore::open_existing(&path)?;
    assert_eq!(
        store.matches("Hello world.", "EN", "ES")?[0].target,
        "Saludos mundo."
    );
    assert_eq!(
        store.export_tm(&dir.path().join("corrected.tmx"), &Cancellation::default())?,
        1
    );
    store.undo()?;
    assert_eq!(store.segment(a.id)?.state, SegmentState::Draft);
    assert!(store.concordance("Hello", "en", "es")?.is_empty());
    store.undo()?;
    assert_eq!(
        store.matches("Hello world.", "en", "es")?[0].target,
        "Hola mundo."
    );
    store.redo()?;
    store.redo()?;
    assert_eq!(
        store.matches("Hello world.", "en", "es")?[0].target,
        "Saludos mundo."
    );
    let exported = dir.path().join("roundtrip.tmx");
    store.export_tm(&exported, &Cancellation::default())?;
    let mut imported = ProjectStore::open(&dir.path().join("imported.lcat"))?;
    assert_eq!(
        imported.import_tmx(&exported, "en", "es", &Cancellation::default())?,
        1
    );
    assert_eq!(
        imported.matches("Hello world.", "en", "es")?[0].target,
        "Saludos mundo."
    );
    imported.close()?;
    store.undo()?;
    let row = store.segment(a.id)?;
    store.edit(&command(&row, "otra rama"))?;
    assert!(store.redo()?.is_none());
    assert!(store.matches("Hello world.", "en", "es")?.is_empty());
    store.close()?;
    Ok(())
}

#[test]
fn variants_imported_units_read_only_and_disabled_memories_are_preserved() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&dir.path().join("variants.lcat"))?;
    let doc = store.import_document(&document(), &Cancellation::default())?;
    let rows = store.page(doc, 0, 10, "")?;
    store.insert_tm_units(
        [TmUnit {
            source: "Hello world.".into(),
            target: "Importada".into(),
            source_lang: "en".into(),
            target_lang: "es".into(),
            raw_xml: String::new(),
        }],
        &Cancellation::default(),
    )?;
    let memory = store
        .write_memory()?
        .ok_or_else(|| CatError::Invalid("sin memoria".into()))?;
    store.confirm(&command(&rows[0], "Hola"))?;
    store.confirm(&command(&rows[1], "Buenas"))?;
    assert_eq!(store.matches("Hello world.", "en", "es")?.len(), 3);
    let first = store.segment(rows[0].id)?;
    store.edit(&command(&first, "Corregida"))?;
    let remaining = store.matches("Hello world.", "en", "es")?;
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().any(|m| m.target == "Importada"));
    assert!(remaining.iter().any(|m| m.target == "Buenas"));
    store.configure_memory(memory, false, true)?;
    assert!(store.select_write_memory(Some(memory)).is_err());
    let current = store.segment(first.id)?;
    assert_eq!(
        store.confirm(&command(&current, "Corregida"))?.learning,
        LearningOutcome::Disabled
    );
    assert_eq!(store.matches("Hello world.", "en", "es")?.len(), 2);
    store.configure_memory(memory, true, false)?;
    assert_eq!(
        store.matches("Hello world.", "en", "es")?[0].target,
        "Importada"
    );
    store.configure_memory(memory, true, true)?;
    store.select_write_memory(None)?;
    assert_eq!(
        store
            .confirm(&command(&store.segment(first.id)?, "Otra"))?
            .learning,
        LearningOutcome::Disabled
    );
    let another = store.create_memory("Cliente", "en", "es")?;
    store.select_write_memory(Some(another))?;
    assert_eq!(
        store
            .confirm(&command(&store.segment(first.id)?, "Otra"))?
            .learning,
        LearningOutcome::Learned
    );
    assert!(
        store
            .matches("Hello world.", "en", "es")?
            .iter()
            .any(|m| m.memory_name == "Cliente" && m.target == "Otra")
    );
    assert!(store.matches("Hello world.", "en", "pt-BR")?.is_empty());
    store.close()?;
    Ok(())
}

#[test]
fn learning_failure_is_atomic_and_edit_state_does_not_imply_confirmation() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("atomic.lcat");
    let mut store = ProjectStore::open(&path)?;
    let doc = store.import_document(&document(), &Cancellation::default())?;
    let row = store.page(doc, 0, 1, "")?.remove(0);
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER fail_learning BEFORE INSERT ON tm WHEN new.learned_segment_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'test failure'); END;")?;
    assert!(store.confirm(&command(&row, "Hola")).is_err());
    assert_eq!(store.segment(row.id)?.target, "");
    assert_eq!(store.segment(row.id)?.state, SegmentState::Draft);
    assert!(store.undo()?.is_none());
    assert!(store.matches("Hello world.", "en", "es")?.is_empty());
    assert!(store.confirm(&command(&row, "  ")).is_err());
    let mut edit = command(&row, "Importación de estado");
    edit.state = SegmentState::Confirmed;
    store.edit(&edit)?;
    assert!(store.matches("Hello world.", "en", "es")?.is_empty());
    store.close()?;
    Ok(())
}

#[test]
fn mass_replacement_suspends_all_contributions_and_undo_restores_them() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&dir.path().join("replacement.lcat"))?;
    let doc = store.import_document(&document(), &Cancellation::default())?;
    let rows = store.page(doc, 0, 10, "")?;
    store.confirm(&command(&rows[0], "Hola mundo"))?;
    store.confirm(&command(&rows[1], "Hola planeta"))?;
    assert_eq!(store.matches("Hello world.", "en", "es")?.len(), 2);
    assert_eq!(store.replace_targets(doc, "Hola", "Saludos")?, 2);
    assert!(store.concordance("Hello", "en", "es")?.is_empty());
    assert_eq!(store.undo_group()?.len(), 2);
    assert_eq!(store.matches("Hello world.", "en", "es")?.len(), 2);
    assert!(
        store
            .page(doc, 0, 10, "")?
            .iter()
            .all(|s| s.state == SegmentState::Confirmed)
    );
    store.redo_group()?;
    assert!(store.matches("Hello world.", "en", "es")?.is_empty());
    store.close()?;
    Ok(())
}
