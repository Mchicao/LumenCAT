# Decisiones arquitectónicas

## 001 — Núcleo único nativo

Un paquete Rust, biblioteca + binario egui/eframe. Módulos `model`, `storage`, `formats`, `tm`, `qa`, `worker`, `app`. No dividir crates sin frontera real. SQLite bundled/WAL/FULL para estado; no servidor, cuenta ni red. Eframe 0.32.3 inicialmente (versión conocida en el entorno, API inspeccionada), renderer glow mínimo; wgpu/eframe reciente se evaluará con medidas, no por moda. Electron/Tauri/WebView/JVM quedan descartados por requisitos explícitos. Winit directo agrega mantenimiento sin evidencia de beneficio.

## 002 — Durabilidad y comandos

Una conexión escritora poseída por worker; transacciones atómicas de cambio + historial persistente. Debounce objetivo 150–250 ms con máximo de espera aun bajo escritura continua. UI distingue pendiente/en cola/durable/error; una respuesta commit es la única confirmación de guardado. Deshacer/rehacer usa el mismo camino y persiste texto/estado/origen. Confirmar es humano y no escribe TM automáticamente. Documento original y path registrados; exportes nuevos nunca reemplazan original ni destino existente.

WAL + synchronous FULL protege commits en límites del SO/dispositivo, no garantiza hardware defectuoso. Al abrir: verificar application_id/schema, adquirir exclusión de sesión, comprobar cierre limpio, quick_check/integrity según estado, SQLite recupera WAL; avisar recuperación. No borrar WAL ni intentar reparar DB corrupta en sitio. Bloquear edición si integridad falla y preservar archivos para recuperación/backup. Primera migración transaccional; futuras migraciones exigen backup SQLite coherente antes de alterar schema. Rechazar versiones futuras.

## 003 — Documento y formatos

IR: `Document { name, format, original bytes, languages, segments }`; `Segment { id estable, ordinal, source, target, state, locked, origin, revision }`. Source inmutable. Persistir skeleton original separado. Serializador adapta IR al envelope, no regenerar XML arbitrariamente. Primer corte limita XLIFF a unidades sin inline codes ni seg-source; rechazo explícito evita destrucción. TXT preserva terminadores. Parsing/serialización/importación siempre fuera UI. Operaciones largas cooperativamente cancelables y transaccionales.

## 004 — Memorias grandes

Tablas TU + idioma + metadata/TU original, índices exactos; FTS5 para candidatos y concordancia. Normalización auxiliar NFC, nunca cambiar raw. Exacto exige raw+idiomas; fuzzy scoring Unicode de máximo 256 candidatos y límite de longitud para no bloquear worker con inputs hostiles. No mantener TM en UI. M1 una colección; múltiples memorias/prioridad/read-write/penalty en M2 con schema explícito. No semántica hasta demostrar base. Las sugerencias muestran porcentaje/origen; insertar es un comando humano, no confirmar.

## 005 — Autoridad humana y privacidad

Modelo futuro: `AgentAction -> ContextPreview -> Provider -> Proposal -> diff -> aceptación humana -> Command -> DB`. `LlmProvider` recibe objetos estructurados y no tiene acceso a storage; provider capability local/cloud controlada en dispatch central. AI Disabled por defecto, política local-only y veto cloud de proyecto ambos deben autorizar despacho. Keys en almacén SO fuera proyecto. Provider no puede confirmar, exportar ni editar TM. Proposals llevan revisión base y origen; aceptación rechaza revisiones obsoletas; operación masiva undoable. M3 implementa providers, no scaffolding de red en M1.

## 006 — Concurrencia acotada

Un worker de proyecto con channel bounded y try_send desde UI; cola llena preserva dirty text y comunica backpressure. Paginación por ordinal/ID; ventana de filas y segmento activo, no Vec de documento completo en app. Matching/QA al worker. Cancel token atómico; import rollback si cancelado. Operaciones masivas monopolizantes deben poder pausar/cancelar; fase posterior podrá separar lector TM del escritor si las medidas lo exigen. No cientos de tasks.

