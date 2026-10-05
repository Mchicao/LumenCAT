use lumencat::{formats, model::*, storage::ProjectStore};
use rusqlite::Connection;
use std::{fs, io::BufReader, path::Path};

const DDL: &str = "CREATE TABLE translation_memories (
    id INTEGER PRIMARY KEY,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    tucount INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE translation_units (
    id INTEGER PRIMARY KEY,
    source_segment TEXT,
    target_segment TEXT,
    creation_date DATETIME,
    creation_user TEXT,
    change_date DATETIME,
    change_user TEXT,
    last_used_date DATETIME,
    usage_counter INTEGER NOT NULL DEFAULT 0
);";

fn segment(culture: &str, elements: &str) -> String {
    format!(
        "<Segment xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><Elements>{elements}</Elements><CultureName>{culture}</CultureName></Segment>"
    )
}

fn txt(value: &str) -> String {
    format!("<Text><Value>{value}</Value></Text>")
}

fn tag(kind: &str, anchor: u32, align: u32, tag_id: &str) -> String {
    format!(
        "<Tag><Type>{kind}</Type><Anchor>{anchor}</Anchor><AlignmentAnchor>{align}</AlignmentAnchor><TagID>{tag_id}</TagID><CanHide>false</CanHide></Tag>"
    )
}

struct Fixture {
    path: std::path::PathBuf,
    connection: Connection,
}

fn build(path: &Path, tucount: i64, units: &[(String, String)]) -> Result<Fixture> {
    let connection = Connection::open(path)?;
    connection.execute_batch(DDL)?;
    connection.execute(
        "INSERT INTO translation_memories (source_language, target_language, tucount) VALUES ('en-US', 'de-DE', ?1)",
        [tucount],
    )?;
    for (i, (source, target)) in units.iter().enumerate() {
        connection.execute(
            "INSERT INTO translation_units (id, source_segment, target_segment, creation_date, creation_user, usage_counter) VALUES (?1, ?2, ?3, '2000-01-18 15:43:15', 'CAROL-ANN', 2)",
            rusqlite::params![(i + 1) as i64, source, target],
        )?;
    }
    Ok(Fixture {
        path: path.to_path_buf(),
        connection,
    })
}

fn tagged_units() -> Vec<(String, String)> {
    vec![
        (
            segment("en-US", &txt("Plain source.")),
            segment("de-DE", &txt("Einfache Quelle.")),
        ),
        (
            segment(
                "en-US",
                &format!(
                    "{}{}{}{}{}",
                    txt("This conference presents the new "),
                    tag("Start", 1, 1, "1"),
                    txt("education programme"),
                    tag("End", 1, 0, "1"),
                    txt(" unveiled by the Minister for Education last year.")
                ),
            ),
            segment(
                "de-DE",
                &format!(
                    "{}{}{}{}{}",
                    txt("Auf dieser Tagung wird das neue "),
                    tag("Start", 1, 1, "1"),
                    txt("Erziehungsprogramm"),
                    tag("End", 1, 0, "1"),
                    txt(
                        " präsentiert, welches letztes Jahr vom Bildungsminister vorgestellt wurde."
                    )
                ),
            ),
        ),
        (
            segment("en-US", &txt("For example: Sunday.")),
            segment(
                "de-DE",
                &format!("{}{}", txt("Beispiel:"), tag("Standalone", 1, 0, "7")),
            ),
        ),
    ]
}

const TAGGED_SOURCE: &str = "This conference presents the new <g id=\"1\">education programme</g> unveiled by the Minister for Education last year.";
const TAGGED_TARGET: &str = "Auf dieser Tagung wird das neue <g id=\"1\">Erziehungsprogramm</g> präsentiert, welches letztes Jahr vom Bildungsminister vorgestellt wurde.";

fn native_units(path: &Path) -> Result<Vec<TmUnit>> {
    let mut units = Vec::new();
    formats::sdltm_native::import(path, "en-US", "de-DE", &Cancellation::default(), |unit| {
        units.push(unit);
        Ok(())
    })?;
    Ok(units)
}

fn native_error(path: &Path) -> String {
    let Err(error) = native_units(path) else {
        panic!("esperaba un rechazo del lector nativo para {path:?}");
    };
    error.to_string()
}

