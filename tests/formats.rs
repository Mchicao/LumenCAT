use lumencat::{formats::*, model::*};
use std::{fs, io::Cursor};

#[test]
fn txt_roundtrip_preserves_bom_mixed_endings_empty_lines_and_original() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let original = dir.path().join("original.txt");
    let bytes = "\u{feff}日本語\r\n\r\nعربي🙂e\u{301}\nlast\r";
    fs::write(&original, bytes)?;
    let cancel = Cancellation::default();
    let document = import_document(&original, "ja", "es", &cancel)?;
    assert_eq!(document.segments.len(), 4);
    let targets = document
        .segments
        .iter()
        .map(|s| s.source.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        serialize_document(&document, &targets, &cancel)?,
        bytes.as_bytes()
    );
    let output = dir.path().join("translated.txt");
    export_document(&document, &targets, &output, &cancel)?;
    assert!(export_document(&document, &targets, &original, &cancel).is_err());
    assert!(export_document(&document, &targets, &output, &cancel).is_err());
    assert_eq!(fs::read(&original)?, bytes.as_bytes());
    assert_eq!(fs::read(&output)?, bytes.as_bytes());
    let mut multiline = targets;
    multiline[0] = "a\nb".into();
    assert!(serialize_document(&document, &multiline, &cancel).is_err());
    Ok(())
}

#[test]
fn xliff_preserves_envelope_ignores_alt_trans_and_escapes_unicode_targets() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("source.xlf");
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?><x:xliff xmlns:x="urn:oasis:names:tc:xliff:document:1.2" version="1.2"><x:file original="source" source-language="en" target-language="es" datatype="plaintext"><x:header><note>customer metadata</note></x:header><x:body><x:trans-unit id="1"><x:source>A &amp; B🙂</x:source><x:target state="needs-review-translation" xml:lang="es">old</x:target><x:alt-trans><x:source>ignore me</x:source><x:target>ignore too</x:target></x:alt-trans></x:trans-unit><x:trans-unit id="2" translate="no"><x:source>日本語</x:source></x:trans-unit></x:body></x:file></x:xliff>"#;
    fs::write(&path, xml)?;
    let cancel = Cancellation::default();
    let doc = import_document(&path, "en", "es", &cancel)?;
    assert_eq!(doc.segments.len(), 2);
    assert_eq!(doc.segments[0].source, "A & B🙂");
    assert!(doc.segments[1].locked);
    let targets = vec!["<عربي> & \"e\u{301}\"\r".into(), "中文".into()];
    let serialized = serialize_document(&doc, &targets, &cancel)?;
    let output =
        String::from_utf8(serialized.clone()).map_err(|e| CatError::Format(e.to_string()))?;
    assert!(output.contains("<x:header><note>customer metadata</note></x:header>"));
    assert!(output.contains("state=\"needs-review-translation\""));
    assert!(output.contains("xml:lang=\"es\""));
    assert!(output.contains(
        "<x:alt-trans><x:source>ignore me</x:source><x:target>ignore too</x:target></x:alt-trans>"
    ));
    fs::write(&path, serialized)?;
    let reparsed = import_document(&path, "en", "es", &cancel)?;
    assert_eq!(
        reparsed
            .segments
            .iter()
            .map(|s| s.target.clone())
            .collect::<Vec<_>>(),
        targets
    );
    Ok(())
}

#[test]
fn xml_subset_rejects_constructs_that_would_lose_structure() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("bad.xlf");
    for xml in [
        r#"<!DOCTYPE xliff [<!ENTITY ext SYSTEM "file:///secret">]><xliff version="1.2"/>"#,
        r#"<xliff version="1.2"><file><body><trans-unit id="1"><source>A</source><seg-source>A</seg-source></trans-unit></body></file></xliff>"#,
        r#"<xliff version="1.2"><file><body><trans-unit id="1"><source>A</source>"#,
        r#"<?xml version="1.0" encoding="ISO-8859-1"?><xliff version="1.2"/>"#,
        r#"<xliff version="2.1"/>"#,
        r#"<xliff version="1.2"><file><body><trans-unit id="1"><source>A</source></trans-unit><trans-unit id="1"><source>B</source></trans-unit></body></file></xliff>"#,
    ] {
        fs::write(&path, xml)?;
        assert!(
            import_document(&path, "en", "es", &Cancellation::default()).is_err(),
            "accepted {xml}"
        );
    }
    Ok(())
}

