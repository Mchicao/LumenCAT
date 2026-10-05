use lumencat::{formats, model::*, storage::ProjectStore};
use std::fs;

const XML: &str = r#"<?xml version="1.0"?><xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2"><file original="sample" source-language="en" target-language="es"><header><note>Keep me</note></header><body><trans-unit id="1"><source>Hello <g id="bold" ctype="bold">world <x id="image" equiv-text="picture"/></g>! <bpt id="b1" rid="r1">&lt;i&gt;</bpt>Text<ept id="e1" rid="r1">&lt;/i&gt;</ept></source><target state="new" custom="keep">Hola <g id="bold" ctype="bold">mundo <x id="image" equiv-text="picture"/></g>! <bpt id="b1" rid="r1">&lt;i&gt;</bpt>Texto<ept id="e1" rid="r1">&lt;/i&gt;</ept></target><note>Context</note></trans-unit></body></file></xliff>"#;

#[test]
fn inline_translation_preserves_native_codes_metadata_and_original() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("inline.xlf");
    fs::write(&path, XML)?;
    let cancel = Cancellation::default();
    let mut doc = formats::import_document(&path, "en", "es", &cancel)?;
    assert_eq!(doc.segments.len(), 1);
    let source = &doc.segments[0].source;
    assert!(source.contains("world"));
    assert!(!source.contains("&lt;i&gt;"));
    let unchanged = formats::serialize_document(&doc, &[doc.segments[0].target.clone()], &cancel)?;
    assert_eq!(unchanged, XML.as_bytes());
    let target = doc.segments[0]
        .target
        .replace("mundo", "planeta")
        .replace("Texto", "Contenido & más");
    doc.segments[0].state = SegmentState::Confirmed;
    let output = dir.path().join("translated.xlf");
    formats::export_document(&doc, std::slice::from_ref(&target), &output, &cancel)?;
    let bytes = fs::read_to_string(&output)?;
    assert!(bytes.contains("<header><note>Keep me</note></header>"));
    assert!(bytes.contains("custom=\"keep\""));
    assert!(bytes.contains("<note>Context</note>"));
    assert!(bytes.contains(
        "<g id=\"bold\" ctype=\"bold\">planeta <x id=\"image\" equiv-text=\"picture\"/></g>"
    ));
    assert!(bytes.contains("<bpt id=\"b1\" rid=\"r1\">&lt;i&gt;</bpt>Contenido &amp; más<ept id=\"e1\" rid=\"r1\">&lt;/i&gt;</ept>"));
    let reopened = formats::import_document(&output, "en", "es", &cancel)?;
    assert_eq!(reopened.segments[0].target, target);
    assert_eq!(reopened.segments[0].state, SegmentState::Confirmed);
    assert_eq!(fs::read_to_string(path)?, XML);
    Ok(())
}

#[test]
fn nested_groups_markers_and_native_placeholders_are_editable() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("nested.xlf");
    fs::write(
        &path,
        r#"<xliff version="1.2"><file><body><trans-unit id="a"><source><g id="a">Outer <g id="b">inner</g></g> <mrk mtype="term">term</mrk><ph id="p">&lt;br/&gt;</ph><it id="i" pos="begin">&lt;u&gt;</it><bx id="b" rid="pair"/>after<ex id="e" rid="pair"/></source></trans-unit></body></file></xliff>"#,
    )?;
    let cancel = Cancellation::default();
    let doc = formats::import_document(&path, "en", "es", &cancel)?;
    let target = doc.segments[0]
        .source
        .replace("Outer", "Exterior")
        .replace("inner", "interior")
        .replace("term", "término")
        .replace("after", "después");
    let output = formats::serialize_document(&doc, std::slice::from_ref(&target), &cancel)?;
    fs::write(dir.path().join("out.xlf"), output)?;
    let reopened = formats::import_document(&dir.path().join("out.xlf"), "en", "es", &cancel)?;
    assert_eq!(reopened.segments[0].target, target);
    Ok(())
}

#[test]
fn broken_codes_fail_confirmation_and_export_without_mutating_project() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("inline.xlf");
    fs::write(&path, XML)?;
    let cancel = Cancellation::default();
    let doc = formats::import_document(&path, "en", "es", &cancel)?;
    let mut store = ProjectStore::open(&dir.path().join("project.lcat"))?;
    let id = store.import_document(&doc, &cancel)?;
    let segment = store.page(id, 0, 1, "")?.remove(0);
    let broken = segment.target.replace("</g>", "");
    let command = EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: broken.clone(),
        state: SegmentState::Draft,
        locked: false,
        origin: Origin::Human,
    };
    assert!(store.confirm(&command).is_err());
    assert_eq!(store.segment(segment.id)?.revision, segment.revision);
    assert!(formats::serialize_document(&doc, &[broken], &cancel).is_err());
    let command = EditCommand {
        target: segment.target.replace("mundo", "planeta"),
        ..command
    };
    let confirmation = store.confirm(&command)?;
    assert_eq!(confirmation.learning, LearningOutcome::UnsupportedCodes);
    assert_eq!(confirmation.segment.state, SegmentState::Confirmed);
    store.close()?;
    Ok(())
}
