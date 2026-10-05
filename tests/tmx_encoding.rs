use lumencat::{formats, model::*, storage::ProjectStore};
use std::{
    fs,
    io::{BufReader, Cursor},
};

const TU: &str = r#"<tu tuid="unicode"><prop type="client">café</prop><tuv xml:lang="en"><seg>Hello <bpt i="1" x="1">&lt;b&gt;</bpt>🌍<ept i="1">&lt;/b&gt;</ept></seg></tuv><tuv xml:lang="es"><seg>Hola <bpt i="2" x="1">&lt;b&gt;</bpt>🌍<ept i="2">&lt;/b&gt;</ept></seg></tuv></tu>"#;

fn utf16(xml: &str, little: bool) -> Vec<u8> {
    let mut bytes = if little {
        vec![0xff, 0xfe]
    } else {
        vec![0xfe, 0xff]
    };
    for word in xml.encode_utf16() {
        bytes.extend(if little {
            word.to_le_bytes()
        } else {
            word.to_be_bytes()
        });
    }
    bytes
}

#[test]
fn utf16_memories_keep_codes_and_unicode_through_storage_and_utf8_export() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let evidence = std::env::var_os("LUMENCAT_TMX_ENCODING_RUN_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &evidence {
        fs::create_dir(path)?;
    }
    let dir = evidence.as_deref().unwrap_or(temporary.path());
    let cancel = Cancellation::default();
    for (index, (little, encoding)) in [(true, "UTF-16"), (false, "UTF-16BE"), (true, "utf-16le")]
        .into_iter()
        .enumerate()
    {
        let xml = format!(
            r#"<?xml version="1.0" encoding="{encoding}"?><tmx version="1.4b"><body>{TU}</body></tmx>"#
        );
        let bytes = utf16(&xml, little);
        for capacity in [1, 7, 8192] {
            let mut units = Vec::new();
            let reader = BufReader::with_capacity(capacity, Cursor::new(&bytes));
            assert_eq!(
                formats::import_tmx(reader, "en", "es", &cancel, |unit| {
                    units.push(unit);
                    Ok(())
                })?,
                1
            );
            assert_eq!(units[0].source, "Hello <g id=\"1\">🌍</g>");
            assert_eq!(units[0].target, "Hola <g id=\"1\">🌍</g>");
        }
        let input = dir.join(format!("input-{index}.tmx"));
        fs::write(&input, &bytes)?;
        let project = dir.join(format!("project-{index}.lcat"));
        let mut store = ProjectStore::open(&project)?;
        assert_eq!(store.import_tmx(&input, "en", "es", &cancel)?, 1);
        store.close()?;
        drop(store);
        let mut store = ProjectStore::open(&project)?;
        let found = store.concordance("Hello", "en", "es")?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].target, "Hola <g id=\"1\">🌍</g>");
        let output = dir.join(format!("out-{index}.tmx"));
        assert_eq!(store.export_tm(&output, &cancel)?, 1);
        let exported = fs::read_to_string(&output)?;
        assert!(exported.contains("<prop type=\"client\">café</prop>"));
        let mut recovered = Vec::new();
        assert_eq!(
            formats::import_tmx(Cursor::new(&exported), "en", "es", &cancel, |unit| {
                recovered.push(unit);
                Ok(())
            })?,
            1
        );
        assert_eq!(recovered[0].target, found[0].target);
        assert_eq!(fs::read(&input)?, bytes);
        store.close()?;
    }
    Ok(())
}

#[test]
fn surrogate_pairs_survive_a_decoder_chunk_boundary() -> Result<()> {
    let prefix = r#"<tmx version="1.4"><body><tu><tuv xml:lang="en"><seg>"#;
    let source = format!("{}🌍", "a".repeat(4095 - prefix.encode_utf16().count()));
    let xml = format!(
        "{prefix}{source}</seg></tuv><tuv xml:lang=\"es\"><seg>Destino 🌍</seg></tuv></tu></body></tmx>"
    );
    for little in [true, false] {
        let mut units = Vec::new();
        assert_eq!(
            formats::import_tmx(
                Cursor::new(utf16(&xml, little)),
                "en",
                "es",
                &Cancellation::default(),
                |unit| {
                    units.push(unit);
                    Ok(())
                }
            )?,
            1
        );
        assert_eq!(units[0].source, source);
        assert_eq!(units[0].target, "Destino 🌍");
    }
    Ok(())
}

