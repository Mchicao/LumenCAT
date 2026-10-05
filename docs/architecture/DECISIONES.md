# Decisiones arquitectónicas

## 001 — Núcleo único nativo

Un paquete Rust, biblioteca + un único ejecutable de escritorio `lumencat` con GPUI. Módulos `model`, `storage`, `formats`, `tm`, `qa`, `worker`, `gpui_app`. No dividir crates sin frontera real. SQLite bundled/WAL/FULL para estado; no servidor, cuenta ni red. GPUI y su backend nativo gobiernan render y ciclo de vida; no mantener una segunda interfaz ni un lanzador duplicado. Electron/Tauri/WebView/JVM quedan descartados por requisitos explícitos. Winit directo agrega mantenimiento sin evidencia de beneficio.

## 002 — Durabilidad y comandos

Una conexión escritora poseída por worker; transacciones atómicas de cambio + historial persistente. Debounce objetivo 150–250 ms con máximo de espera aun bajo escritura continua. UI distingue pendiente/en cola/durable/error; una respuesta commit es la única confirmación de guardado. Deshacer/rehacer usa el mismo camino y persiste texto/estado/origen. Confirmar es humano y aprende texto plano en una memoria de escritura elegida y compatible; guardar un borrador no aprende. Confirmación, historial y actividad TM se guardan en la misma transacción. Documento original y path registrados; exportes nuevos nunca reemplazan original ni destino existente.

WAL + synchronous FULL protege commits en límites del SO/dispositivo, no garantiza hardware defectuoso. Al abrir: verificar application_id/schema, adquirir exclusión de sesión, comprobar cierre limpio, quick_check/integrity según estado, SQLite recupera WAL; avisar recuperación. No borrar WAL ni intentar reparar DB corrupta en sitio. Bloquear edición si integridad falla y preservar archivos para recuperación/backup. Primera migración transaccional; futuras migraciones exigen backup SQLite coherente antes de alterar schema. Rechazar versiones futuras.

Evolución F00: el esquema v2 agrupa entradas de historial por operación. Los registros v1 se migran como operaciones individuales conservando cursor y redo. Toda migración existente exige respaldo online SQLite validado, transacción, conteos e integridad/relaciones; un fallo conserva el respaldo y revierte el schema. Recuperar siempre crea una copia nueva. Reemplazar múltiples destinos se deshace como unidad; los comandos masivos revisionados tienen límite de 10 000 segmentos y rollback completo ante conflicto/cancelación.

## 003 — Documento y formatos

IR: `Document { name, format, original bytes, languages, segments }`; `Segment { id estable, ordinal, source, target, state, locked, origin, revision }`. Source inmutable. Persistir skeleton original separado. Serializador adapta IR al envelope, no regenerar XML arbitrariamente. Primer corte limita XLIFF a unidades sin inline codes ni seg-source; rechazo explícito evita destrucción. TXT preserva terminadores. Parsing/serialización/importación siempre fuera UI. Operaciones largas cooperativamente cancelables y transaccionales.

## 004 — Memorias grandes

Tablas TU + idioma + metadata/TU original, índices exactos; FTS5 para candidatos y concordancia. Normalización auxiliar NFC, nunca cambiar raw. Exacto exige raw+idiomas (los códigos de idioma se comparan sin distinguir mayúsculas, las variantes regionales siguen siendo distintas); fuzzy scoring Unicode de máximo 256 candidatos y límite de longitud para no bloquear worker con inputs hostiles. No mantener TM en UI. F02 introduce colecciones por par, selección de escritura, solo lectura y exclusión de búsquedas; prioridades y penalizaciones quedan pendientes. Las contribuciones aprendidas se versionan por colección+segmento; editar el destino las suspende, reconfirmar publica y undo/redo restaura actividad. FTS conserva versiones inmutables y las consultas filtran `active`. No semántica hasta demostrar base. Las sugerencias muestran porcentaje/colección/procedencia; insertar es un comando humano, no confirmar.

## 005 — Autoridad humana y privacidad

Modelo futuro: `AgentAction -> ContextPreview -> Provider -> Proposal -> diff -> aceptación humana -> Command -> DB`. `LlmProvider` recibe objetos estructurados y no tiene acceso a storage; provider capability local/cloud controlada en dispatch central. AI Disabled por defecto, política local-only y veto cloud de proyecto ambos deben autorizar despacho. Keys en almacén SO fuera proyecto. Provider no puede confirmar, exportar ni editar TM. Proposals llevan revisión base y origen; aceptación rechaza revisiones obsoletas; operación masiva undoable. M3 implementa providers, no scaffolding de red en M1.

## 006 — Concurrencia acotada

Un worker de proyecto con channel bounded y try_send desde UI; cola llena preserva dirty text y comunica backpressure. Paginación por ordinal/ID; ventana de filas y segmento activo, no Vec de documento completo en app. Matching/QA al worker. Cancel token atómico; import rollback si cancelado. Operaciones masivas monopolizantes deben poder pausar/cancelar; fase posterior podrá separar lector TM del escritor si las medidas lo exigen. No cientos de tasks.

