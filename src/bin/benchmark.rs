//! Benchmark reproducible de rutas reales; fixtures temporales se eliminan al salir.
use lumencat::{formats, model::*, qa, storage::ProjectStore};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

fn sample(mut operation: impl FnMut() -> Result<()>) -> Result<(f64, f64)> {
    let mut times = Vec::with_capacity(30);
    for _ in 0..30 {
        let start = Instant::now();
        operation()?;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    Ok((times[15], times[28]))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let count: usize = args
        .next()
        .unwrap_or_else(|| "10000".into())
        .parse()
        .map_err(|_| CatError::Invalid("cantidad benchmark inválida".into()))?;
    if count == 0 || count > 5_000_000 {
        return Err(CatError::Invalid("benchmark: 1 a 5 millones".into()));
    }
    let report = args
        .next()
        .ok_or_else(|| CatError::Invalid("usar benchmark COUNT output/report.csv".into()))?;
    let report_path = Path::new(&report);
    let parent = report_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut report = File::options()
        .write(true)
        .create_new(true)
        .open(report_path)?;
    let directory = tempfile::tempdir_in(parent)?;
    let path = directory.path();
    let token = Cancellation::default();
    let tmx_path = path.join("synthetic.tmx");
    {
        let mut out = BufWriter::new(File::create(&tmx_path)?);
        write!(
            out,
            "<tmx version=\"1.4\"><header creationtool=\"LumenCAT benchmark\" creationtoolversion=\"1\" segtype=\"sentence\" o-tmf=\"LumenCAT\" adminlang=\"en\" srclang=\"en\" datatype=\"PlainText\"/><body>"
        )?;
        for id in 0..count {
            write!(
                out,
                "<tu tuid=\"{id}\"><tuv xml:lang=\"en\"><seg>Order {id} ready for delivery</seg></tuv><tuv xml:lang=\"es\"><seg>Pedido {id} listo para entrega</seg></tuv></tu>"
            )?;
        }
        write!(out, "</body></tmx>")?;
        out.flush()?;
    }
    writeln!(report, "operation,count,p50_ms,p95_ms,notes")?;
    let started = Instant::now();
    let db_path = path.join("benchmark.db");
    let mut store = ProjectStore::open(&db_path)?;
    writeln!(
        report,
        "create_project,{count},{:.3},,synthetic",
        started.elapsed().as_secs_f64() * 1000.0
    )?;
    let started = Instant::now();
    let imported = store.import_tmx(&tmx_path, "en", "es", &token)?;
    if imported != count {
        return Err(CatError::Invalid("conteo import TM distinto".into()));
    }
    writeln!(
        report,
        "import_tmx,{count},{:.3},,count_verified",
        started.elapsed().as_secs_f64() * 1000.0
    )?;
    let query = format!("Order {} ready for delivery", count / 2);
    let (p50, p95) = sample(|| {
        let matches = store.matches(&query, "en", "es")?;
        if !matches.iter().any(|m| m.exact && m.source == query) {
            return Err(CatError::Invalid("exact missing".into()));
        }
        Ok(())
    })?;
    writeln!(
        report,
        "exact_tm,{count},{p50:.3},{p95:.3},raw_exact_verified"
    )?;
    let fuzzy_query = format!("Order {} ready for dispatch", count / 2);
    let (p50, p95) = sample(|| {
        let _ = store.matches(&fuzzy_query, "en", "es")?;
        Ok(())
    })?;
    writeln!(
        report,
        "fuzzy_tm,{count},{p50:.3},{p95:.3},approximate_candidates"
    )?;
    let matches = store.matches(&fuzzy_query, "en", "es")?;
    writeln!(
        report,
        "fuzzy_fixture_recall,{count},{},,top8_contains_expected",
        usize::from(matches.iter().any(|m| m.source == query))
    )?;
    let (p50, p95) = sample(|| {
        let _ = store.concordance(&format!("{}", count / 2), "en", "es")?;
        Ok(())
    })?;
    writeln!(
        report,
        "concordance,{count},{p50:.3},{p95:.3},unique_numeric_token"
    )?;
    let txt = path.join("document.txt");
    {
        let mut out = BufWriter::new(File::create(&txt)?);
        for id in 0..count.min(250_000) {
            writeln!(out, "Line {id} has 42 items.")?;
        }
        out.flush()?;
    }
    let started = Instant::now();
    let doc = formats::import_document(&txt, "en", "es", &token)?;
    let doc_id = store.import_document(&doc, &token)?;
    writeln!(
        report,
        "import_document,{},{:.3},,includes_parse_sqlite",
        doc.segments.len(),
        started.elapsed().as_secs_f64() * 1000.0
    )?;
    let (p50, p95) = sample(|| {
        let _ = store.page(doc_id, doc.segments.len() / 2, 128, "")?;
        Ok(())
    })?;
    writeln!(
        report,
        "document_page,{},{p50:.3},{p95:.3},128_rows",
        doc.segments.len()
    )?;
    let segment = store
        .page(doc_id, 0, 1, "")?
        .into_iter()
        .next()
        .ok_or_else(|| CatError::Invalid("sin segmento".into()))?;
    let mut revision = segment.revision;
    let (p50, p95) = sample(|| {
        let updated = store.edit(&EditCommand {
            segment_id: segment.id,
            expected_revision: revision,
            target: format!("Línea {revision} con 42 artículos."),
            state: SegmentState::Draft,
            locked: false,
            origin: Origin::Human,
        })?;
        revision = updated.revision;
        Ok(())
    })?;
    writeln!(
        report,
        "autosave_commit,{count},{p50:.3},{p95:.3},WAL_FULL_history"
    )?;
    let started = Instant::now();
    let mut issues = 0;
    for segment in &doc.segments {
        issues += qa::check(&segment.source, &segment.target, false).len();
    }
    writeln!(
        report,
        "qa_document,{},{:.3},,issues_{issues}",
        doc.segments.len(),
        started.elapsed().as_secs_f64() * 1000.0
    )?;
    let (p50, p95) = sample(|| {
        let _ = store.page(doc_id, 0, 128, "42 items")?;
        Ok(())
    })?;
    writeln!(
        report,
        "document_search,{},{p50:.3},{p95:.3},substring_initial_page",
        doc.segments.len()
    )?;
    let started = Instant::now();
    let exported = store.export_tm(&path.join("export.tmx"), &token)?;
    if exported != count {
        return Err(CatError::Invalid("conteo export TM distinto".into()));
    }
    writeln!(
        report,
        "export_tmx,{count},{:.3},,count_verified",
        started.elapsed().as_secs_f64() * 1000.0
    )?;
    store.close()?;
    drop(store);
    writeln!(
        report,
        "database_bytes,{count},{},,after_checkpoint",
        std::fs::metadata(&db_path)?.len()
    )?;
    let (p50, p95) = sample(|| {
        let mut reopened = ProjectStore::open(&db_path)?;
        let _ = reopened.documents()?;
        reopened.close()?;
        Ok(())
    })?;
    writeln!(
        report,
        "open_project,{count},{p50:.3},{p95:.3},warm_filesystem"
    )?;
    report.sync_all()?;
    Ok(())
}
