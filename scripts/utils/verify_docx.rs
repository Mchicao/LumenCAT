use lumencat::{formats, model::*, storage::ProjectStore};
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Indica el directorio de muestras")?,
    );
    let cancel = Cancellation::default();
    let mut store = ProjectStore::open(&directory.join("ui-verification.lcat"))?;
    for entry in std::fs::read_dir(&directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "docx")
            || path
                .file_stem()
                .is_some_and(|n| n.to_string_lossy().ends_with("-translated"))
        {
            continue;
        }
        let original = std::fs::read(&path)?;
        match formats::import_document(&path, "en", "es", &cancel) {
            Err(error) => println!("REJECTED {}: {error}", path.display()),
            Ok(document) => {
                let count = document.segments.len();
                let id = store.import_document(&document, &cancel)?;
                let first = store
                    .page(id, 0, 1, "")?
                    .into_iter()
                    .next()
                    .ok_or("Sin segmentos")?;
                store.edit(&EditCommand {
                    segment_id: first.id,
                    expected_revision: first.revision,
                    target: "Traducción de prueba: café y acción".into(),
                    state: SegmentState::Confirmed,
                    locked: false,
                    origin: Origin::Human,
                })?;
                let translated = store.load_document(id)?;
                let targets: Vec<_> = translated
                    .segments
                    .iter()
                    .map(|s| {
                        if s.target.is_empty() {
                            s.source.clone()
                        } else {
                            s.target.clone()
                        }
                    })
                    .collect();
                let output = path.with_file_name(format!(
                    "{}-translated.docx",
                    path.file_stem().ok_or("Sin nombre")?.to_string_lossy()
                ));
                formats::export_document(&translated, &targets, &output, &cancel)?;
                let imported = formats::import_document(&output, "es", "en", &cancel)?;
                assert_eq!(imported.segments.len(), count);
                assert_eq!(imported.segments[0].source, targets[0]);
                let mut before = zip::ZipArchive::new(Cursor::new(&original))?;
                let mut after = zip::ZipArchive::new(Cursor::new(std::fs::read(&output)?))?;
                for index in 0..before.len() {
                    let mut part = before.by_index(index)?;
                    if part.name() == "word/document.xml" {
                        continue;
                    }
                    let mut a = Vec::new();
                    let mut b = Vec::new();
                    part.read_to_end(&mut a)?;
                    after.by_name(part.name())?.read_to_end(&mut b)?;
                    assert_eq!(a, b, "Parte alterada: {}", part.name());
                }
                println!(
                    "PASS {}: {count} segmentos; importación, edición SQLite, exportación y reimportación; partes ajenas idénticas",
                    path.display()
                );
            }
        }
        assert_eq!(std::fs::read(&path)?, original);
    }
    let fixture = ImportedDocument {
        name: "Navegación y etiquetas · QA".into(),
        format: DocumentFormat::Txt,
        original: Vec::new(),
        original_path: None,
        source_lang: "en".into(),
        target_lang: "es".into(),
        segments: (0..180)
            .map(|i| ImportedSegment {
                external_id: i.to_string(),
                source: format!(
                    "Segment {}: café <g id=\"1\">formatted text</g> <x id=\"2\"/>",
                    i + 1
                ),
                target: String::new(),
                state: SegmentState::Draft,
                locked: false,
            })
            .collect(),
    };
    store.import_document(&fixture, &cancel)?;
    store.close()?;
    Ok(())
}
