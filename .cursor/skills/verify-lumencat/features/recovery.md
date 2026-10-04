## Sub-features

- `recovery.unclean-open`: reabrir tras cierre abrupto marca `recovered`, valida integridad SQLite y avisa en el mensaje de estado (legacy `src/app.rs:266`; GPUI `src/gpui_app/mod.rs:711-725`).
- `recovery.close-gate`: el cierre espera el commit; un error de guardado impide cerrar y conserva el borrador (legacy `src/app.rs:473-487`; GPUI con `save_error` y estado rojo, `src/gpui_app/mod.rs:2373-2382`).
- `recovery.draft-recovery-legacy`: diálogo legacy «Recuperación del borrador pendiente» (TXT nuevo o descarte explícito autorizado; `src/app.rs:619-631` y `758-778`, `Task::RecoverText` en `src/worker.rs:207`). GPUI no tiene este diálogo: el borrador permanece en el editor con estado de error.
- `recovery.single-instance`: bloqueo exclusivo por proyecto con `.lumencat-lock` (`src/storage.rs:46-47`); una segunda sesión es rechazada.
- `recovery.foreign-future-rejection`: una base ajena o de versión futura se rechaza sin modificarla, por `application_id`+`user_version` en la apertura (`src/storage.rs:60-61`; test `refuses_foreign_and_future_schema_without_modification`).
- `recovery.migration-backup` / `recovery.migration-rollback`: apertura de v1–v4 migra a v5 con respaldo coherente `<archivo>.backup-v<versión>-<id>.lcat`; fallo revierte la transacción y conserva el respaldo (`src/storage/migrations.rs`). No hay migración hacia atrás.
- `recovery.backup-api`: `ProjectStore::backup_to`, `recover_copy` y `open_existing` existen en `src/storage.rs`; rechazan destinos existentes y validan integridad. Sin diálogos GUI de respaldo/recuperación en este corte.

## How to get to it (user POV)

Lo observable en la app: al abrir un proyecto con cierre no limpio, el mensaje de estado informa «Se recuperó un cierre no limpio. SQLite validó la base; revisa el último segmento» (legacy, `src/app.rs:266`) o el equivalente GPUI (`src/gpui_app/mod.rs:711-725`); una base ajena o futura no abre y no es modificada.

Recuperación de borrador (solo legacy): si un guardado falla, la barra inferior ofrece **Reintentar guardado pendiente** y **Recuperar borrador / recargar desde disco…**. El diálogo guarda el texto completo del editor en un TXT nuevo («Guardar borrador completo en archivo nuevo»; el proyecto conserva los cambios pendientes) o, marcando la casilla de autorización, permite «Confirmar descarte y recargar desde disco». GPUI no tiene este diálogo: el borrador permanece en el editor y la bolita de estado pasa a roja «Save Error»; el cierre de ventana queda bloqueado mientras haya guardado fallido («Guardando antes de cerrar…»).

Negativos esperables: abrir el mismo proyecto dos veces falla (lock exclusivo, `src/storage.rs:46-47`); abrir una base ajena o de versión futura falla sin modificarla.

## Driving it with cua-driver

Nivel de prueba por sub-feature; conserva separados los resultados históricos y la build actual:

- `unclean-open`, `foreign-future-rejection` y durabilidad de historial/exportación: **CORE PASS**. `cargo test --locked --test recovery --test storage` → 5/5 (log: `logs/tests/recovery-20261003-core.log`). `tests/recovery.rs` lanza el binario auxiliar `recovery_probe` (compila con el crate vía `CARGO_BIN_EXE`), lo mata con una transacción abierta y comprueba reapertura con `recovered=true`, conservación del commit confirmado, descarte de la transacción incompleta, undo/redo durable, exportación del destino confirmado y original intacto; la reapertura limpia vuelve a `recovered=false`. `tests/storage.rs` cubre rechazo de base ajena/futura sin modificación, rollback de importación cancelada, undo/redo con revisión y lock, y aislamiento Unicode.
- `recovery_probe` **no tiene `--help`**: exige un directorio como primer argumento (`src/bin/recovery_probe.rs:8-9`) y se queda vivo esperando que el padre lo mate; el contrato se ejercita desde `tests/recovery.rs`, no desde CLI. `cargo run --locked --bin recovery_probe -- --help` termina en error de ruta (esperado; log: `logs/tests/recovery-20261003-probe-help.log`).
- Cierre limpio y persistencia: **GUI PASS histórico** (3 de octubre, `e2e-rc-002`/`e2e-rc-001`). No prueba `close-gate` ante guardado fallido: ese negativo no se condujo. Consulta sus `results.md` sin atribuirlos a la build actual.
- `draft-recovery-legacy`: **GUI**, no ejecutada en esta entrega: forzar un fallo de guardado requiere condiciones no inducibles sin instrumentación; documenta el diálogo con snapshots solo si el coordinador lo autoriza, sin fabricar errores.
- `single-instance`: sin ejecución GUI dedicada esta entrega; el controlador ya impide un segundo `launch` sobre el mismo proyecto y el lock está cubierto por la apertura exclusiva del archivo.
- Corte `524046f`, 4 de octubre: `cargo test --locked --lib migration_tests` → 3/3; `--test storage --test replacement --test recovery` → 9/9, incluidos respaldo con WAL y recuperación sin sobrescribir. CORE PASS, logs `logs/tests/audit-trados-{core,migrations}.log`; no acredita migración por GUI.
- Persistencia GUI actual: `audit-main-gpui-20261004` cerró/reabrió por controlador y conservó traducciones, estados, idiomas, TM y terminología. Legacy `audit-main-legacy-20261004` cerró/reabrió limpio, pero sin importar ni editar por bloqueo de transporte. No se indujo guardado fallido ni interrupción GUI.

## Gotchas

- No copies solo el `.lcat` durante una sesión activa: WAL/SHM deben acompañar a la base. `close()` hace `wal_checkpoint(TRUNCATE)` y marca `clean=1` (`src/storage.rs:437-448`); el flag `recovered` solo aplica en bases con versión > 0.
- No hay migración hacia atrás; una base de versión futura se rechaza. Para volver a un binario anterior recupera una copia NUEVA del respaldo de su versión; revertir código no degrada el proyecto.
- El lock es por proceso (`src/storage.rs:46-47`): un `launch` duplicado sobre el mismo proyecto falla de forma esperada; usa un `RunId` (y proyecto) nuevo por instancia.
- La recuperación de borrador por TXT (`Task::RecoverText`, `src/worker.rs:207`) usa temporal + `persist_noclobber`: nunca sobrescribe y un borrador >1 MiB se guarda completo pese al límite persistido (es escape, no excepción de regla).
- GPUI sin diálogo de recuperación no es un bug pendiente de «arreglar» por verificación: es la frontera declarada del corte. Reporta huecos con `archivo:línea`.
- Una falla de hardware/SO puede perder texto aún no comprometido; los commits confirmados son durables dentro de garantías del SO. No hay reparador de corrupción ni backups automáticos programados.
- Consulta [RECUPERACION](../../../../docs/guides/RECUPERACION.md) antes de abrir bases anteriores; no copies solo el `.lcat` con la sesión activa para fabricar un respaldo.
