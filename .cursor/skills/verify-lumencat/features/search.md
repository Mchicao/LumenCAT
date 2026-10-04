## Sub-features

- `search.document`: buscar literal en fuente y destino del documento activo (alcance **Documento**).
- `search.segment`: comparar la consulta contra el segmento activo (alcance **Segmento**).
- `search.concordance`: consultar la memoria TM por concordancia desde la consulta o el editor (botón **Concordancia** o F3).
- `search.replace-document`: reemplazar en los destinos no bloqueados del documento activo.
- `search.replace-segment`: reemplazar solo en el destino del segmento activo.
- `search.clear-escape`: Esc cancela resultados y panel de reemplazo; **Limpiar** vacía la consulta.

## How to get to it (user POV)

Con proyecto, documento y segmento activos, la barra «Buscar y reemplazar» tiene el campo `search-query` (etiqueta UIA «Buscar en origen y destino»), el botón conmutador `search-scope` (**Documento**/**Segmento**), **Buscar** (`search-submit`), **Concordancia** (`concordance`), **Reemplazar** (`replace-toggle`) y **Limpiar** (`search-clear`, activo solo con búsqueda vigente). Ctrl+F enfoca la consulta; Ctrl+H además muestra el campo de reemplazo `replacement-text` (etiqueta UIA «Texto de reemplazo») con **Aplicar al destino** (`replace-apply`). Enter en consulta o reemplazo ejecuta la búsqueda.

Alcance **Documento**: busca subcadenas literales en origen o destino de todo el documento y el grid muestra solo las filas coincidentes con «N segmentos encontrados» en la barra de estado; sin coincidencias muestra **No matching segments found.** Alcance **Segmento**: solo evalúa origen/destino del segmento activo (1 o 0 resultados).

**Reemplazo** (consulta no vacía, sin cambios pendientes ni worker ocupado): con alcance Documento reescribe destinos no bloqueados y reporta «Reemplazo guardado: N segmentos»; distingue mayúsculas, preserva los códigos protegidos `<g>/<x/>` y escribe una entrada de historial por segmento (Undo/Redo avanza segmento a segmento, no como grupo en esta build). Con alcance Segmento reemplaza solo el destino activo editable y guarda. Un destino bloqueado no cambia.

**Concordancia**: con consulta no vacía, **Concordancia** o F3 (enfocado el campo de búsqueda) consulta la TM del par del documento activo; F3 desde el editor consulta el destino solo si está todo seleccionado y si no, el origen. Los resultados llenan la pestaña **TM MATCHES** del panel derecho (véase [memories](memories.md)). Esc limpia resultados y panel de reemplazo y devuelve el foco al editor (el texto de la consulta permanece); **Limpiar** vacía también la consulta y vuelve al grid completo.

## Driving it with cua-driver

Sobre una sesión aislada con documento importado, `-Action snapshot` antes de decidir coordenadas; los campos son canvas GPUI, así que `-Action type -X <x> -Y <y> -Text '<consulta>'` sobre la posición actual del campo (foco previo con `-Action click`). El árbol UIA sí expone `search-query`, `search-scope`, `search-submit`, `concordance`, `replace-toggle`, `search-clear`, `replace-apply` y la barra «Buscar y reemplazar»: cuando el token exista, prefiere `-Label <etiqueta> -Role <rol>` a coordenadas. Evita hotkeys: el transporte background de GPUI puede rechazarlas; si `background_unavailable`, registra BLOCKED para ese atajo y usa el botón equivalente.

Recorrido mínimo: (1) consulta con coincidencias en alcance Documento y una sin coincidencias («No matching segments found.»); (2) cambio de alcance a Segmento con su resultado; (3) reemplazo de documento con un destino bloqueado y un tag presente: verifica que el bloqueado no cambia, que el tag sobrevive y que Undo restaura segmento a segmento; (4) reemplazo de segmento sobre el activo; (5) Esc y **Limpiar**. Registra texto consultado, contador «N segmentos encontrados» y estado del grid en cada paso.

## Gotchas

- Documento busca con `instr` literal desde cero en fuente o destino y recorta la consulta; Segmento inspecciona solo el activo: sin segmento activo no hay resultado.
- La búsqueda es sensible a mayúsculas; el reemplazo también (`replace` literal por parte de texto, no por regex).
- Reemplazar exige consulta no vacía y estado limpio (sin borrador sin guardar ni operación ocupada); si el botón está deshabilitado, guarda primero.
- El reemplazo de documento omite segmentos bloqueados y no toca orígenes; pone los destinos en estado Draft (borrador) aunque estuvieran confirmados.
- En esta build cada segmento reemplazado genera su propia entrada de historial: un solo Undo restaura UN segmento. La antigua afirmación «unidad de undo/redo agrupada» corresponde a otra rama; verifícalo en tu build antes de asumirlo.
- Escape limpia resultados y modo pero conserva el texto de consulta; **Limpiar** (solo visible como activo con búsqueda vigente) sí vacía el campo. Los campos de GPUI insertan en el caret: para cambiar de consulta pasa por **Limpiar**, nunca escribas encima.
- `tests/replacement.rs` y `tests/search_cancel.rs` son núcleo: preservación de tags, bloqueos, rollback y cancelación de escaneo. No los presentes como E2E GUI.
