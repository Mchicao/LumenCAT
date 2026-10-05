# Construir el primer flujo LumenCAT

## Purpose / Big Picture

Traducir offline un documento textual, recuperar trabajo y exportar sin modificar entrada. Esta primera entrega construye un slice, no todas las funciones profesionales.

## Progress

- [x] (2026-09-30) Inspección Rust/OpenCode y dos investigaciones Luna high.
- [x] (2026-09-30) Investigación y arquitectura mínima registradas antes de implementar.
- [x] (2026-09-30) Formatos TXT/XLIFF textual/DOCX restringido, storage/TM, worker y UI con ownership separado.
- [x] (2026-09-30) Tres revisiones adversariales OpenCode GLM5.3 max completadas; primera arquitectónica incompleta por restricción de lectura fuera repo. Correcciones integradas y verificadas.
- [x] (2026-09-30) fmt/clippy/tests/build:21 pruebas aprobadas, release; benchmarks10k/100k/1M/3M con conteos comprobados y fixtures retirados.
- [x] (2026-09-30) Resultados, límites, guía y memoria del proyecto actualizados. EXE preservado y SHA256 comprobado; cargo clean solo target del proyecto.
- [ ] Validación real UI/Word, corpus autorizado, fuzzing/migraciones futuras y ampliación tags/runs antes de declarar MVP profesional listo.

## Context and Orientation

Repo nuevo `C:/Proyectos/LumenCAT`. Un paquete Rust con `src/`, `tests/`, documentación funcional, `output/`, logs y cache. Sin repo remoto ni commit/push autorizados.

## Plan of Work

Leer INVESTIGACION/MVP/DECISIONES. Definir tipos compartidos primero. Implementar parsers/serializadores y storage en paralelo con ownership exclusivo. Integrar worker de DB y UI GPUI. Ejecutar workflow de import/edit/undo/recovery/export con tests de integración; benchmarking independiente. Revisar con GLM en modo lectura.

## Concrete Steps

Desde repo: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --locked --release`; luego binario benchmark a rutas nuevas en output. GUI `cargo run --locked --bin lumencat` con proyecto local nuevo.

## Validation and Acceptance

Los criterios de `docs/architecture/MVP.md` deben tener evidencia o estado pendiente. Tests prueban límites de datos y rutas reales, no getters/scaffolding. Sin permiso de computer use no afirmar scrolling ni pruebas visuales reales. Un build no demuestra UX.

## Idempotence and Recovery

No sobrescribir outputs ni inputs. Operaciones canceladas hacen rollback; no borrar WAL. DB futura/corrupta rechazada. Migraciones con checkpoint/backup coherente. No tocar daily-driver channels.

## Interfaces and Dependencies

Un paquete, módulos model/formats/storage/tm/qa/worker/gpui_app. Contratos detallados en DECISIONES. Std concurrencia; SQLite, quick-xml, GPUI y dependencias enfocadas, lockfile fijado.

## Surprises & Discoveries

Interoperar con XLIFF genérico no preserva necesariamente sabores de proveedor. Verbalis no debe rotularse OSI sin revisar licencia. No hay corpus real autorizado disponible aún.

Recuperación OR perdió candidato raro; test rojo→verde y benchmark mostraron corrección. Join SQL hacía concordancia10k p95750ms; CROSS JOIN FTS redujo a0.055ms. Frecuencias exactas escalaban fuzzy3M226ms; conteo saturado vía vocab instance llevó a1.274ms. Windows bloqueó un rebuild de benchmark.exe mientras benchmark estaba ejecutándose; se repitió build al terminar, sin cambios destructivos.

## Decision Log

Primer corte restringe formatos sin tags con rechazo explícito; tags protegidos preceden compatibilidad ampliada. LumenCAT nombre provisional. Único paquete hasta demostrar frontera.

## Outcomes & Retrospective

Primera entrega construida: núcleo local y GUI compilada,21 checks de comportamiento y benchmark sintético hasta3M. Ejecutable final7,836,160bytes, proyecto tras limpiar11,406,952bytes (antes de estas últimas notas). Cache retirada1,720,247,728bytes; reconstrucción locked disponible. No se probó interacción visual, RAM idle, cold startup ni abrir en Word real por restricción computer use. No afirmar producto profesional completo. DOCX M2 necesita runs/tags protegidos y corpus Word real; IA/TBX/múltiplesTMs/SDL quedan roadmap.

## Artifacts and Notes

Fuentes en INVESTIGACION; revisiones en logs/swarm; resultados reproducibles en output.

Evidencia final `logs/tests/validation-final.log`, `output/artifacts.json`, `output/build-footprint-final.json`, benchmarks finales y `docs/technical/ENTREGA.md`. Repo local main sin commits/remotos. La limpieza retiró los artefactos cargo tras verificar; EXEs entregados no necesitan esa caché.
