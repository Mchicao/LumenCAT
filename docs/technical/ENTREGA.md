# Primera entrega LumenCAT — 30 septiembre 2026

## Implementado y verificado

El checkout actual está en `main` y sigue `origin/main` en `b9e188c` (`feat: initial commit LumenCAT`); los cambios de esta pasada permanecen locales y sin commit. Esta tarea no publicó cambios. Un paquete Rust con GUI egui/eframe nativa, sin servidor/Node/Python/JVM/WebView ni red. Biblioteca y ejecutable real, no scaffolding vacío.

Proyecto SQLite WAL/FULL, historial undo/redo persistido, revisiones optimistas, locks, editor/confirmación, caché virtualizada1024 filas, TMX textual streaming/exact/fuzzy/concordancia, búsqueda paginada cancelable, QA local, TXT/XLIFF1.2 textual/DOCX conservador, exporte nuevo atómico sin clobber. IA desactivada; providers/proposals diseñados en ADR, no implementados todavía.

23 pruebas aprobadas; fmt, Clippy all-targets con warnings denegados y release build aprobados (`logs/tests/validation-word-runs.log`). Se mata proceso con commit previo+transacción pendiente y se verifica recuperación/undo/export. DOCX ahora admite fragmentos y runs de formato idéntico, formato de párrafo, decodifica referencias numéricas y lee con límite estricto; incluye round-trip, formato mixto rechazado y controles XML ilegales. Investigación Luna high y revisión adversarial OpenCode GLM-5.3 Max con cambios aplicados y un falso positivo contrastado con W3C.

Benchmarks release10k/100k/1M/3M, conteos import/export exactos. En3M p95 exact0.028ms, fuzzy1.274ms, concordancia0.160ms, commit2.208ms; página128 de documento250k0.148ms. Ver BENCHMARKS para método, import/export, datos previos y límites.

## Artefactos

`output/lumencat.exe`7835136bytes (7.47MiB), `output/benchmark.exe`2564608bytes; SHA256 en `output/artifacts.json`. Guía `docs/guides/INICIO.md`, investigación `docs/technical/INVESTIGACION.md`, arquitectura/modelo/aceptación en `docs/architecture/`. CSV y reviews en output; ejemplos sintéticos en output/demo.

La caché local `.cache/target` se retiró después de validar y copiar EXEs con hashes verificados:1.5GiB recuperados en esta pasada; el workspace ocupa10.91MiB sin `.git`. Toolchain Rust1.97 no requirió actualización. Cargo.lock conserva reconstrucción; cachés globales y otros repositorios no se tocaron. Impeccable actualización solicitada instaló4.3.1 en Claude/Gemini/OpenCode, vínculo .agent dejó instrucciones para su fuente; no se hizo git update externo.

## Pendientes explícitos

Esta pasada no abrió/exportó con Microsoft Word real ni probó interacción/scroll/IME/RAM/startup de la GUI. No decir compatibilidad Word universal: solo DOCX Transitional con formato inline uniforme, estilos de párrafo y tablas simples; formato mixto y estructuras complejas se rechazan para evitar perder texto/formato. Falta corpus autorizado real, OOXML schema validation completa y fuzzing sostenido.

MVP profesional completo aún pendiente: runs/tags protegidos, DOCX complejo, TBX/terminología, múltiples TMs y sus permisos/metadata editables, reemplazos masivos, backups/migraciones futuras, providers IA y propuestas/diffs, SDLXLIFF/paquetes. Autosave conserva commits dentro de garantías OS/storage; texto pendiente/hardware defectuoso no tiene garantía absoluta. Corrupción se detecta y bloquea; reparador automático todavía no existe.

La primera entrega cumple investigación/diseño y slice implementado; no equivale a todos los milestones ni a release de producción.