## Dependencias justificadas

`rusqlite` evita SQLite FFI propia, bundled permite FTS5/version conocida; `quick-xml` evita parser XML casero, streaming; `unicode-normalization` NFC; `strsim` Levenshtein Unicode existente; `tempfile` escritura exclusiva/commit sin clobber; `thiserror` errores tipados; `tracing` + subscriber logging estructurado sin contenido; `gpui` y `gpui_platform` GUI y ciclo de vida nativo. Std threads/channels/archivos cubre el resto. Versiones y lockfile fijados; no runtime Node/Python ni librerías de red. Revisar avisos y mantener lock antes de distribuir.

## 007 — Word adelantado por prioridad explícita del usuario

DOCX básico pasa a M1. [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) y [Microsoft WordprocessingML](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document) definen paquete OPC, partes relacionadas y runs con formato propio. Preservar ZIP/partes/relaciones originales y modificar solo texto aceptado. Se rechazan campos, tracked changes, stories no procesadas y runs de formato mixto; concatenar estos últimos sin códigos inline perdería formato. No equivale a compatibilidad universal Word. El editor protegido de runs/tags deberá preceder soporte de documentos complejos.

`zip` 4.6.1 con `deflate-flate2-zlib-rs` solamente (no defaults/codecs/AES innecesarios); se inspeccionaron features vía cargo info. Límites ZIP propios sobre entradas, bytes expandidos y XML; no extraer rutas a disco. Round-trip compara bytes de partes ajenas, contenido y estructura; abrir con Word real añade evidencia de aplicación, no sustituye validación de fidelidad.

## 010 — Runs Word repetidos con formato idéntico

En el parser DOCX, los `w:t` de un párrafo se concatenan solo si cada `w:r` tiene el mismo XML `w:rPr` (comparación conservadora byte a byte). Extendido por la decisión 011: hoy los runs contiguos con `w:rPr` idéntico forman regiones y las fronteras de estilo se exponen como códigos protegidos en lugar de rechazarse. Ver `docs/technical/DOCX_WORD_RESEARCH.md` y prueba de round-trip.

## 011 — Regiones de formato e imágenes Word con códigos protegidos

El segmento DOCX sigue siendo el párrafo, pero los runs contiguos con `w:rPr` byte-idéntico se agrupan en **regiones**. La primera región del párrafo define el estilo base y su texto se muestra sin códigos; cada región con estilo distinto se expone como par `<g id="k">…</g>` (ids 1..n por párrafo, sin anidar). Los runs exclusivamente gráficos (`w:drawing` con una imagen estática embebida) se representan como `<x id="k"/>` y se copian byte a byte en la exportación; sus `w:rPr` no alteran el formato del texto vecino. El editor protegido existente (`src/editing.rs`, `insert_next_tag`) opera sobre estos códigos sin cambios.

La exportación exige: todos los códigos del original presentes exactamente una vez (sin duplicar, anidar ni códigos desconocidos), texto no vacío en cada región, y un round-trip propio que compara texto plano + multiset de `<x/>` + conteo de `<g>` (tolerante al reordenado del texto alrededor de códigos). La reconstrucción emite un `w:r` por fragmento de texto con el `w:rPr` original de su región; se pierden atributos `w:rsid*` de los runs reescritos, sin efecto en Word.

Solo se aceptan imágenes estáticas embebidas: dentro de `w:drawing` se permiten prefijos `wp/a/pic/a14/a16/wp14`, todo `graphicData` debe ser `picture`, se rechaza texto (`w:*`, `a:t`, `txbxContent`), gráficos, SmartArt, VML (`w:pict`), `mc:AlternateContent` y `r:embed` sin relación de imagen válida en `word/_rels/document.xml.rels`. Límites declarados: marcado entre los runs de un párrafo (bookmarks intercalados), runs con texto e imagen juntos, `w:tab`/`w:br`/`w:cr`, hyperlinks, campos, revisiones, content controls, historias con texto fuera de `word/document.xml` y Strict OOXML siguen rechazados con mensaje explícito. Un DOCX sin texto traducible (solo imágenes) importa con cero segmentos y exporta con todas las partes internas idénticas; el contenedor ZIP puede cambiar de hash.

Evidencia con Word real 16.0.20326: `word-inspection.docx` (tablas con celdas combinadas, tres imágenes inline, formato mixto), `mammoth-underline.docx` (formato mixto) y `mammoth-tiny-picture.docx` (solo imagen) traducen, exportan y reabren con tablas/celdas/anchos, dimensiones de imágenes, secciones, páginas y párrafos idénticos; el render PDF compara malla de tinta por página (las imágenes se reflow-ean con el texto más largo, sin pérdida). Ver `docs/technical/CONTINUACION_DOCX.md`.

## 008 — Espacio de compilación