#[test]
fn native_import_maps_tags_and_system_fields_and_roundtrips_tmx() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let fixture = build(&directory.path().join("memory.sdltm"), 3, &tagged_units())?;
    let units = native_units(&fixture.path)?;
    drop(fixture.connection);
    assert_eq!(units.len(), 3);
    assert_eq!(units[0].source, "Plain source.");
    assert_eq!(units[0].target, "Einfache Quelle.");
    assert!(
        units[0]
            .raw_xml
            .contains("creationdate=\"20000118T154315Z\"")
            && units[0].raw_xml.contains("creationid=\"CAROL-ANN\"")
            && units[0].raw_xml.contains("usagecount=\"2\""),
        "atributos de sistema ausentes en {}",
        units[0].raw_xml
    );
    assert_eq!(units[1].source, TAGGED_SOURCE);
    assert_eq!(units[1].target, TAGGED_TARGET);
    assert!(
        units[1]
            .raw_xml
            .contains("<bpt i=\"1\" type=\"1\" x=\"1\">")
            && units[1].raw_xml.contains("<ept i=\"1\">"),
        "dialecto bpt/ept ausente en {}",
        units[1].raw_xml
    );
    assert!(
        units[2].target.contains("<x id=") && units[2].raw_xml.contains("<ph type=\"7\""),
        "standalone mal representado: {} / {}",
        units[2].target,
        units[2].raw_xml
    );

    let store_directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&store_directory.path().join("project.lcat"))?;
    store.insert_tm_units(units, &Cancellation::default())?;
    let exact: Vec<_> = store
        .matches(TAGGED_SOURCE, "en-US", "de-DE")?
        .into_iter()
        .filter(|m| m.exact)
        .collect();
    assert_eq!(exact.len(), 1, "falta la coincidencia exacta con códigos");
    assert_eq!(exact[0].target, TAGGED_TARGET);
    assert!(
        !store
            .concordance("education programme", "en-US", "de-DE")?
            .is_empty()
    );
    let exported = store_directory.path().join("export.tmx");
    assert_eq!(store.export_tm(&exported, &Cancellation::default())?, 3);
    let mut reimported = Vec::new();
    formats::import_tmx(
        BufReader::new(fs::File::open(&exported)?),
        "en-US",
        "de-DE",
        &Cancellation::default(),
        |unit| {
            reimported.push(unit);
            Ok(())
        },
    )?;
    assert_eq!(reimported.len(), 3);
    assert!(
        reimported
            .iter()
            .any(|u| u.source == TAGGED_SOURCE && u.target == TAGGED_TARGET)
    );
    store.close()?;
    Ok(())
}

#[test]
fn native_import_rejects_drift_unknown_variants_and_foreign_files() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let drift = build(&directory.path().join("drift.sdltm"), 9, &tagged_units())?;
    let error = native_error(&drift.path);
    drop(drift.connection);
    assert!(
        error.contains("declara 9 unidades pero se leyeron 3"),
        "mensaje de desviación: {error}"
    );

    let mismatch = build(&directory.path().join("pair.sdltm"), 3, &tagged_units())?;
    let error = formats::sdltm_native::import(
        &mismatch.path,
        "es-ES",
        "de-DE",
        &Cancellation::default(),
        |_| Ok(()),
    )
    .err()
    .map(|e| e.to_string())
    .unwrap_or_default();
    drop(mismatch.connection);
    assert!(
        error.contains("declara en-US→de-DE"),
        "mensaje de par de idiomas: {error}"
    );

    let mut placeholder = tagged_units();
    placeholder[0].0 = segment(
        "en-US",
        &format!("{}{}", txt("Broken "), tag("TextPlaceholder", 1, 0, "9")),
    );
    let unsupported = build(&directory.path().join("variant.sdltm"), 3, &placeholder)?;
    let error = native_error(&unsupported.path);
    drop(unsupported.connection);
    assert!(
        error.contains("TextPlaceholder"),
        "mensaje de variante no soportada: {error}"
    );

    let mut unbalanced = tagged_units();
    unbalanced[0].0 = segment(
        "en-US",
        &format!("{}{}", txt("Only end "), tag("End", 4, 0, "1")),
    );
    let unbalanced = build(&directory.path().join("unbalanced.sdltm"), 3, &unbalanced)?;
    assert!(native_units(&unbalanced.path).is_err());
    drop(unbalanced.connection);

    let foreign_path = directory.path().join("foreign.db");
    Connection::open(&foreign_path)?.execute_batch("CREATE TABLE other (x);")?;
    let error = native_error(&foreign_path);
    assert!(
        error.contains("no es una memoria SDLTM reconocida"),
        "mensaje de archivo ajeno: {error}"
    );

    let incomplete_path = directory.path().join("incomplete.sdltm");
    let connection = Connection::open(&incomplete_path)?;
    connection.execute_batch(
        "CREATE TABLE translation_memories (id INTEGER PRIMARY KEY, source_language TEXT, target_language TEXT, tucount INTEGER);
         CREATE TABLE translation_units (id INTEGER PRIMARY KEY, source_segment TEXT);",
    )?;
    drop(connection);
    let error = native_error(&incomplete_path);
    assert!(
        error.contains("target_segment"),
        "mensaje de esquema incompleto: {error}"
    );

    let cancelled = build(&directory.path().join("cancel.sdltm"), 3, &tagged_units())?;
    let cancellation = Cancellation::default();
    cancellation.cancel();
    assert!(matches!(
        formats::sdltm_native::import(&cancelled.path, "en-US", "de-DE", &cancellation, |_| Ok(())),
        Err(CatError::Cancelled)
    ));
    Ok(())
}

