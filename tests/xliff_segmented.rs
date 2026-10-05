use lumencat::{formats, model::*, storage::ProjectStore};
use std::fs;

const XML: &str = r#"<?xml version="1.0"?><xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2"><file original="sample.txt" datatype="plaintext" source-language="en" target-language="es"><header><note>Keep me</note></header><body><trans-unit id="paragraph" approved="yes"><source>Hello <g id="bold">world</g>. Goodbye.</source><seg-source><mrk mtype="seg" mid="a">Hello <g id="bold">world</g>.</mrk> <mrk mtype="seg" mid="b">Goodbye.</mrk></seg-source><target state="final" xml:space="preserve"><mrk mtype="seg" mid="b">Adiós.</mrk> <mrk mtype="seg" mid="a" comment="keep">Hola <g id="bold">mundo</g>.</mrk></target><alt-trans mid="b"><source>Alternative.</source><seg-source><mrk mtype="seg">Alternative.</mrk></seg-source><target>Alternativa.</target></alt-trans><note>Context</note></trans-unit><trans-unit id="plain"><source>Other.</source><target>Otro.</target></trans-unit></body></file></xliff>"#;

#[test]
fn segmented_units_persist_independent_edits_and_preserve_target_order() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let evidence = std::env::var_os("LUMENCAT_XLIFF_TEST_RUN_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &evidence {
        fs::create_dir(path)?;
    }
    let dir = evidence.as_deref().unwrap_or(temporary.path());
    let path = dir.join("segmented.xlf");
    fs::write(&path, XML)?;
    let cancel = Cancellation::default();
    let doc = formats::import_document(&path, "en", "es", &cancel)?;
    assert_eq!(doc.segments.len(), 3);
    assert!(doc.segments[0].source.starts_with("Hello "));
    assert_eq!(doc.segments[1].source, "Goodbye.");
    assert_eq!(doc.segments[1].target, "Adiós.");
    assert_ne!(doc.segments[0].external_id, doc.segments[1].external_id);
    let unchanged: Vec<_> = doc.segments.iter().map(|s| s.target.clone()).collect();
    assert_eq!(
        formats::serialize_document(&doc, &unchanged, &cancel)?,
        XML.as_bytes()
    );
    let project = dir.join("project.lcat");
    let mut store = ProjectStore::open(&project)?;
    let id = store.import_document(&doc, &cancel)?;
    let rows = store.page(id, 0, 10, "")?;
    for (row, target) in rows.iter().take(2).zip([
        rows[0].target.replace("mundo", "planeta & café"),
        "Hasta luego.".into(),
    ]) {
        store.confirm(&EditCommand {
            segment_id: row.id,
            expected_revision: row.revision,
            target,
            state: SegmentState::Draft,
            locked: false,
            origin: Origin::Human,
        })?;
    }
    store.close()?;
    drop(store);
    let mut store = ProjectStore::open(&project)?;
    let saved = store.load_document(id)?;
    let targets: Vec<_> = saved.segments.iter().map(|s| s.target.clone()).collect();
    let output = dir.join("translated.xlf");
    formats::export_document(&saved, &targets, &output, &cancel)?;
    let bytes = fs::read_to_string(&output)?;
    assert!(bytes.contains(r#"<mrk mtype="seg" mid="b">Hasta luego.</mrk>"#));
    assert!(bytes.contains(r#"mid="a" comment="keep">Hola <g id="bold">planeta &amp; café</g>."#));
    assert!(bytes.contains(r#"approved="no""#));
    assert!(bytes.contains("<header><note>Keep me</note></header>"));
    assert!(bytes.contains("<note>Context</note>"));
    assert!(bytes.contains("<target>Alternativa.</target>"));
    let reopened = formats::import_document(&output, "en", "es", &cancel)?;
    assert_eq!(reopened.segments.len(), 3);
    for (segment, expected) in reopened.segments.iter().zip(&targets) {
        assert_eq!(&segment.target, expected);
    }
    assert_eq!(reopened.segments[0].state, SegmentState::Confirmed);
    assert_eq!(reopened.segments[1].state, SegmentState::Confirmed);
    assert_eq!(fs::read_to_string(&path)?, XML);
    store.close()?;
    Ok(())
}

#[test]
fn missing_or_empty_target_uses_source_segmentation_without_exposing_wrappers() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let cancel = Cancellation::default();
    for (index, target) in ["", "<q:target/>", "<q:target> </q:target>"]
        .iter()
        .enumerate()
    {
        let xml = format!(
            r#"<q:xliff xmlns:q="urn:oasis:names:tc:xliff:document:1.2" version="1.2"><q:file source-language="en" target-language="es"><q:body><q:trans-unit id="a"><q:source><q:g id="g">One. Two.</q:g></q:source><q:seg-source><q:g id="g"><q:mrk mtype="seg">One.</q:mrk> <q:mrk mtype="seg">Two.</q:mrk></q:g></q:seg-source>{target}</q:trans-unit></q:body></q:file></q:xliff>"#
        );
        let path = dir.path().join(format!("source-{index}.xlf"));
        fs::write(&path, &xml)?;
        let doc = formats::import_document(&path, "en", "es", &cancel)?;
        assert_eq!(doc.segments[0].source, "One.");
        assert_eq!(doc.segments[1].source, "Two.");
        assert!(doc.segments.iter().all(|s| s.target.is_empty()));
        assert_eq!(
            formats::serialize_document(&doc, &[String::new(), String::new()], &cancel)?,
            xml.as_bytes()
        );
        let out = dir.path().join(format!("out-{index}.xlf"));
        formats::export_document(&doc, &["Uno.".into(), "Dos.".into()], &out, &cancel)?;
        let output = fs::read_to_string(&out)?;
        assert!(output.contains("<q:g id=\"g\"><q:mrk mtype=\"seg\">Uno.</q:mrk> <q:mrk mtype=\"seg\">Dos.</q:mrk></q:g>"));
        let reopened = formats::import_document(&out, "en", "es", &cancel)?;
        assert_eq!(reopened.segments[0].target, "Uno.");
        assert_eq!(reopened.segments[1].target, "Dos.");
    }
    Ok(())
}

#[test]
fn ambiguous_or_inconsistent_segmentation_is_rejected() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let cancel = Cancellation::default();
    let cases = [
        (
            r#"<mrk mtype="seg" mid="a">One.</mrk><mrk mtype="seg" mid="a">Two.</mrk>"#,
            "",
        ),
        (
            r#"<mrk mtype="seg" mid="a">One.</mrk><mrk mtype="seg" mid="b">Two.</mrk>"#,
            r#"<target><mrk mtype="seg" mid="a">Uno.</mrk><mrk mtype="seg" mid="c">Dos.</mrk></target>"#,
        ),
        (
            r#"<mrk mtype="seg" mid="a">One.</mrk><mrk mtype="seg" mid="b">Two.</mrk>"#,
            "<target>Uno. Dos.</target>",
        ),
        (
            r#"<mrk mtype="seg"><mrk mtype="seg">One.</mrk>Two.</mrk>"#,
            "",
        ),
        (r#"<mrk mtype="seg" mid="a">Different.</mrk>"#, ""),
    ];
    for (index, (segmented, target)) in cases.into_iter().enumerate() {
        let path = dir.path().join(format!("bad-{index}.xlf"));
        fs::write(
            &path,
            format!(
                r#"<xliff version="1.2"><file><body><trans-unit id="a"><source>One.Two.</source><seg-source>{segmented}</seg-source>{target}</trans-unit></body></file></xliff>"#
            ),
        )?;
        assert!(
            formats::import_document(&path, "en", "es", &cancel).is_err(),
            "case {index}"
        );
    }
    Ok(())
}

#[test]
fn damaged_segment_codes_do_not_confirm_or_create_an_export() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("segmented.xlf");
    fs::write(&path, XML)?;
    let cancel = Cancellation::default();
    let doc = formats::import_document(&path, "en", "es", &cancel)?;
    let mut store = ProjectStore::open(&dir.path().join("project.lcat"))?;
    let id = store.import_document(&doc, &cancel)?;
    let row = store.page(id, 0, 1, "")?.remove(0);
    let broken = row.target.replace("</g>", "");
    assert!(
        store
            .confirm(&EditCommand {
                segment_id: row.id,
                expected_revision: row.revision,
                target: broken.clone(),
                state: SegmentState::Draft,
                locked: false,
                origin: Origin::Human,
            })
            .is_err()
    );
    assert_eq!(store.segment(row.id)?.target, row.target);
    let destination = dir.path().join("invalid.xlf");
    let mut targets: Vec<_> = doc.segments.iter().map(|s| s.target.clone()).collect();
    targets[0] = broken;
    assert!(formats::export_document(&doc, &targets, &destination, &cancel).is_err());
    assert!(!destination.exists());
    assert_eq!(fs::read_to_string(path)?, XML);
    store.close()?;
    Ok(())
}

#[test]
fn empty_markers_and_semantically_equal_xml_keep_their_identity() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let cancel = Cancellation::default();
    let path = dir.path().join("empty.xlf");
    let xml = r#"<xliff version="1.2"><file><body><trans-unit id="a"><source><![CDATA[ One & two.]]></source><seg-source><mrk mtype="seg" mid="1"/> <mrk mtype="seg" mid="2">One &amp; two.</mrk></seg-source><target><mrk mtype="seg" mid="1"/><mrk mtype="seg" mid="2">Uno y dos.</mrk></target></trans-unit></body></file></xliff>"#;
    fs::write(&path, xml)?;
    let doc = formats::import_document(&path, "en", "es", &cancel)?;
    assert_eq!(doc.segments[0].source, "");
    assert_eq!(doc.segments[1].source, "One & two.");
    let output = dir.path().join("translated.xlf");
    formats::export_document(
        &doc,
        &["Añadido.".into(), "Uno y dos.".into()],
        &output,
        &cancel,
    )?;
    let reopened = formats::import_document(&output, "en", "es", &cancel)?;
    assert_eq!(
        reopened.segments[0].external_id,
        doc.segments[0].external_id
    );
    assert_eq!(reopened.segments[0].target, "Añadido.");
    Ok(())
}

#[test]
fn new_target_carries_namespaces_declared_on_seg_source() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let cancel = Cancellation::default();
    for (index, target) in ["", "<target/>"].iter().enumerate() {
        let xml = format!(
            r#"<xliff version="1.2"><file><body><trans-unit id="a"><source xmlns:q="urn:oasis:names:tc:xliff:document:1.2"><q:g id="g">One.</q:g></source><seg-source xmlns:q="urn:oasis:names:tc:xliff:document:1.2" xml:space="preserve"><q:mrk mtype="seg" mid="1"><q:g id="g">One.</q:g></q:mrk></seg-source>{target}</trans-unit></body></file></xliff>"#
        );
        let path = dir.path().join(format!("source-{index}.xlf"));
        fs::write(&path, xml)?;
        let doc = formats::import_document(&path, "en", "es", &cancel)?;
        let output = dir.path().join(format!("translated-{index}.xlf"));
        formats::export_document(
            &doc,
            &[doc.segments[0].source.replace("One", "Uno")],
            &output,
            &cancel,
        )?;
        let bytes = fs::read_to_string(&output)?;
        assert!(bytes.contains(r#"<target state="needs-review-translation" xmlns:q="urn:oasis:names:tc:xliff:document:1.2" xml:space="preserve">"#));
        assert!(formats::import_document(&output, "en", "es", &cancel).is_ok());
    }
    Ok(())
}

#[test]
fn mixed_review_is_conservative_and_mixed_locks_are_not_silently_expanded() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("segmented.xlf");
    fs::write(&path, XML)?;
    let cancel = Cancellation::default();
    let mut doc = formats::import_document(&path, "en", "es", &cancel)?;
    let targets: Vec<_> = doc.segments.iter().map(|s| s.target.clone()).collect();
    doc.segments[1].state = SegmentState::Draft;
    let output = dir.path().join("draft.xlf");
    formats::export_document(&doc, &targets, &output, &cancel)?;
    let reopened = formats::import_document(&output, "en", "es", &cancel)?;
    assert_eq!(reopened.segments[0].state, SegmentState::Draft);
    assert_eq!(reopened.segments[1].state, SegmentState::Draft);
    doc.segments[0].locked = true;
    let rejected = dir.path().join("mixed-locks.xlf");
    assert!(formats::export_document(&doc, &targets, &rejected, &cancel).is_err());
    assert!(!rejected.exists());
    doc.segments[1].locked = true;
    let output = dir.path().join("locked.xlf");
    formats::export_document(&doc, &targets, &output, &cancel)?;
    let reopened = formats::import_document(&output, "en", "es", &cancel)?;
    assert!(reopened.segments[0].locked);
    assert!(reopened.segments[1].locked);
    assert!(!reopened.segments[2].locked);
    Ok(())
}
