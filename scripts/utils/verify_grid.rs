use lumencat::{formats, model::*, storage::ProjectStore};
use std::{io::Write, path::PathBuf};

fn main() -> Result<()> {
    let directory = std::env::args().nth(1).map(PathBuf::from).ok_or_else(|| {
        CatError::Invalid("Uso: verify_grid <directorio nuevo de verificación>".into())
    })?;
    std::fs::create_dir_all(&directory)?;
    let source = directory.join("source.txt");
    let mut writer = std::io::BufWriter::new(
        std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(&source)?,
    );
    for index in 0..100_000 {
        writeln!(writer, "Segment {} café 世界", index + 1)?;
    }
    writer.flush()?;
    let cancel = Cancellation::default();
    let document = formats::import_document(&source, "en", "es", &cancel)?;
    assert_eq!(document.segments.len(), 100_000);
    let project = directory.join("large.lcat");
    if project.exists() {
        return Err(CatError::Invalid(
            "El proyecto de verificación ya existe".into(),
        ));
    }
    let mut store = ProjectStore::open(&project)?;
    let id = store.import_document(&document, &cancel)?;
    let first = store.page(id, 0, 1, "")?.remove(0);
    let last = store.page(id, 99_999, 1, "")?.remove(0);
    for (segment, target) in [
        (&first, "Primera traducción café 世界"),
        (&last, "Última traducción café 世界"),
    ] {
        store.confirm(&EditCommand {
            segment_id: segment.id,
            expected_revision: segment.revision,
            target: target.into(),
            state: SegmentState::Draft,
            locked: false,
            origin: Origin::Human,
        })?;
    }
    assert_eq!(store.progress(id)?, (100_000, 2));
    store.close()?;
    drop(store);
    let mut reopened = ProjectStore::open_existing(&project)?;
    assert_eq!(
        reopened.segment(first.id)?.target,
        "Primera traducción café 世界"
    );
    assert_eq!(
        reopened.segment(last.id)?.target,
        "Última traducción café 世界"
    );
    reopened.close()?;
    println!(
        "PASS: 100 000 segmentos; extremos confirmados, durables y recuperados. Proyecto GUI: {}",
        project.display()
    );
    Ok(())
}
