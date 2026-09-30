# Verificación GPUI y DOCX — 2026-09-30

GPUI funciona como entrada predeterminada. Se verificó en Windows la traducción de un DOCX público con tabla, desde la edición de sus seis segmentos hasta su exportación por la interfaz. La compatibilidad Word sigue siendo parcial: gráficos, SmartArt, cuadros de texto e historias secundarias fueron rechazados explícitamente.

## Integración y checks

Se incorporaron en el árbol de trabajo los cambios de código, tests y documentación de `origin/main`, commit `6eacc756232a49abb9771c2055708d5f26c1352e` («Mejora compatibilidad DOCX y valida referencias XML»), preservando el trabajo GPUI local. No se modificó el historial Git ni se hicieron commits o push. La rama continúa basada en `b9e188c`.

- `cargo check --all-targets --locked`: aprobado.
- `cargo test --locked`: 26 pruebas aprobadas, cero fallos, incluidos recuperación tras terminación, rollback, revisiones y bloqueos.
- `cargo clippy --all-targets --locked -- -D warnings`: aprobado. Cargo sigue notificando incompatibilidad futura de la dependencia `proc-macro-error2 2.0.1`.
- `cargo fmt --all -- --check` y `git diff --check`: aprobados.
- Build debug del binario principal: aprobada. Lanzamiento real GPUI predeterminado y egui con `--legacy-egui`: verificados; no se afirma una nueva build release.

## Comportamiento observado

La prueba usó `cua-driver`, con activación breve autorizada por el usuario para combinaciones de teclado. Sus resultados de entrega no se trataron como prueba suficiente: se inspeccionaron capturas, valores de diálogos, SQLite y el archivo exportado.

- Fila activa, clic directo, flechas y auto-scroll: navegación desde un filtro al segmento 150 y luego al 151, cruzando el límite de página 128.
- Ctrl+F y Ctrl+H: entrada real en buscador/reemplazo, alcance seleccionable, búsqueda documental, cambio de `café` a `té` y restauración con Ctrl+Z comprobada en SQLite.
- Tags: badges de apertura/cierre/autocierre, Ctrl+, inserta el siguiente tag. Prueba de lógica para tags repetidos, Unicode y reemplazo sin cambiar atributos. Esto no habilita códigos inline en el importador XLIFF ni formato mixto de runs Word.
- Texto con acentos mediante entrada nativa: `Traducción café`, `té`, `Arriba` y las traducciones de celdas persistieron correctamente. No se certifican IME, RTL, selección parcial con ratón o todos los caracteres suplementarios.
- Autoguardado: se detectó que el temporizador GPUI original no completaba una edición aislada en este entorno. El pulso acotado de 250 ms mediante hilo y `async-channel` sí persistió una edición sin confirmar/navegar/Ctrl+S. Evidencia en `output/verification/autosave-result.json`.
- Importación TMX por diálogo, match exacto y aplicación explícita: origen `tm` persistido; indicador central `TM 100%` visible. El porcentaje es metadata de sesión.
- Traducción DOCX por la ventana: seis segmentos confirmados, avance 100%, cero pendientes, conteos de palabras/caracteres visibles.
- Cierre: espera guardar, cierra el worker y deja `session.clean=1`. Se comprobó el cierre con una modificación pendiente.

El test de reemplazo fuerza un fallo por superar 1 MiB después de haber procesado un segmento anterior: toda la operación y su historial se revierten. También verifica bloqueos, conservación de tags, undo/redo y reapertura limpia.

## Corpus Word público

Descargas originales, URLs, tamaños, SHA-256 y conteo de nodos de texto por parte: `output/verification/corpus-manifest.json`. Los DOCX están en `output/verification/`, ignorado por Git. Las muestras con visualizaciones contienen nodos de texto reales, no solo imágenes vacías.

| Muestra | Procedencia | Resultado |
|---|---|---|
| `test.docx` | [python-docx](https://github.com/python-openxml/python-docx/blob/master/tests/test_files/test.docx) | Aceptado: 2 segmentos; edición SQLite, exportación y reimportación. |
| `mammoth-tables.docx` | [Mammoth](https://github.com/mwilliamson/mammoth.js/blob/master/test/test-data/tables.docx) | Aceptado: 6 segmentos, incluida tabla. También traducido/exportado desde GPUI. |
| `paragraphs-tables.docx` | [python-docx](https://github.com/python-openxml/python-docx/blob/master/features/steps/test_files/blk-paras-and-tables.docx) | Rechazado: texto en encabezado/pie de página. |
| `table.docx` | [python-docx](https://github.com/python-openxml/python-docx/blob/master/features/steps/test_files/tbl-2x2-table.docx) | Rechazado: tabla sin párrafos traducibles. |
| `inline-shapes.docx` | [python-docx](https://github.com/python-openxml/python-docx/blob/master/features/steps/test_files/shp-inline-shape-access.docx) | Rechazado: texto en partes de diagramas. |
| `chart-column.docx` | [Open XML SDK](https://github.com/OfficeDev/Open-XML-SDK/tree/main/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/chart) | Rechazado: texto en parte de gráfico. |
| `smartart.docx` | [Open XML SDK](https://github.com/OfficeDev/Open-XML-SDK/tree/main/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/smart%20art) | Rechazado: texto en partes de diagramas. |
| `textbox.docx` | [Open XML SDK](https://github.com/OfficeDev/Open-XML-SDK/tree/main/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/textbox) | Rechazado: contenido `pict`. |
| `mammoth-textbox.docx` | [Mammoth](https://github.com/mwilliamson/mammoth.js/blob/master/test/test-data/text-box.docx) | Rechazado: contenido/namespace alternativo. |

El soporte de marcadores de párrafo se añadió conservando sus anclajes y XML. Ninguno de estos resultados permite afirmar compatibilidad universal DOCX ni paridad de render con Microsoft Word.

## Artefactos y reproducción

- `output/verification/gpui-table-es.docx`: exportado con el botón de GPUI. Sus textos son `Arriba`, `Superior izquierda`, `Superior derecha`, `Inferior izquierda`, `Inferior derecha`, `Debajo`. Se verificó una tabla conservada y bytes idénticos de todas las partes diferentes de `word/document.xml`.
- El DOCX fuente permanece idéntico al BLOB original almacenado durante la importación.
- Capturas: `gpui-final.png`, `gpui-docx-complete.png`, `gpui-tm-applied.png`, `gpui-scroll-151.png`, `gpui-unicode.png`, `legacy-egui.png`.
- Logs: `logs/tests/final.txt`, `logs/tests/clippy.txt`, `logs/tests/real-docx.txt`.
- Verificador reproducible: `cargo run --locked --example verify_docx -- <directorio_nuevo_con_muestras_docx>`. Usa un directorio nuevo: el exportador protege destinos existentes. El script usa el importador, ProjectStore, edición, exportador y reimportador de la app; verifica fuentes y partes ajenas.

Opcional: actualizar el contexto de diseño con `impeccable init`. `docs/PRODUCT.md` todavía describe egui y conserva el esquema antiguo de la skill; este trabajo no reescribió ese contexto.