Rust instalado 1.97 ya satisface el proyecto; no actualizar toolchain sin necesidad medida. Profiles dev/test sin debug ni incremental y release thin-LTO/strip-debuginfo; target dentro `.cache/target`. Esto reduce artefactos frente a defaults a costa de recompilación incremental, por verificar con tamaños reales. No borrar cachés ni modificar perfiles de otros repositorios.

## 009 — Correcciones guiadas por evidencia

Consulta fuzzy con OR amplio omitió candidato y join elegido por SQLite hizo concordancia p95750ms en10k. Recuperar primero dos términos raros y usar FTS CROSS JOIN TM evita ese orden. Conteo exacto de frecuencia aún recorrió postings comunes: p95226ms en3M. `fts5vocab instance` con contador saturado512 limita trabajo; p95 final1.274ms en3M en corpus sintético. Score sigue Levenshtein NFC, exact exige raw+idiomas. No promesa de recall general, CJK ni corpus real basada en esos números.

Todos los filtros/TM/concordancia UI llevan token cooperativo y SQLite progress_handler retirado tras llamada. Dispatcher conserva token por request ID y descarta respuesta stale; un nuevo job no pierde su token por respuesta anterior. Locks no cambian estado/origen. Draft fallido se conserva y puede exportarse íntegro a TXT nuevo o descartarse explícitamente.

XLIFF textual reconoce translated/final/signed-off/approved al importar, refleja confirmación humana y lock al exportar, elimina aprobación obsoleta al reemplazar traducción. No hay workflow de revisión final propio aún. Identidad de unidades única dentro de file; source y envelope originales preservados.

## 012 — Terminología por conceptos, independiente de TM

F03.1 introduce bases activables dentro del proyecto, conceptos con notas/dominio/procedencia y expresiones por idioma con estado preferido/permitido/prohibido y sensibilidad a mayúsculas. El schema v5 añade tres tablas sin reescribir documentos, historial ni TU; una base v4 recibe respaldo antes de migrar. Añadir un concepto es una transacción humana, no aprende TM, no edita segmentos ni confirma. GPUI administra los recursos y presenta los avisos QA del worker.

Reconocimiento por secuencias de tokens Unicode alfanuméricos y marcas combinantes, NFC auxiliar y minúsculas Unicode cuando no se distingue el caso. Se conservan texto/offsets originales. Solo DOCX interpreta `<g>/<x/>` como códigos al reconocer; TXT/XLIFF textual conserva esas cadenas como texto literal. No se promete morfología ni segmentación de lenguas sin espacios.

QA exige alguna equivalencia permitida, no todas las variantes de un concepto. Si varios conceptos coinciden en el mismo rango, muestra ambigüedad y no decide qué sentido ni qué prohibición corresponde. Un límite o error deja terminología «no evaluada», conserva QA textual y no anuncia aprobación. El escaneo actual tiene techo de 10.000 expresiones por proyecto y 512 coincidencias/incidencias por consulta; requiere indexación/medición antes de ampliarlo. TSV/TBX, excepciones, edición de variantes, selección y resaltado quedan para cortes posteriores.

## 013 — GPUI con accesibilidad nativa para Computer Use

GPUI y `gpui_platform` usan el mismo commit upstream fijado en Cargo.lock: `8e7fbcc131ec0f41cc4e78fbc8c50ef41f47b382`, HEAD verificado el 30-09-2026. La versión publicada 0.2.2 no incluye el puente Windows necesario; el upstream conserva ese número de versión, por lo que la identidad real es el SHA. La restricción `version = "=0.2.2"` evita resolver el crate fixture homónimo 0.0.0 incluido en el repositorio de Zed. Se usa `gpui_platform::application()` y su backend AccessKit/UIA; no se introduce un servidor de automatización propio.

Botones, documentos, filas, campos, pestañas, TM, QA y estado exponen roles, nombres y AutomationId. `Invoke` comparte el handler del clic; `ValuePattern.SetValue` comparte autoguardado, historial y QA del editor. Origen, celdas y destinos bloqueados son de solo lectura. Las filas usan Invoke y describen la selección: AccessKit Windows 0.34 aún no ofrece SelectionItem para Role::Row, y declarar selected suprimiría Invoke. Documentos y pestañas conservan SelectionItem.

El agente observa, actúa por token del snapshot y verifica un estado nuevo. Consultas filtradas y snapshots sin screenshot reducen el volumen de observación entre capturas visuales. No se garantiza un ahorro fijo de tokens ni compatibilidad con todos los lectores UIA: el cliente .NET UIAutomationClient devolvió un árbol vacío en esta máquina; cua-driver sí accede al proveedor. ValuePattern reemplaza el campo completo; selección parcial/TextPattern, IME/RTL siguen pendientes; la virtualización del grid llegó después con `ListState` y expone solo las filas visibles.

El pie permanente con mensajes promocionales se elimina. Se conserva un estado accesible y solo se muestra texto cuando hay trabajo pendiente o un error que atender.