#[test]
fn mismatched_declarations_and_malformed_utf16_are_rejected_atomically() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let cancel = Cancellation::default();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?><tmx version="1.4"><body>{TU}</body></tmx>"#
    );
    let mut truncated = utf16(&xml, true);
    truncated.pop();
    let mut invalid = utf16(
        &format!(
            r#"<tmx version="1.4"><body>{}</body></tmx>"#,
            TU.repeat(100)
        ),
        true,
    );
    invalid.extend(0xd800_u16.to_le_bytes());
    let mut streamed = 0;
    assert!(
        formats::import_tmx(Cursor::new(&invalid), "en", "es", &cancel, |_| {
            streamed += 1;
            Ok(())
        })
        .is_err()
    );
    assert!(streamed > 0);
    let inputs = [
        xml.as_bytes().to_vec(),
        utf16(&xml.replace("UTF-16", "UTF-8"), true),
        utf16(&xml.replace("UTF-16", "UTF-16LE"), false),
        truncated,
        invalid,
        utf16(
            r#"<!DOCTYPE tmx SYSTEM "file:///not-used.dtd"><tmx version="1.4"/>"#,
            true,
        ),
    ];
    let mut store = ProjectStore::open(&dir.path().join("project.lcat"))?;
    for (index, bytes) in inputs.iter().enumerate() {
        let path = dir.path().join(format!("bad-{index}.tmx"));
        fs::write(&path, bytes)?;
        assert!(
            store.import_tmx(&path, "en", "es", &cancel).is_err(),
            "case {index}"
        );
        assert!(
            store.concordance("Hello", "en", "es")?.is_empty(),
            "case {index}"
        );
        assert_eq!(fs::read(&path)?, *bytes);
    }
    store.close()?;
    Ok(())
}

#[test]
fn utf8_bom_and_cancellation_keep_the_existing_stream_contract() -> Result<()> {
    let xml = format!(r#"<tmx version="1.4"><body>{TU}{TU}</body></tmx>"#);
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(xml.as_bytes());
    assert_eq!(
        formats::import_tmx(
            BufReader::with_capacity(1, Cursor::new(bytes)),
            "en",
            "es",
            &Cancellation::default(),
            |_| Ok(())
        )?,
        2
    );
    let cancel = Cancellation::default();
    let token = cancel.clone();
    let mut seen = 0;
    let result = formats::import_tmx(Cursor::new(utf16(&xml, false)), "en", "es", &cancel, |_| {
        seen += 1;
        token.cancel();
        Ok(())
    });
    assert!(matches!(result, Err(CatError::Cancelled)));
    assert_eq!(seen, 1);
    Ok(())
}

#[cfg(windows)]
#[test]
#[ignore = "requiere Trados y LUMENCAT_TMX_SDK_RUN_DIR nuevo bajo output/verification"]
fn utf16_codes_and_unicode_are_consumed_by_the_trados_memory_engine() -> Result<()> {
    let dir = std::path::PathBuf::from(
        std::env::var_os("LUMENCAT_TMX_SDK_RUN_DIR")
            .ok_or_else(|| CatError::Invalid("falta directorio desechable de evidencia".into()))?,
    );
    fs::create_dir(&dir)?;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?><tmx version="1.4"><body>{}</body></tmx>"#,
        TU.replace("xml:lang=\"en\"", "xml:lang=\"en-US\"")
            .replace("xml:lang=\"es\"", "xml:lang=\"de-DE\"")
            .replace("Hello ", "LumenCAT UTF16 SDK café ")
            .replace("Hola ", "LumenCAT UTF16 SDK geprüft ")
    );
    let encoded = utf16(&xml, false);
    let input = dir.join("input-utf16be.tmx");
    fs::write(&input, &encoded)?;
    let mut store = ProjectStore::open(&dir.join("project.lcat"))?;
    let cancel = Cancellation::default();
    assert_eq!(store.import_memory(&input, "en-US", "de-DE", &cancel)?, 1);
    let installed = std::path::PathBuf::from(
        std::env::var_os("ProgramFiles")
            .ok_or_else(|| CatError::Invalid("falta ProgramFiles".into()))?,
    )
    .join("Trados/Trados Studio/Studio19/Samples/Projects/SampleProject/TMs/English-German.sdltm");
    let original = dir.join("original.sdltm");
    fs::copy(installed, &original)?;
    let baseline = fs::read(&original)?;
    let updated = dir.join("updated.sdltm");
    let report = store.update_sdltm(&original, &updated, "en-US", "de-DE", &cancel)?;
    assert_eq!(report.added, 1);
    assert_eq!(report.unit_count, 44);
    let mut recovered = Vec::new();
    assert_eq!(
        formats::sdltm::import(&updated, "en-US", "de-DE", &cancel, |unit| {
            recovered.push(unit);
            Ok(())
        })?,
        44
    );
    let unit = recovered
        .iter()
        .find(|unit| unit.source.starts_with("LumenCAT UTF16 SDK café "))
        .ok_or_else(|| {
            CatError::Invalid("el motor Trados no recuperó la unidad Unicode importada".into())
        })?;
    assert!(unit.source.contains("🌍"));
    assert!(unit.target.starts_with("LumenCAT UTF16 SDK geprüft "));
    assert!(unit.target.contains("🌍"));
    assert!(
        lumencat::editing::parts(&unit.source)
            .iter()
            .any(|(_, code)| *code)
    );
    assert!(
        lumencat::editing::parts(&unit.target)
            .iter()
            .any(|(_, code)| *code)
    );
    fs::write(dir.join("recovered.tmx"), {
        let mut bytes = Vec::new();
        formats::export_tmx(&mut bytes, recovered.into_iter().map(Ok), &cancel)?;
        bytes
    })?;
    assert_eq!(fs::read(&original)?, baseline);
    assert_eq!(fs::read(&input)?, encoded);
    store.close()?;
    Ok(())
}
