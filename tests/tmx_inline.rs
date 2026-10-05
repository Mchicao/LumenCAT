use lumencat::{formats, model::*, storage::ProjectStore};
use std::{fs, io::Cursor};

const TMX: &str = r#"<tmx version="1.4b"><header creationtool="Trados" creationtoolversion="19" segtype="sentence" o-tmf="SDL TM8 Format" adminlang="en" srclang="en-US" datatype="xml"/><body><tu tuid="native" creationid="author" usagecount="7"><prop type="x-Client">Keep</prop><tuv xml:lang="es-ES"><seg>Hola <bpt i="7" x="bold">&lt;b&gt;</bpt>mundo<ept i="7">&lt;/b&gt;</ept><ph x="break">&lt;br/&gt;</ph></seg></tuv><tuv xml:lang="en-US"><seg>Hello <bpt i="1" x="bold">&lt;b&gt;</bpt>world<ept i="1">&lt;/b&gt;</ept><ph x="break">&lt;br/&gt;</ph></seg></tuv><tuv xml:lang="fr-FR"><seg>Bonjour <bpt i="4" x="bold">&lt;b&gt;</bpt>monde<ept i="4">&lt;/b&gt;</ept><ph x="break">&lt;br/&gt;</ph></seg></tuv></tu></body></tmx>"#;

#[test]
fn tmx_level_two_keeps_native_codes_variants_and_metadata_through_storage() -> Result<()> {
    let cancel = Cancellation::default();
    let mut units = Vec::new();
    assert_eq!(
        formats::import_tmx(Cursor::new(TMX), "en-US", "es-ES", &cancel, |unit| {
            units.push(unit);
            Ok(())
        })?,
        1
    );
    assert!(units[0].source.contains("world</g>"));
    assert!(units[0].target.contains("mundo</g>"));
    assert!(!units[0].source.contains("&lt;b&gt;"));
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.tmx");
    fs::write(&input, TMX)?;
    let mut store = ProjectStore::open(&dir.path().join("project.lcat"))?;
    assert_eq!(store.import_tmx(&input, "en-US", "es-ES", &cancel)?, 1);
    let found = store.matches(&units[0].source, "en-US", "es-ES")?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].target, units[0].target);
    let output = dir.path().join("out.tmx");
    assert_eq!(store.export_tm(&output, &cancel)?, 1);
    let bytes = fs::read_to_string(&output)?;
    assert!(bytes.contains("usagecount=\"7\""));
    assert!(bytes.contains("<prop type=\"x-Client\">Keep</prop>"));
    assert!(bytes.contains("xml:lang=\"fr-FR\""));
    assert!(
        bytes.contains("<bpt i=\"7\" x=\"bold\">&lt;b&gt;</bpt>mundo<ept i=\"7\">&lt;/b&gt;</ept>")
    );
    let mut reopened = Vec::new();
    formats::import_tmx(Cursor::new(bytes), "en-US", "es-ES", &cancel, |unit| {
        reopened.push(unit);
        Ok(())
    })?;
    assert_eq!(units[0].source, reopened[0].source);
    assert_eq!(units[0].target, reopened[0].target);
    assert_eq!(fs::read_to_string(input)?, TMX);
    store.close()?;
    Ok(())
}

#[test]
fn malformed_native_pairs_roll_back_the_entire_memory_import() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("bad.tmx");
    let valid = TMX
        .split_once("<body>")
        .ok_or_else(|| CatError::Invalid("fixture TMX sin body".into()))?
        .1
        .split_once("</body>")
        .ok_or_else(|| CatError::Invalid("fixture TMX sin cierre body".into()))?
        .0;
    fs::write(
        &input,
        format!(
            "<tmx version=\"1.4\"><body>{valid}{}</body></tmx>",
            valid
                .replace("tuid=\"native\"", "tuid=\"bad\"")
                .replace("<ept i=\"7\">", "<ept i=\"99\">")
        ),
    )?;
    let mut store = ProjectStore::open(&dir.path().join("project.lcat"))?;
    assert!(
        store
            .import_tmx(&input, "en-US", "es-ES", &Cancellation::default())
            .is_err()
    );
    assert!(store.concordance("Hello", "en-US", "es-ES")?.is_empty());
    store.close()?;
    Ok(())
}

#[test]
fn language_specific_formatting_is_preserved_without_inventing_source_tags() -> Result<()> {
    let xml = r#"<tmx version="1.4"><body><tu><tuv xml:lang="en"><seg>Plain source</seg></tuv><tuv xml:lang="es"><seg><bpt i="2" x="target-only"/>Destino<ept i="2"/><ph x="extra"/></seg></tuv></tu></body></tmx>"#;
    let cancel = Cancellation::default();
    let mut units = Vec::new();
    assert_eq!(
        formats::import_tmx(Cursor::new(xml), "en", "es", &cancel, |unit| {
            units.push(unit);
            Ok(())
        })?,
        1
    );
    assert_eq!(units[0].source, "Plain source");
    assert!(units[0].target.contains("Destino</g>"));
    let mut output = Vec::new();
    formats::export_tmx(&mut output, units.into_iter().map(Ok), &cancel)?;
    let output = String::from_utf8(output).map_err(|error| CatError::Format(error.to_string()))?;
    assert!(
        output.contains("<bpt i=\"2\" x=\"target-only\"/>Destino<ept i=\"2\"/><ph x=\"extra\"/>")
    );
    Ok(())
}