## Dependencias justificadas

`rusqlite` evita SQLite FFI propia, bundled permite FTS5/version conocida; `quick-xml` evita parser XML casero, streaming; `unicode-normalization` NFC; `strsim` Levenshtein Unicode existente; `tempfile` escritura exclusiva/commit sin clobber; `thiserror` errores tipados; `tracing` + subscriber logging estructurado sin contenido; `eframe` GUI y ciclo de vida nativo. Std threads/channels/archivos cubre el resto. Versiones y lockfile fijados; no runtime Node/Python ni librerías de red. Revisar avisos y mantener lock antes de distribuir.

## 007 — Word adelantado por prioridad explícita del usuario

DOCX básico pasa a M1. [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) y [Microsoft WordprocessingML](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document) definen paquete OPC, partes relacionadas y runs con formato propio. Preservar ZIP/partes/relaciones originales y modificar solo texto aceptado. Se rechazan campos, tracked changes, stories no procesadas y runs de formato mixto; concatenar estos últimos sin códigos inline perdería formato. No equivale a compatibilidad universal Word. El editor protegido de runs/tags deberá preceder soporte de documentos complejos.

`zip` 4.6.1 con `deflate-flate2-zlib-rs` solamente (no defaults/codecs/AES innecesarios); se inspeccionaron features vía cargo info. Límites ZIP propios sobre entradas, bytes expandidos y XML; no extraer rutas a disco. Round-trip compara bytes de partes ajenas, contenido y estructura; abrir con Word real añade evidencia de aplicación, no sustituye validación de fidelidad.

## 010 — Runs Word repetidos con formato idéntico

En el parser DOCX, los `w:t` de un párrafo se concatenan solo si cada `w:r` tiene el mismo XML `w:rPr` (comparación conservadora byte a byte). La exportación pone el target en el primer nodo de texto y elimina los restantes; el formato se mantiene porque los runs aceptados comparten propiedades. Cualquier diferencia, elemento inline desconocido, campo o salto se rechaza. Esto cubre fragmentación de texto del mismo estilo; no implementa códigos protegidos ni compatibilidad de formato mixto. Ver `docs/technical/DOCX_WORD_RESEARCH.md` y prueba de round-trip.

## 008 — Espacio de compilación

Rust instalado 1.97 ya satisface el proyecto; no actualizar toolchain sin necesidad medida. Profiles dev/test sin debug ni incremental y release thin-LTO/strip-debuginfo; target dentro `.cache/target`. Esto reduce artefactos frente a defaults a costa de recompilación incremental, por verificar con tamaños reales. No borrar cachés ni modificar perfiles de otros repositorios.

## 009 — Correcciones guiadas por evidencia

Consulta fuzzy con OR amplio omitió candidato y join elegido por SQLite hizo concordancia p95750ms en10k. Recuperar primero dos términos raros y usar FTS CROSS JOIN TM evita ese orden. Conteo exacto de frecuencia aún recorrió postings comunes: p95226ms en3M. `fts5vocab instance` con contador saturado512 limita trabajo; p95 final1.274ms en3M en corpus sintético. Score sigue Levenshtein NFC, exact exige raw+idiomas. No promesa de recall general, CJK ni corpus real basada en esos números.

Todos los filtros/TM/concordancia UI llevan token cooperativo y SQLite progress_handler retirado tras llamada. Dispatcher conserva token por request ID y descarta respuesta stale; un nuevo job no pierde su token por respuesta anterior. Locks no cambian estado/origen. Draft fallido se conserva y puede exportarse íntegro a TXT nuevo o descartarse explícitamente.

XLIFF textual reconoce translated/final/signed-off/approved al importar, refleja confirmación humana y lock al exportar, elimina aprobación obsoleta al reemplazar traducción. No hay workflow de revisión final propio aún. Identidad de unidades única dentro de file; source y envelope originales preservados.
