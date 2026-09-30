use lumencat::{editing, formats, model::*, storage::ProjectStore};
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

/// Traducción explícita por segmento: conserva los códigos protegidos `<g>`/`<x/>`
/// y traduce solo las partes de texto con un diccionario del corpus de prueba.
fn translate(source: &str) -> String {
    const PHRASES: &[(&str, &str)] = &[
        (
            "Equipment inspection report",
            "Informe de inspección del equipamiento",
        ),
        ("Check the ", "Revisa los "),
        ("critical", "críticos"),
        (
            " components before starting.",
            " componentes antes de comenzar.",
        ),
        ("Inspection results", "Resultados de la inspección"),
        (
            "Inspection checklist",
            "Lista de verificación de inspección",
        ),
        ("Component", "Componente"),
        ("Status", "Estado"),
        ("Observation", "Observación"),
        ("Safety guard", "Protección de seguridad"),
        ("Ready", "Listo"),
        ("No visible damage", "Sin daños visibles"),
        ("Power cable", "Cable de alimentación"),
        ("Replace", "Reemplazar"),
        (
            "Disconnect before inspection",
            "Desconecta antes de inspeccionar",
        ),
        ("Reference photograph", "Fotografía de referencia"),
        ("Detail photographs", "Fotografías de detalle"),
        ("Front view", "Vista frontal"),
        ("Rear view", "Vista trasera"),
        ("Maintenance instructions", "Instrucciones de mantenimiento"),
        (
            "Disconnect the equipment and verify that the indicator is off.",
            "Desconecta el equipamiento y verifica que el indicador esté apagado.",
        ),
        (
            "Record the inspection results and report any damage.",
            "Registra los resultados de la inspección e informa cualquier daño.",
        ),
    ];
    editing::parts(source)
        .into_iter()
        .map(|(piece, is_tag)| {
            if is_tag {
                return piece.to_owned();
            }
            for (from, to) in PHRASES {
                if piece == *from {
                    return (*to).to_owned();
                }
            }
            format!("[ES] {piece}")
        })
        .collect()
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Indica el directorio de muestras")?,
    );
    let cancel = Cancellation::default();
    let mut store = ProjectStore::open(&directory.join("ui-verification.lcat"))?;
    let mut rejected = Vec::new();
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
        let document = match formats::import_document(&path, "en", "es", &cancel) {
            Err(error) => {
                println!("REJECTED {}: {error}", path.display());
                rejected.push(path.display().to_string());
                continue;
            }
            Ok(document) => document,
        };
        let count = document.segments.len();
        let id = store.import_document(&document, &cancel)?;
        let mut ordinal = 0;
        loop {
            let page = store.page(id, ordinal, 50, "")?;
            if page.is_empty() {
                break;
            }
            for segment in page {
                ordinal = segment.ordinal + 1;
                store.edit(&EditCommand {
                    segment_id: segment.id,
                    expected_revision: segment.revision,
                    target: translate(&segment.source),
                    state: SegmentState::Confirmed,
                    locked: false,
                    origin: Origin::Human,
                })?;
            }
            if ordinal >= count {
                break;
            }
        }
        let translated = store.load_document(id)?;
        let targets: Vec<_> = translated
            .segments
            .iter()
            .map(|s| s.target.clone())
            .collect();
        let output = path.with_file_name(format!(
            "{}-translated.docx",
            path.file_stem().ok_or("Sin nombre")?.to_string_lossy()
        ));
        formats::export_document(&translated, &targets, &output, &cancel)?;
        let imported = formats::import_document(&output, "es", "en", &cancel)?;
        assert_eq!(imported.segments.len(), count);
        for (segment, target) in imported.segments.iter().zip(&targets) {
            assert_eq!(&segment.source, target);
        }
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
            "PASS {}: {count} segmentos; traducción completa vía SQLite, exportación y reimportación; partes ajenas idénticas",
            path.display()
        );
        assert_eq!(std::fs::read(&path)?, original);
    }
    store.close()?;
    if !rejected.is_empty() {
        return Err(format!("REJECTED: {}", rejected.join(", ")).into());
    }
    Ok(())
}
