use lumencat::{
    formats::docx::{import_docx, serialize_docx},
    model::{Cancellation, EditCommand, Origin, SegmentState},
    storage::ProjectStore,
};
use std::{
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    )
}
fn package(
    path: &Path,
    xml: &str,
    extras: &[(&str, &[u8])],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let mut types = TYPES.replace(
        "</Types>",
        "<Default Extension=\"bin\" ContentType=\"application/octet-stream\"/></Types>",
    );
    if extras.iter().any(|(name, _)| *name == "word/styles.xml") {
        types=types.replace("</Types>","<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>");
    }
    for (name, data) in [
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", RELS.as_bytes()),
        ("word/document.xml", xml.as_bytes()),
    ]
    .into_iter()
    .filter(|(name, _)| !extras.iter().any(|(replacement, _)| replacement == name))
    .chain(extras.iter().copied())
    {
        writer.start_file(
            name,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )?;
        writer.write_all(data)?;
    }
    std::fs::write(path, writer.finish()?.into_inner())?;
    Ok(())
}
fn part(bytes: &[u8], name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    let mut output = Vec::new();
    zip.by_name(name)?.read_to_end(&mut output)?;
    Ok(output)
}

#[test]
fn rejects_expansion_dtd_and_corrupt_crc() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.docx");
    let xml = document("<w:p><w:r><w:t>Hello</w:t></w:r></w:p>");
    let bomb = vec![b'a'; 4 * 1024 * 1024];
    package(&input, &xml, &[("word/media/bomb.bin", &bomb)])?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    package(&input, &format!("<!DOCTYPE x [<!ENTITY y 'x'>]>{xml}"), &[])?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    package(&input, &xml, &[("word/styles.xml", b"\xff")])?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    package(&input, &xml, &[])?;
    let mut bytes = std::fs::read(&input)?;
    let start = {
        let mut zip = ZipArchive::new(Cursor::new(&bytes))?;
        zip.by_name("word/document.xml")?.data_start() as usize
    };
    bytes[start + 5] ^= 1;
    std::fs::write(&input, bytes)?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    Ok(())
}

#[test]
fn word_roundtrip_preserves_package_and_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.docx");
    let xml = document(
        r#"<w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Hello &amp; world</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
    );
    let resource = b"opaque bytes \x00\xff";
    package(&input,&xml,&[("word/styles.xml",b"<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"/>"),("word/media/opaque.bin",resource),("word/_rels/document.xml.rels",b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rStyle\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>")])?;
    let cancel = Cancellation::default();
    let imported = import_docx(&input, "en", "es", &cancel)?;
    assert_eq!(imported.segments[0].source, "Hello & world");
    let targets = vec![
        "  Hola & <mundo> 😀 אבג 漢字 e\u{301} ".into(),
        "Tabla".into(),
    ];
    let output = serialize_docx(&imported, &targets, &cancel)?;
    for name in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/styles.xml",
        "word/_rels/document.xml.rels",
        "word/media/opaque.bin",
    ] {
        assert_eq!(part(&output, name)?, part(&imported.original, name)?);
    }
    let output_path = dir.path().join("translated.docx");
    std::fs::write(&output_path, &output)?;
    let again = import_docx(&output_path, "es", "en", &cancel)?;
    assert_eq!(
        again.segments.iter().map(|s| &s.source).collect::<Vec<_>>(),
        targets.iter().collect::<Vec<_>>()
    );
    let output_xml = String::from_utf8(part(&output, "word/document.xml")?)?;
    assert!(output_xml.contains("<w:pStyle w:val=\"Title\"/>"));
    assert!(output_xml.contains("<w:rPr><w:b/></w:rPr>"));
    assert_eq!(std::fs::read(input)?, imported.original);
    Ok(())
}

