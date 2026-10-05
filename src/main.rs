#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    let log_root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LumenCAT/logs/app");
    if let Err(error) = initialize_logging(&log_root) {
        // El diagnóstico no debe impedir abrir el trabajo ni imprimir contenido del documento.
        eprintln!("No se pudo inicializar el registro local: {error}");
    }
    tracing::info!(
        event = "application_start",
        version = env!("CARGO_PKG_VERSION")
    );
    let args: Vec<_> = std::env::args().skip(1).collect();
    let project = args
        .windows(2)
        .find(|pair| pair[0] == "--project")
        .map(|pair| pair[1].clone());
    let settings_directory = args
        .windows(2)
        .find(|pair| pair[0] == "--settings-dir")
        .map(|pair| std::path::PathBuf::from(&pair[1]));
    lumencat::gpui_app::runtime::run(project, settings_directory);
}

fn initialize_logging(
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(directory)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let file = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("{timestamp}-{}.jsonl", std::process::id())))?;
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(file)
        .with_max_level(
            if std::env::var_os("LUMENCAT_DIAGNOSTICS").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            },
        )
        .try_init()?;
    Ok(())
}
