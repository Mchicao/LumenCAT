## Sub-features

- `recovery.unclean-open`: reabrir tras cierre abrupto marca `recovered`, valida integridad SQLite y avisa en el mensaje de estado (legacy `src/app.rs:266`; GPUI `src/gpui_app/mod.rs:711-725`).
- `recovery.close-gate`: el cierre espera el commit; un error de guardado impide cerrar y conserva el borrador (legacy `src/app.rs:473-487`; GPUI con `save_error` y estado rojo, `src/gpui_app/mod.rs:2373-2382`).
- `recovery.draft-recovery-legacy`: diálogo legacy «Recuperación del borrador pendiente» (TXT nuevo o descarte explícito autorizado; `src/app.rs:619-631` y `758-778`, `Task::RecoverText` en `src/worker.rs:207`). GPUI no tiene este diálogo: el borrador permanece en el editor con estado de error.
- `recovery.single-instance`: bloqueo exclusivo por proyecto con `.lumencat-lock` (`src/storage.rs:46-47`); una segunda sesión es rechazada.
- `recovery.foreign-future-rejection`: una base ajena o de versión futura se rechaza sin modificarla, por `application_id`+`user_version` en la apertura (`src/storage.rs:60-61`; test `refuses_foreign_and_future_schema_without_modification`).
- `recovery.migration-backup` / `recovery.migration-rollback`: **NO implementadas**. No hay migraciones (solo esquema v1) ni respaldo automático `<archivo>.backup-v<versión>-<id>.lcat`: la receta anterior describía API y tests inexistentes. Respaldo coherente + migraciones es el paquete **F00** de [`docs/technical/BRECHAS_TRADOS.md`](../../../../docs/technical/BRECHAS_TRADOS.md), pendiente. No verifiques migraciones ni inventes botones.
- `recovery.backup-api`: **retirada**. `ProjectStore::backup_to` y `recover_copy` no existen en `src/storage.rs`; tampoco el test `backup_includes_wal_and_recovery_never_overwrites`. Lo que sí existe: `close()` hace `wal_checkpoint(TRUNCATE)` y marca `clean=1` (`src/storage.rs:437-448`), la apertura expone `recovered` (`src/storage.rs:28,81-85`) y exportación/recuperación de borrador con `persist_noclobber` (`src/storage.rs:433`), cubiertas por `tests/recovery.rs`.

## How to get to it (user POV)

Lo observable en la app: al abrir un proyecto con cierre no limpio, el mensaje de estado informa «Se recuperó un cierre no limpio. SQLite validó la base; revisa el último segmento» (legacy, `src/app.rs:266`) o el equivalente GPUI (`src/gpui_app/mod.rs:711-725`); una base ajena o futura no abre y no es modificada.

Recuperación de borrador (solo legacy): si un guardado falla, la barra inferior ofrece **Reintentar guardado pendiente** y **Recuperar borrador / recargar desde disco…**. El diálogo guarda el texto completo del editor en un TXT nuevo («Guardar borrador completo en archivo nuevo»; el proyecto conserva los cambios pendientes) o, marcando la casilla de autorización, permite «Confirmar descarte y recargar desde disco». GPUI no tiene este diálogo: el borrador permanece en el editor y la bolita de estado pasa a roja «Save Error»; el cierre de ventana queda bloqueado mientras haya guardado fallido («Guardando antes de cerrar…»).

Negativos esperables: abrir el mismo proyecto dos veces falla (lock exclusivo, `src/storage.rs:46-47`); abrir una base ajena o de versión futura falla sin modificarla.

## Driving it with cua-driver

Nivel de prueba por sub-feature y resultado de la ejecución del 3 de octubre de 2026:

- `unclean-open`, `foreign-future-rejection` y durabilidad de historial/exportación: **CORE PASS**. `cargo test --locked --test recovery --test storage` → 5/5 (log: `logs/tests/recovery-20261003-core.log`). `tests/recovery.rs` lanza el binario auxiliar `recovery_probe` (compila con el crate vía `CARGO_BIN_EXE`), lo mata con una transacción abierta y comprueba reapertura con `recovered=true`, conservación del commit confirmado, descarte de la transacción incompleta, undo/redo durable, exportación del destino confirmado y original intacto; la reapertura limpia vuelve a `recovered=false`. `tests/storage.rs` cubre rechazo de base ajena/futura sin modificación, rollback de importación cancelada, undo/redo con revisión y lock, y aislamiento Unicode.
- `recovery_probe` **no tiene `--help`**: exige un directorio como primer argumento (`src/bin/recovery_probe.rs:8-9`) y se queda vivo esperando que el padre lo mate; el contrato se ejercita desde `tests/recovery.rs`, no desde CLI. `cargo run --locked --bin recovery_probe -- --help` termina en error de ruta (esperado; log: `logs/tests/recovery-20261003-probe-help.log`).
- `close-gate` y persistencia tras reopen: **GUI PASS** (3 de octubre de 2026). GPUI (`e2e-rc-002`): importar TXT por diálogo → escribir destino → «Confirmar y avanzar» (fila 1 `confirmed`) → cleanup por UIA «Cerrar» → `-Action reopen` → fila 1 sigue `confirmed` con «Aviso de privacidad 42.» y sin aviso de recuperación (`output/verification/e2e-rc-002/results.md`). Legacy (`e2e-rc-001`): cierre limpio por click UIA y reapertura sin aviso (`output/verification/e2e-rc-001/results.md`).
- `draft-recovery-legacy`: **GUI**, no ejecutada en esta entrega: forzar un fallo de guardado requiere condiciones no inducibles sin instrumentación; documenta el diálogo con snapshots solo si el coordinador lo autoriza, sin fabricar errores.
- `single-instance`: sin ejecución GUI dedicada esta entrega; el controlador ya impide un segundo `launch` sobre el mismo proyecto y el lock está cubierto por la apertura exclusiva del archivo.
- Persistencia entre ejecuciones: `-Action reopen` reutiliza el proyecto del `RunId` sin borrar evidencia; es la vía aceptada para comprobar durabilidad desde la GUI.

## Gotchas

- No copies solo el `.lcat` durante una sesión activa: WAL/SHM deben acompañar a la base. `close()` hace `wal_checkpoint(TRUNCATE)` y marca `clean=1` (`src/storage.rs:437-448`); el flag `recovered` solo aplica en bases con versión > 0.
- No hay migración hacia atrás ni hacia adelante: no existe camino de migraciones; una base de otra versión se rechaza sin modificarla. El respaldo antes de migrar (F00) está pendiente: si necesitas «volver», recupera una copia externa hecha con la instancia cerrada.
- El lock es por proceso (`src/storage.rs:46-47`): un `launch` duplicado sobre el mismo proyecto falla de forma esperada; usa un `RunId` (y proyecto) nuevo por instancia.
- La recuperación de borrador por TXT (`Task::RecoverText`, `src/worker.rs:207`) usa temporal + `persist_noclobber`: nunca sobrescribe y un borrador >1 MiB se guarda completo pese al límite persistido (es escape, no excepción de regla).
- GPUI sin diálogo de recuperación no es un bug pendiente de «arreglar» por verificación: es la frontera declarada del corte. Reporta huecos con `archivo:línea`.
- Una falla de hardware/SO puede perder texto aún no comprometido; los commits confirmados son durables dentro de garantías del SO. No hay reparador de corrupción ni backups automáticos programados.
- `docs/guides/RECUPERACION.md`, citado por la receta anterior, no existe en el árbol; no lo referencies hasta que se cree.
