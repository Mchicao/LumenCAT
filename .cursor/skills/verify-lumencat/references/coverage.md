# Cobertura y niveles de prueba

## Registra evidencia, no intención

| Estado | Evidencia mínima |
|---|---|
| GUI PASS | Entrada real en app, acción y capturas antes/después; resultado visible y efectos durables cuando proceda |
| CORE PASS | Comando real worker/API/parser y aserciones sobre datos/archivos; nunca afirmar GUI |
| FAIL | Expectativa concreta, resultado distinto y reproducción/artefacto |
| BLOCKED | Paso exacto, rechazo del transporte o dependencia ausente; no imputarlo automáticamente a la app |
| NOT RUN | Receta disponible sin ejecución actual |

Por cada subfunción identifica también las entradas no ejecutadas, casos negativos y límites. E2E parcial de una feature no prueba todo su archivo. La captura debe mostrar el texto/estado esperado; no basta que Cua acepte el comando. Un seed puede preparar un corpus, nunca acreditar edición/importación por el usuario.

## Checks auxiliares existentes

Desde la raíz del repo:

```powershell
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --bin lumencat --bin lumencat-gpui
git diff --check
```

Consulta las recetas para suites concretas (`storage`, `recovery`, `formats`, `docx`, `qa`, `replacement`, `search_cancel`, `tm_retrieval`, `tm_learning`, `terminology`) y `cargo test --locked --lib migration_tests`. `cargo test` no valida GPUI/egui, entrada física, Word ni Trados. Terminología local, aprendizaje y migraciones v1–v4→v5 existen en `524046f`; no confundas guías pendientes de actualización con ausencia de producto.

- `cargo run --locked --example verify_grid -- output/verification/grid-corpus-nuevo` crea 100.000 segmentos y confirma extremos vía API. Abre ese proyecto por GUI para medir la lista; prueba importación por diálogo por separado.
- Lee `scripts/utils/verify_docx.rs`, `verify_word.ps1` y `render_pdf.ps1` antes de usarlos. COM/Word y render son verificaciones del documento; no prueban importación desde GPUI por sí solos.
- No acredites TBX/MultiTerm/SDLXLIFF/paquetes ni proveedores pagados sin implementación, fixtures y autorización. Un bug del transporte Cua no debe "arreglarse" modificando código funcional de LumenCAT.

## Limpieza y riesgos

El controlador verifica PID, hora de inicio y ejecutable antes de actuar. Windows puede ocultar `Process.Path` y `CommandLine`; se recupera la ruta mediante `debug_window_info`, pero la línea de comandos permanece no verificada. La captura y los artefactos deben identificar el proyecto temporal.

`cleanup` intenta el botón nativo Cerrar. `-ForceStop` termina únicamente esa instancia si no cerró y pierde borradores; no es prueba de cierre durable. Si launch falla tras crear proceso, conserva `instance.json` y ejecuta cleanup antes del siguiente intento. Nunca mates por nombre ni borres evidencia.

## Mantener el mapa

Cuando cambien controles o capacidades, actualiza receta, índice y registro de ejecución. Usa `/maintain-verification-skill` para auditar la skill contra fuente y app; no conviertas resultados históricos en prueba de la build actual.
