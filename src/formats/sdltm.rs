use crate::{model::*, storage::migrations};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

const SCRIPT: &str = include_str!("../../scripts/utils/trados_memory_bridge.ps1");

#[derive(Debug, Deserialize)]
pub struct Report {
    pub source_lang: String,
    pub target_lang: String,
    pub unit_count: usize,
    pub read: usize,
    pub imported: usize,
    pub added: usize,
    pub overwritten: usize,
    pub merged: usize,
    pub discarded: usize,
    pub errors: usize,
    pub bad: usize,
    pub duplicates: usize,
    error: Option<String>,
}

pub fn available() -> bool {
    studio_directory().is_ok()
}

fn studio_directory() -> Result<PathBuf> {
    if !cfg!(windows) {
        return Err(CatError::Invalid(
            "SDLTM requiere el puente opcional de Trados Studio para Windows".into(),
        ));
    }
    let directory = std::env::var_os("LUMENCAT_TRADOS_SDK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into()),
            )
            .join("Trados/Trados Studio/Studio19")
        });
    for name in [
        "Sdl.Core.Globalization.dll",
        "Sdl.LanguagePlatform.Core.dll",
        "Sdl.LanguagePlatform.TranslationMemory.dll",
        "Sdl.LanguagePlatform.TranslationMemoryApi.dll",
    ] {
        if !directory.join(name).is_file() {
            return Err(CatError::Invalid("SDLTM requiere Trados Studio instalado/licenciado; para una ruta distinta configura LUMENCAT_TRADOS_SDK_DIR. Puedes usar TMX sin Trados.".into()));
        }
    }
    Ok(directory)
}

fn snapshot(source: &Path, directory: &Path, cancel: &Cancellation) -> Result<PathBuf> {
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let is_memory: bool = connection
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='translation_memories'")?
        .exists([])?;
    if !is_memory {
        return Err(CatError::Format(
            "el archivo no es una memoria SDLTM reconocida".into(),
        ));
    }
    let backup = migrations::backup(&connection, directory, "sdltm-snapshot-", cancel)?;
    let working = directory.join("working.sdltm");
    backup
        .into_temp_path()
        .persist(&working)
        .map_err(|error| CatError::Io(error.error))?;
    fs::write(directory.join(".lumencat-sdltm-workspace"), [])?;
    Ok(working)
}

fn bridge(
    action: &str,
    working: &Path,
    tmx: &Path,
    sl: &str,
    tl: &str,
    cancel: &Cancellation,
) -> Result<Report> {
    cancel.check()?;
    let directory = working
        .parent()
        .ok_or_else(|| CatError::Invalid("copia SDLTM sin directorio".into()))?;
    let script = directory.join("bridge.ps1");
    let report = directory.join("report.json");
    fs::write(&script, SCRIPT)?;
    let powershell =
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(script)
        .arg("-Action")
        .arg(action)
        .arg("-StudioDirectory")
        .arg(studio_directory()?)
        .arg("-WorkingMemory")
        .arg(working)
        .arg("-Tmx")
        .arg(tmx)
        .arg("-Report")
        .arg(&report)
        .arg("-SourceLanguage")
        .arg(sl)
        .arg("-TargetLanguage")
        .arg(tl)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn()?;
    let status = loop {
        if let Err(error) = cancel.check() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => (),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    cancel.check()?;
    let report: Report = serde_json::from_slice(&fs::read(report).map_err(|_| CatError::Invalid("El puente Trados terminó sin informe; revisa la instalación y el permiso de ejecutar scripts locales".into()))?).map_err(|_| CatError::Invalid("informe del puente Trados inválido".into()))?;
    if report.error.as_deref() == Some("language_mismatch") {
        return Err(CatError::Invalid(format!(
            "La SDLTM declara {}→{}; se solicitó {sl}→{tl}. Configura el par exacto antes de importar o actualizar.",
            report.source_lang, report.target_lang
        )));
    }
    if !status.success() || report.error.is_some() {
        return Err(CatError::Invalid(format!(
            "El SDK Trados rechazó la operación ({}): {} leídas, {} importadas, {} descartadas, {} duplicadas, {} errores y {} inválidas. No se publicó ningún destino ni se modificó el original",
            report.error.as_deref().unwrap_or("sdk_api_failed"),
            report.read,
            report.imported,
            report.discarded,
            report.duplicates,
            report.errors,
            report.bad
        )));
    }
    Ok(report)
}

pub fn import(
    source: &Path,
    sl: &str,
    tl: &str,
    cancel: &Cancellation,
    sink: impl FnMut(TmUnit) -> Result<()>,
) -> Result<usize> {
    studio_directory()?;
    let directory = tempfile::tempdir()?;
    let working = snapshot(source, directory.path(), cancel)?;
    let tmx = directory.path().join("export.tmx");
    let report = bridge("export", &working, &tmx, sl, tl, cancel)?;
    let count = super::import_tmx(BufReader::new(fs::File::open(tmx)?), sl, tl, cancel, sink)?;
    if count != report.unit_count {
        return Err(CatError::Invalid(format!(
            "SDLTM exportó {} unidades, pero se leyeron {count}; importación rechazada",
            report.unit_count
        )));
    }
    Ok(count)
}

pub fn update(
    source: &Path,
    destination: &Path,
    tmx: &Path,
    sl: &str,
    tl: &str,
    cancel: &Cancellation,
) -> Result<Report> {
    studio_directory()?;
    if destination.exists() {
        return Err(CatError::Invalid(
            "El destino SDLTM ya existe; elige un archivo nuevo. El original nunca se sobrescribe."
                .into(),
        ));
    }
    let expected_read =
        super::import_tmx(BufReader::new(fs::File::open(tmx)?), sl, tl, cancel, |_| {
            Ok(())
        })?;
    if expected_read == 0 {
        return Err(CatError::Invalid(
            "El TMX no contiene unidades del par SDLTM solicitado".into(),
        ));
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = tempfile::tempdir_in(parent)?;
    let working = snapshot(source, directory.path(), cancel)?;
    let report = bridge("update", &working, tmx, sl, tl, cancel)?;
    if report.read != expected_read {
        return Err(CatError::Invalid(
            "El SDK no leyó todas las unidades preparadas; no se publicó la copia".into(),
        ));
    }
    {
        let connection = Connection::open_with_flags(&working, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        let count: usize =
            connection.query_row("SELECT count(*) FROM translation_units", [], |row| {
                row.get(0)
            })?;
        if integrity != "ok" || count != report.unit_count {
            return Err(CatError::Invalid(
                "La copia SDLTM no superó integridad/conteo; no se publicó".into(),
            ));
        }
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut fs::File::open(&working)?, temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    cancel.check()?;
    temporary
        .persist_noclobber(destination)
        .map_err(|error| CatError::Io(error.error))?;
    Ok(report)
}
