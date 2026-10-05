#![cfg(windows)]
use lumencat::{formats, model::*, storage::ProjectStore};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requiere Trados instalado y LUMENCAT_SDK_TEST_RUN_DIR nuevo bajo output/verification"]
fn official_sdk_import_and_update_preserve_the_original_and_reopen_the_copy() -> Result<()> {
    let dir = PathBuf::from(
        std::env::var_os("LUMENCAT_SDK_TEST_RUN_DIR")
            .ok_or_else(|| CatError::Invalid("falta directorio desechable de evidencia".into()))?,
    );
    fs::create_dir(&dir)?;
    let installed = PathBuf::from(
        std::env::var_os("ProgramFiles")
            .ok_or_else(|| CatError::Invalid("falta ProgramFiles".into()))?,
    )
    .join("Trados/Trados Studio/Studio19/Samples/Projects/SampleProject/TMs/English-German.sdltm");
    let original = dir.join("original.sdltm");
    fs::copy(installed, &original)?;
    let baseline = fs::read(&original)?;
    let cancel = Cancellation::default();
    let mut store = ProjectStore::open(&dir.join("project.lcat"))?;
    assert_eq!(
        store.import_memory(&original, "en-US", "de-DE", &cancel)?,
        43
    );
    assert!(
        !store
            .concordance("education programme", "en-US", "de-DE")?
            .is_empty()
    );
    store.insert_tm_units(
        [TmUnit {
            source: "LumenCAT SDK update check.".into(),
            target: "Aktualisierte Übersetzung.".into(),
            source_lang: "en-US".into(),
            target_lang: "de-DE".into(),
            raw_xml: String::new(),
        }],
        &cancel,
    )?;
    store.insert_tm_units(
        [TmUnit {
            source: "Other language must not leak.".into(),
            target: "Otra traducción.".into(),
            source_lang: "en-US".into(),
            target_lang: "es-ES".into(),
            raw_xml: String::new(),
        }],
        &cancel,
    )?;
    let updated = dir.join("updated.sdltm");
    let report = store.update_sdltm(&original, &updated, "en-US", "de-DE", &cancel)?;
    assert_eq!(report.unit_count, 44);
    assert_eq!(report.added, 1);
    assert_eq!(report.read, 44);
    assert_eq!(report.discarded, 0);
    let mut units = Vec::new();
    assert_eq!(
        formats::sdltm::import(&updated, "en-US", "de-DE", &cancel, |unit| {
            units.push(unit);
            Ok(())
        })?,
        44
    );
    assert!(
        units
            .iter()
            .any(|unit| unit.source == "LumenCAT SDK update check."
                && unit.target == "Aktualisierte Übersetzung.")
    );
    assert!(
        !units
            .iter()
            .any(|unit| unit.source == "Other language must not leak.")
    );
    store.insert_tm_units(
        [TmUnit {
            source: "LumenCAT SDK update check.".into(),
            target: "Korrigierte Übersetzung.".into(),
            source_lang: "en-US".into(),
            target_lang: "de-DE".into(),
            raw_xml: String::new(),
        }],
        &cancel,
    )?;
    let corrected = dir.join("corrected.sdltm");
    let correction = store.update_sdltm(&updated, &corrected, "en-US", "de-DE", &cancel)?;
    assert_eq!(correction.unit_count, 44);
    let mut corrected_units = Vec::new();
    formats::sdltm::import(&corrected, "en-US", "de-DE", &cancel, |unit| {
        corrected_units.push(unit);
        Ok(())
    })?;
    assert!(
        corrected_units
            .iter()
            .any(|unit| unit.source == "LumenCAT SDK update check."
                && unit.target == "Korrigierte Übersetzung.")
    );
    assert_eq!(fs::read(&original)?, baseline);
    assert!(
        store
            .update_sdltm(&original, &updated, "en-US", "de-DE", &cancel)
            .is_err()
    );
    let mismatch = dir.join("mismatch.sdltm");
    assert!(
        store
            .update_sdltm(&original, &mismatch, "en-US", "es-ES", &cancel)
            .is_err()
    );
    assert!(!mismatch.exists());
    cancel.cancel();
    let cancelled = dir.join("cancelled.sdltm");
    assert!(matches!(
        store.update_sdltm(&original, &cancelled, "en-US", "de-DE", &cancel),
        Err(CatError::Cancelled)
    ));
    assert!(!cancelled.exists());
    assert_eq!(fs::read(&original)?, baseline);
    store.close()?;
    Ok(())
}