#[test]
fn xliff_language_conflicts_are_explicit_and_regions_not_relabelled() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("portuguese.xlf");
    let xml = r#"<xliff version="1.2"><file source-language="pt-BR" target-language="es"><body><trans-unit id="1"><source>Olá</source></trans-unit></body></file></xliff>"#;
    fs::write(&path, xml)?;
    let cancel = Cancellation::default();
    assert!(import_document(&path, "pt-PT", "es", &cancel).is_err());
    assert!(import_document(&path, "pt-BR", "fr", &cancel).is_err());
    let doc = import_document(&path, "pt-BR", "es", &cancel)?;
    assert_eq!(doc.source_lang, "pt-BR");
    assert_eq!(doc.segments[0].source, "Olá");
    let output = serialize_document(&doc, &["Hola".into()], &cancel)?;
    assert!(
        std::str::from_utf8(&output)
            .map_err(|e| CatError::Format(e.to_string()))?
            .contains("source-language=\"pt-BR\"")
    );
    assert_eq!(fs::read(&path)?, xml.as_bytes());
    Ok(())
}

#[test]
fn xliff_human_state_and_locks_roundtrip_without_stale_review_approval() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.xlf");
    let xml = r#"<xliff version="1.2"><file><body><trans-unit id="1" approved="yes" translate="no"><source>A</source><target state="final">Approved</target></trans-unit><trans-unit id="2"><source>B</source><target state="signed-off">Signed</target></trans-unit><trans-unit id="3"><source>C</source><target state="translated">Translated</target></trans-unit><trans-unit id="4" approved="yes"><source>D</source><target>Reviewed</target></trans-unit></body></file></xliff>"#;
    fs::write(&path, xml)?;
    let cancel = Cancellation::default();
    let mut doc = import_document(&path, "en", "es", &cancel)?;
    assert!(
        doc.segments
            .iter()
            .all(|s| s.state == SegmentState::Confirmed)
    );
    let mut targets = doc
        .segments
        .iter()
        .map(|s| s.target.clone())
        .collect::<Vec<_>>();
    assert_eq!(serialize_document(&doc, &targets, &cancel)?, xml.as_bytes());
    targets[0] = "Human replacement".into();
    doc.segments[0].locked = false;
    doc.segments[1].state = SegmentState::Draft;
    doc.segments[3].state = SegmentState::Draft;
    let output = serialize_document(&doc, &targets, &cancel)?;
    let text = std::str::from_utf8(&output).map_err(|e| CatError::Format(e.to_string()))?;
    assert!(!text.contains("approved=\"yes\""));
    fs::write(&path, output)?;
    let reparsed = import_document(&path, "en", "es", &cancel)?;
    assert_eq!(reparsed.segments[0].state, SegmentState::Confirmed);
    assert!(!reparsed.segments[0].locked);
    assert_eq!(reparsed.segments[1].state, SegmentState::Draft);
    assert_eq!(reparsed.segments[2].state, SegmentState::Confirmed);
    assert_eq!(reparsed.segments[3].state, SegmentState::Draft);
    Ok(())
}

#[test]
fn tmx_stream_roundtrip_preserves_metadata_and_rejects_ambiguous_or_inline_units() -> Result<()> {
    let xml = r#"<?xml version="1.0"?><tmx version="1.4" xmlns:customer="urn:customer"><header/><body><tu tuid="a" creationid="human" customer:domain="legal"><prop type="client">ACME</prop><tuv xml:lang="en"><seg>A &amp; B🙂</seg></tuv><tuv xml:lang="es"><prop type="note">legal</prop><seg>عربي日本語é</seg></tuv><tuv xml:lang="de"><seg>third variant</seg></tuv></tu></body></tmx>"#;
    let cancel = Cancellation::default();
    let mut units = Vec::new();
    assert_eq!(
        import_tmx(Cursor::new(xml), "en", "es", &cancel, |u| {
            units.push(u);
            Ok(())
        })?,
        1
    );
    assert_eq!(units[0].source, "A & B🙂");
    assert!(units[0].raw_xml.contains("creationid=\"human\""));
    assert!(units[0].raw_xml.contains("xmlns:customer=\"urn:customer\""));
    let mut output = Vec::new();
    assert_eq!(
        export_tmx(&mut output, units.into_iter().map(Ok), &cancel)?,
        1
    );
    let mut reparsed = Vec::new();
    import_tmx(Cursor::new(output), "en", "es", &cancel, |u| {
        reparsed.push(u);
        Ok(())
    })?;
    assert!(reparsed[0].raw_xml.contains("third variant"));
    assert!(reparsed[0].raw_xml.contains("customer:domain=\"legal\""));
    for tu in [
        r#"<tu><tuv xml:lang="en"><seg>A</seg></tuv><tuv xml:lang="EN"><seg>B</seg></tuv></tu>"#,
        r#"<tu><tuv xml:lang="en"><seg>A<ph>x</ph></seg></tuv></tu>"#,
    ] {
        let xml = format!("<tmx version=\"1.4\"><body>{tu}</body></tmx>");
        assert!(import_tmx(Cursor::new(xml), "en", "es", &cancel, |_| Ok(())).is_err());
    }
    cancel.cancel();
    assert!(matches!(
        import_tmx(Cursor::new(xml), "en", "es", &cancel, |_| Ok(())),
        Err(CatError::Cancelled)
    ));
    Ok(())
}
