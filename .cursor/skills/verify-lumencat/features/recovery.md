## Sub-features

- `recovery.unclean-open`: reabrir tras cierre abrupto marca `recovered`, valida integridad SQLite y avisa en el mensaje de estado GPUI (`src/gpui_app/mod.rs`).
- `recovery.close-gate`: Salir y la X muestran confirmación; Guardar y salir espera el commit. Un error conserva ventana/borrador y permite reintentar, cancelar o descartar solo lo pendiente (`src/gpui_app/exit.rs`).
- `recovery.single-instance`: bloqueo exclusivo por proyecto con `.lumencat-lock` (`src/storage.rs:46-47`); una segunda sesión es rechazada.
- `recovery.foreign-future-rejection`: una base ajena o de versión futura se rechaza sin modificarla, por `application_id`+`user_version` en la apertura (`src/storage.rs:60-61`; test `refuses_foreign_and_future_schema_without_modification`).
- `recovery.migration-backup` / `recovery.migration-rollback`: apertura de v1–v4 migra a v5 con respaldo coherente `<archivo>.backup-v<versión>-<id>.lcat`; fallo revierte la transacción y conserva el respaldo (`src/storage/migrations.rs`). No hay migración hacia atrás.
- `recovery.backup-api`: `ProjectStore::backup_to`, `recover_copy` y `open_existing` existen en `src/storage.rs`; rechazan destinos existentes y validan integridad. Sin diálogos GUI de respaldo/recuperación en este corte.

## How to get to it (user POV)

Lo observable en la app: abrir un proyecto con cierre no limpio muestra el aviso de recuperación GPUI (`src/gpui_app/mod.rs`); una base ajena o futura no abre y no es modificada.

Si un guardado falla, el borrador permanece en el editor y el estado pasa a «Save Error». Al salir, Guardar y salir reintenta; si falla, el diálogo muestra el error y la ventana sigue abierta. Cancelar conserva la sesión. Salir sin guardar descarta solo lo que no llegó a guardarse; el autoguardado se pausa durante la decisión y los commits previos no se revierten. No hay exportación GUI del borrador a TXT para recuperación.

Negativos esperables: abrir el mismo proyecto dos veces falla (lock exclusivo, `src/storage.rs:46-47`); abrir una base ajena o de versión futura falla sin modificarla.

## Driving it with cua-driver

Nivel de prueba por sub-feature; conserva separados los resultados históricos y la build actual:

- `unclean-open`, `foreign-future-rejection` y durabilidad de historial/exportación: **CORE PASS**. `cargo test --locked --test recovery --test storage` → 5/5 (log: `logs/tests/recovery-20261003-core.log`). `tests/recovery.rs` lanza el binario auxiliar `recovery_probe` (compila con el crate vía `CARGO_BIN_EXE`), lo mata con una transacción abierta y comprueba reapertura con `recovered=true`, conservación del commit confirmado, descarte de la transacción incompleta, undo/redo durable, exportación del destino confirmado y original intacto; la reapertura limpia vuelve a `recovered=false`. `tests/storage.rs` cubre rechazo de base ajena/futura sin modificación, rollback de importación cancelada, undo/redo con revisión y lock, y aislamiento Unicode.
- `recovery_probe` **no tiene `--help`**: exige un directorio como primer argumento (`src/bin/recovery_probe.rs:8-9`) y se queda vivo esperando que el padre lo mate; el contrato se ejercita desde `tests/recovery.rs`, no desde CLI. `cargo run --locked --bin recovery_probe -- --help` termina en error de ruta (esperado; log: `logs/tests/recovery-20261003-probe-help.log`).
- Cierre limpio y persistencia: **GUI PASS histórico** (3 de octubre, `e2e-rc-002`). No prueba `close-gate` ante guardado fallido: ese negativo no se condujo. Consulta su `results.md` sin atribuirlo a la build actual.
- `single-instance`: sin ejecución GUI dedicada esta entrega; el controlador ya impide un segundo `launch` sobre el mismo proyecto y el lock está cubierto por la apertura exclusiva del archivo.
- Corte `524046f`, 4 de octubre: `cargo test --locked --lib migration_tests` → 3/3; `--test storage --test replacement --test recovery` → 9/9, incluidos respaldo con WAL y recuperación sin sobrescribir. CORE PASS, logs `logs/tests/audit-trados-{core,migrations}.log`; no acredita migración por GUI.
- Persistencia GUI auditada: `audit-main-gpui-20261004` cerró/reabrió por controlador y conservó traducciones, estados, idiomas, TM y terminología. No se indujo guardado fallido ni interrupción GUI.
- Archivo/cierre, 5 de octubre: `file-menu-final-20261005`, hash `D79F5A28…3186`, acredita **GUI PASS** para Salir/X, cancelación con borrador, guardado fallido por bloqueo temporal del proyecto desechable, reintento y reapertura, descarte y reapertura. SQLite se inspeccionó en modo read-only; conexiones de bloqueo no modificaron filas. Ver `output/verification/file-menu-final-20261005/results.md`. Las pruebas de núcleo actuales son 48; no se atribuyen estos recorridos a builds anteriores.

## Gotchas

- No copies solo el `.lcat` durante una sesión activa: WAL/SHM deben acompañar a la base. `close()` hace `wal_checkpoint(TRUNCATE)` y marca `clean=1` (`src/storage.rs:437-448`); el flag `recovered` solo aplica en bases con versión > 0.
- No hay migración hacia atrás; una base de versión futura se rechaza. Para volver a un binario anterior recupera una copia NUEVA del respaldo de su versión; revertir código no degrada el proyecto.
- El lock es por proceso (`src/storage.rs:46-47`): un `launch` duplicado sobre el mismo proyecto falla de forma esperada; usa un `RunId` (y proyecto) nuevo por instancia.
- La API del worker `Task::RecoverText` conserva su prueba de núcleo: temporal + `persist_noclobber`, sin sobrescritura y sin truncar borradores >1 MiB. No tiene un recorrido GUI y no se acredita como función de la interfaz.
- GPUI sin diálogo de recuperación no es un bug pendiente de «arreglar» por verificación: es la frontera declarada del corte. Reporta huecos con `archivo:línea`.
- Una falla de hardware/SO puede perder texto aún no comprometido; los commits confirmados son durables dentro de garantías del SO. No hay reparador de corrupción ni backups automáticos programados.
- Consulta [RECUPERACION](../../../../docs/guides/RECUPERACION.md) antes de abrir bases anteriores; no copies solo el `.lcat` con la sesión activa para fabricar un respaldo.