#[test]
fn import_memory_uses_the_native_reader_without_trados() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let fixture = build(&directory.path().join("memory.sdltm"), 3, &tagged_units())?;
    drop(fixture.connection);
    // Rust 2024: la mutación de entorno es unsafe; queda confinada a esta prueba.
    unsafe {
        std::env::set_var(
            "LUMENCAT_TRADOS_SDK_DIR",
            directory.path().join("sin-trados"),
        )
    };
    let count = formats::import_memory(
        &fixture.path,
        "en-US",
        "de-DE",
        &Cancellation::default(),
        |_| Ok(()),
    );
    unsafe { std::env::remove_var("LUMENCAT_TRADOS_SDK_DIR") };
    assert_eq!(count?, 3);
    Ok(())
}

#[test]
#[ignore = "requiere LUMENCAT_NATIVE_TEST_RUN_DIR nuevo bajo output/verification; modern.sdltm opcional creada por el probe del SDK"]
fn native_reader_imports_real_and_modern_sdltm_without_trados() -> Result<()> {
    let directory = Path::new(
        &std::env::var("LUMENCAT_NATIVE_TEST_RUN_DIR")
            .map_err(|_| CatError::Invalid("falta LUMENCAT_NATIVE_TEST_RUN_DIR".into()))?,
    )
    .to_path_buf();
    fs::create_dir_all(&directory)?;
    let program_files =
        std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into());
    let installed = Path::new(&program_files).join(
        "Trados/Trados Studio/Studio19/Samples/Projects/SampleProject/TMs/English-German.sdltm",
    );
    let original = directory.join("original.sdltm");
    fs::copy(installed, &original)?;
    let baseline = fs::read(&original)?;
    let mut units = Vec::new();
    assert_eq!(
        formats::sdltm_native::import(
            &original,
            "en-US",
            "de-DE",
            &Cancellation::default(),
            |u| {
                units.push(u);
                Ok(())
            }
        )?,
        43
    );
    assert!(
        units.iter().any(|u| u.source == TAGGED_SOURCE
            && u.target
                .starts_with("Auf dieser Tagung wird das neue <g id=\"1\">Erziehungsprogramm</g>")),
        "la TU con par de tags de la fixture real no se mapeó"
    );
    let legacy_sources: std::collections::BTreeSet<_> =
        units.iter().map(|u| u.source.clone()).collect();
    let store_directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&store_directory.path().join("project.lcat"))?;
    store.insert_tm_units(units, &Cancellation::default())?;
    let exact: Vec<_> = store
        .matches(TAGGED_SOURCE, "en-US", "de-DE")?
        .into_iter()
        .filter(|m| m.exact)
        .collect();
    assert_eq!(
        exact.len(),
        1,
        "falta la coincidencia exacta de la fixture real"
    );
    let exported = directory.join("roundtrip.tmx");
    assert_eq!(store.export_tm(&exported, &Cancellation::default())?, 43);
    let mut reimported = 0;
    formats::import_tmx(
        BufReader::new(fs::File::open(&exported)?),
        "en-US",
        "de-DE",
        &Cancellation::default(),
        |_| {
            reimported += 1;
            Ok(())
        },
    )?;
    assert_eq!(reimported, 43);
    store.close()?;
    assert_eq!(fs::read(&original)?, baseline);

    let modern = directory.join("modern.sdltm");
    if modern.is_file() {
        let mut modern_units = Vec::new();
        assert_eq!(
            formats::sdltm_native::import(
                &modern,
                "en-US",
                "de-DE",
                &Cancellation::default(),
                |u| {
                    modern_units.push(u);
                    Ok(())
                }
            )?,
            43
        );
        let modern_sources: std::collections::BTreeSet<_> =
            modern_units.iter().map(|u| u.source.clone()).collect();
        assert_eq!(
            legacy_sources, modern_sources,
            "las fuentes de la fixture moderna difieren"
        );
        assert_eq!(fs::read(&original)?, baseline);
    }
    Ok(())
}
