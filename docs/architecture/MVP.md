# MVP vertical y criterios de aceptación

Nombre provisional LumenCAT. Prioridad: correctness, trabajo humano, latencia, robustez, interoperabilidad, UX. Es una aplicación nativa local, sin IA ni red en el primer corte.

## Entrega inicial y prioridades

P0: crear/abrir proyecto SQLite local; importar TXT UTF-8 y XLIFF 1.2 textual; editor activo, lista virtualizada y panel TM; editar/confirmar/lock, undo/redo; autosave durable visible; recuperación de cierre abrupto; exportar a nombre nuevo; QA local básico. P1: TMX textual streaming, exact/fuzzy, búsqueda. No declarar MVP profesional completo antes de pruebas reales UI y corpus de formatos. No usar soporte parcial como promesa de round-trip universal.

M1 termina con ese flujo, DOCX básico conservador y benchmark 10k/100k/1M. DOCX se adelanta por prioridad Word solicitada explícitamente durante esta sesión. M2: DOCX con runs/tags protegidos y corpus Word amplio, TBX dialecto explícito, términos preferred/forbidden, concordancia avanzada, filtros, memorias independientes con prioridades/penalties, QA tags y reemplazo masivo undoable. M3: providers OpenAI-compatible/Ollama primero, luego Anthropic/Gemini, preview contexto, políticas privacidad, diff y aceptación humana. M4: SDLXLIFF y paquetes si evidencia legal/técnica lo permite, XLIFF2, otros documentos. Spellcheck/preview/plugins/sync quedan fuera hasta núcleo fiable.

## Modelo de datos

`project_meta` contiene lenguas y versión; `documents` conserva envelope original, formato e identificación; `segments` registra documento, ordinal, source/target, estado, bloqueo, origen y revisión; `history` before/after/cursor global transaccional; `tm_units` source/target/idiomas/TU original e índices; `tm_fts` texto indexado; `session` cierre limpio y recuperación. Datos portables de proyecto separados de logs/credenciales locales. Schema versionado y application_id, fuentes jamás mutadas. Referencias por IDs, no posiciones efímeras de UI.

## UX

Traductor en escritorio iluminado durante horas: tema claro neutro de alto contraste, acento teal sobrio, tipografía legible escalable y estados escritos además de color. Barra proyecto/import/export/búsqueda; documentos a izquierda; lista paginada virtualizada y edición activa al centro; TM a derecha; QA/mensajes abajo. Ctrl+Enter confirmar/avanzar, Alt+flechas navegar, Ctrl+Z/Y deshacer/rehacer, copiar source e insertar match explícitos. Preview de fila acotado; texto activo completo scrollable. No chatbot central. Path inputs iniciales evitan añadir dependencia de diálogos; evaluar diálogos nativos tras flujo funcional.

## Riesgos y aceptación

* Crash: prueba por proceso abortado después de commit y con transacción incompleta; reabrir conserva último commit y notifica recovery. Ventana de texto no durable se mide y se comunica; no promesa absoluta de cero pérdida.
* XML hostil: rechazar DTD/entidades externas, malformed, encoding no soportado, constructos/tagging fuera subset; límites de tamaño/profundidad/segmento; import atómico y cancelable. Fixtures Unicode RTL/CJK/emoji/combining y namespace/metadata; round-trip fuente/target/envelope.
* Export: temporal en mismo directorio, flush/sync, validar parse y persistir sin reemplazar; fallo deja original intacto. Prohibido clobber DB, TM u otro input por alias de path.
* Edición: cambios humanos/TM distinguibles, estado confirmado vuelve draft al modificar, locks hacen error typed, revision evita stale edits, undo/redo durable. IA no puede existir como write shortcut.
* Escala: UI conserva páginas y texto activo; paginación indexada sin offset proporcional; ninguna operación de DB o XML en render; queues bounded con error/dirty recuperable.

## Benchmark baseline

CLI reproducible release en `src/bin/benchmark.rs`, datasets sintéticos deterministas 10k/100k/1M, opcional 3M. Publicar hardware/build/dataset, conteo, DB tamaño, import/export TMX, exact/fuzzy/search/concordance, abrir proyecto/documento, autosave y QA; percentiles y recall cuando corresponda. No inferir corpus real de sintético. Startup cold requiere control cache SO (no usar kill como cold), warm se mide por proceso; RAM idle y scroll/input requieren app nativa real, pendientes si no hay automatización autorizada. Objetivos iniciales: frame CPU p95 <16.7ms, input no espera I/O, save durable p95 <100ms tras enqueue en máquina local, exact p95 <10ms y fuzzy p95 <100ms a 1M, RAM idle <150MiB; aspiracionales hasta medir. Abrir 250k sin cargar todas las filas. Registro en `output/` y `logs/tests/`, nunca raíz.