#[test]
fn word_roundtrip_merges_same_style_text_across_runs() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("split-runs.docx");
    let xml = document(
        "<w:p><w:pPr><w:rPr><w:b/></w:rPr></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Hello&#x26; </w:t><w:t>42.</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t> again.</w:t></w:r></w:p>",
    );
    package(&input, &xml, &[])?;
    let cancel = Cancellation::default();
    let imported = import_docx(&input, "en", "es", &cancel)?;
    assert_eq!(imported.segments[0].source, "Hello& 42. again.");

    let output = serialize_docx(&imported, &["Hola 42. de nuevo.".into()], &cancel)?;
    let output_path = dir.path().join("translated.docx");
    std::fs::write(&output_path, &output)?;
    assert_eq!(
        import_docx(&output_path, "es", "en", &cancel)?.segments[0].source,
        "Hola 42. de nuevo."
    );
    let output_xml = String::from_utf8(part(&output, "word/document.xml")?)?;
    assert_eq!(output_xml.matches("<w:rPr><w:b/></w:rPr>").count(), 3);
    assert!(output_xml.contains("<w:pPr><w:rPr><w:b/></w:rPr></w:pPr>"));
    assert_eq!(output_xml.matches("<w:t").count(), 1);
    Ok(())
}

#[test]
fn rejects_xml_10_disallowed_controls_in_word_sources_and_targets()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("controls.docx");
    for character in ["&#x1;", "&#x1f;"] {
        package(
            &input,
            &document(&format!(
                "<w:p><w:r><w:t>bad{character}text</w:t></w:r></w:p>"
            )),
            &[],
        )?;
        assert!(
            import_docx(&input, "en", "es", &Cancellation::default()).is_err(),
            "invalid XML 1.0 control {character}"
        );
    }

    package(
        &input,
        &document("<w:p><w:r><w:t>valid</w:t></w:r></w:p>"),
        &[],
    )?;
    let imported = import_docx(&input, "en", "es", &Cancellation::default())?;
    for character in ['\u{1}', '\u{1f}'] {
        assert!(
            serialize_docx(
                &imported,
                &[format!("bad{character}text")],
                &Cancellation::default()
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn word_translation_persists_undo_and_exports_without_changing_original()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("input.docx");
    package(
        &input,
        &document("<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Hello 42.</w:t></w:r></w:p>"),
        &[],
    )?;
    let token = Cancellation::default();
    let original = std::fs::read(&input)?;
    let imported = import_docx(&input, "en", "es", &token)?;
    let database = directory.path().join("project.db");
    let mut store = ProjectStore::open(&database)?;
    let id = store.import_document(&imported, &token)?;
    let segment = store.page(id, 0, 1, "")?.remove(0);
    store.edit(&EditCommand {
        segment_id: segment.id,
        expected_revision: segment.revision,
        target: "Hola 42.".into(),
        state: SegmentState::Confirmed,
        locked: false,
        origin: Origin::Human,
    })?;
    store.close()?;
    drop(store);
    let mut store = ProjectStore::open(&database)?;
    assert_eq!(store.undo()?.map(|s| s.target), Some(String::new()));
    assert_eq!(store.redo()?.map(|s| s.target), Some("Hola 42.".into()));
    let translated = store.load_document(id)?;
    let targets: Vec<_> = translated
        .segments
        .iter()
        .map(|s| s.target.clone())
        .collect();
    let destination = directory.path().join("translated.docx");
    lumencat::formats::export_document(&translated, &targets, &destination, &token)?;
    assert_eq!(
        import_docx(&destination, "es", "en", &token)?.segments[0].source,
        "Hola 42."
    );
    assert_eq!(std::fs::read(&input)?, original);
    assert!(
        lumencat::formats::export_document(&translated, &targets, &destination, &token).is_err()
    );
    store.close()?;
    Ok(())
}

#[test]
fn rejects_unsupported_structure_instead_of_dropping_text() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.docx");
    for body in [
        "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t>italic</w:t></w:r></w:p>",
        "<w:p><w:hyperlink><w:r><w:t>link</w:t></w:r></w:hyperlink></w:p>",
        "<w:p><w:ins><w:r><w:t>change</w:t></w:r></w:ins></w:p>",
        "<w:p><w:r><w:t>text</w:t><w:tab/></w:r></w:p>",
        "<w:p><w:r><w:t>text</w:t></w:r>",
        "<w:p><w:r><w:t>&unknown;</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>ab<!--c-->cd</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>ab<?review c?>cd</w:t></w:r></w:p>",
    ] {
        package(&input, &document(body), &[])?;
        assert!(
            import_docx(&input, "en", "es", &Cancellation::default()).is_err(),
            "{body}"
        );
    }
    package(&input,&document("<w:p><w:r><w:t>text</w:t></w:r></w:p>"),&[("word/header1.xml",b"<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t>header</w:t></w:r></w:p></w:hdr>")])?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    package(&input,&document("<w:p><w:r><w:t>text</w:t></w:r></w:p>"),&[("word/_rels/document.xml.rels",b"<Relationships><Relationship TargetMode=\"External\" Target=\"https://example.com\"/></Relationships>")])?;
    assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    for (name,data) in [
        ("word/Embeddings/object.bin",b"opaque".as_slice()),
        ("word/HEADER1.XML",b"<x:hdr xmlns:x=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><x:p><x:r><x:t>header</x:t></x:r></x:p></x:hdr>".as_slice()),
        ("word/glossary/document.xml",b"<x:document xmlns:x=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><x:t>glossary</x:t></x:document>".as_slice()),
        ("word/numbering.xml",b"<x:numbering xmlns:x=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><x:lvlText x:val=\"Chapter %1\"/></x:numbering>".as_slice()),
        ("word/STYLES.XML",b"not XML".as_slice()),
        ("word/_rels/document.xml.rels",b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"r1\" Type=\"style\" Target=\"missing.xml\"/></Relationships>".as_slice()),
        ("word/_rels/document.xml.rels",b"<Relationships xmlns=\"wrong\"><Relationship Id=\"r1\" Type=\"style\" Target=\"document.xml\"/></Relationships>".as_slice()),
        ("[Content_Types].xml",b"<Types xmlns=\"wrong\"><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>".as_slice()),
        ("_rels/.rels",b"<Relationships xmlns=\"wrong\"><Relationship Id=\"r1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>".as_slice()),
        ("word/_rels/document.xml.rels",b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"r1\" Type=\"style\" Target=\"document.xml\"/><Relationship Id=\"r1\" Type=\"style\" Target=\"document.xml\"/></Relationships>".as_slice()),
        ("word/_rels/document.xml.rels",b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"r1\" Type=\"style\" Target=\"%64ocument.xml\"/></Relationships>".as_slice()),
    ] {
        package(&input,&document("<w:p><w:r><w:t>text</w:t></w:r></w:p>"),&[(name,data)])?;
        assert!(import_docx(&input,"en","es",&Cancellation::default()).is_err(),"{name}");
    }
    Ok(())
}

#[test]
fn rejects_duplicate_zip_entries_invalid_targets_and_cancel()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.docx");
    package(
        &input,
        &document("<w:p><w:r><w:t>Hello</w:t></w:r></w:p>"),
        &[("word/aaaa.xml", b"<a/>"), ("word/bbbb.xml", b"<b/>")],
    )?;
    let cancel = Cancellation::default();
    let imported = import_docx(&input, "en", "es", &cancel)?;
    assert!(serialize_docx(&imported, &["\u{ffff}".into()], &cancel).is_err());
    for target in ["Hola\nMundo", "Hola\rMundo", "Hola\r\nMundo", "Hola\tMundo"] {
        assert!(serialize_docx(&imported, &[target.into()], &cancel).is_err());
    }
    assert!(serialize_docx(&imported, &[], &cancel).is_err());
    let mut duplicate = imported.original.clone();
    // Reescribir los nombres de igual longitud en ambos headers crea un duplicado real.
    for offset in 0..duplicate.len().saturating_sub(13) {
        if &duplicate[offset..offset + 13] == b"word/bbbb.xml" {
            duplicate[offset..offset + 13].copy_from_slice(b"word/aaaa.xml");
        }
    }
    std::fs::write(&input, duplicate)?;
    assert!(import_docx(&input, "en", "es", &cancel).is_err());
    cancel.cancel();
    assert!(serialize_docx(&imported, &["Hola".into()], &cancel).is_err());
    Ok(())
}

#[test]
fn rejects_literal_linebreak_sources() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.docx");
    for source in [
        "Hello\nworld",
        "Hello&#13;world",
        "Hello&#10;world",
        "Hello&#9;world",
    ] {
        package(
            &input,
            &document(&format!("<w:p><w:r><w:t>{source}</w:t></w:r></w:p>")),
            &[],
        )?;
        assert!(import_docx(&input, "en", "es", &Cancellation::default()).is_err());
    }
    Ok(())
}
