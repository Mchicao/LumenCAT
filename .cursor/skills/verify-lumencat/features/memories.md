## Sub-features

- `memories.tm-tab-apply`: pestaña **TM MATCHES** del panel derecho con coincidencias exactas/fuzzy del segmento activo e inserción explícita (**Apply Match** / `tm-apply-N`, Ctrl+T o Ctrl+1..9).
- `memories.tmx-import`: importar TMX 1.4 textual desde **Memoria ▾ → Importar TMX**; termina con «TM importada: N unidades».
- `memories.tmx-export`: exportar las unidades del proyecto a TMX nueva desde **Memoria ▾ → Exportar TMX**; validar el XML resultante.
- `memories.concordance-query`: concordancia sobre la memoria (botón **Concordancia** / F3) con resultados en la pestaña TM.
- `memories.create`: crear colecciones de memoria y elegir memoria de escritura. NO implementado en la build actual del checkout principal (existe en `origin/main` f57e43b): no hay gestor de memorias en el lateral ni aprendizaje al confirmar.
- `memories.learn-on-confirm`: confirmar escribe en la memoria de escritura. NO implementado en la build actual: «Confirmar y avanzar» solo fija el estado del segmento; las coincidencias TM provienen únicamente de TMX importadas.
- `memories.readonly` / `memories.enable` / `memories.disable-learning` / `memories.suspend-restore`: alternar escritura/lectura, incluir/excluir de búsquedas, desactivar aprendizaje y suspender/restaurar contribuciones. NO implementados en la build actual (mismos commits pendientes).

## How to get to it (user POV)

**Importar TMX**: pulsa **Memoria ▾** (`memory-menu`) del encabezado; NO despliega un menú: revela los botones **Importar TMX** (`tm-import`) y **Exportar TMX** (`tm-export`) junto a «Exportar documento». **Importar TMX** abre el diálogo nativo con filtro `.tmx`; elige el archivo y la barra de estado termina con «TM importada: N unidades». La memoria vive dentro del `.lcat` del proyecto, para el par del documento activo; no hay colecciones visibles en esta build.

**Consultar coincidencias**: al seleccionar un segmento la app consulta la memoria por su origen y la pestaña **TM MATCHES (n)** del panel derecho lista tarjetas `tm-match-N` con porcentaje, y «Src:»/«Tgt:». Sin memoria o sin parecido, muestra «Sin coincidencias para este segmento…». **Apply Match** (`tm-apply-<idx>`) reemplaza el destino del segmento activo (deshabilitado si está bloqueado); Ctrl+T aplica la primera y Ctrl+1..9 la N.

**Concordancia**: escribe consulta en `search-query` y pulsa **Concordancia** o F3 (enfocado el campo); F3 desde el editor consulta el destino solo si está todo seleccionado y si no, el origen. Resultados en la misma pestaña TM.

**Exportar TMX**: **Memoria ▾ → Exportar TMX** abre diálogo Guardar; elige nombre nuevo. El archivo resultante es TMX 1.4 con header regenerado por LumenCAT: valida con PowerShell/Python que parsea y contiene las unidades activas del proyecto (importadas; aprendidas cuando esa capa exista).

## Driving it with cua-driver

Con el controlador (`launch -Surface gpui -WaitSeconds 1200`, mismo `RunId` en cada acción): importa `source.txt`, importa `sample.tmx` (corpus del controlador: Hello world → Hola mundo) y verifica «TM importada: 1 unidades» en la barra de estado (`application-status`). Muévete al segmento 2 («Hello world»): la pestaña **TM** debe listar un exacto 100 % con «Tgt: Hola mundo» y procedencia importada; pulsa **Apply Match** y comprueba que el destino del editor pasa a «Hola mundo». Exporta TMX a nombre nuevo del RunId y valida el XML con Python (ElementTree) o PowerShell: `<tmx version="1.4">` y la unidad importada presente.

Negativos en UI: segmento bloqueado → el botón **Apply Match** está deshabilitado (clic sin efecto y token `enabled=false` en el árbol); TMX con versión distinta de 1.4 o con códigos inline → mensaje de rechazo y 0 unidades; exportar TMX sobre un destino existente → error «destino ya existe». Persistencia: `-Action reopen` reutiliza el proyecto; la memoria importada sigue respondiendo al segmento 2 tras reabrir.

Evidencia: capturas before/after de cada clic (el controlador las guarda) y el TMX exportado dentro del RunId. No acredites consultas ni exportación solo con `tests/tm_retrieval.rs` o `tests/tm_learning.rs`: son núcleo.

## Gotchas

- Al pulsar **Memoria ▾** los botones TMX aparecen INLINE en el encabezado y desplazan a «Exportar documento»: recalcula coordenadas del snapshot actual antes de clicarlos.
- Tras reabrir el proyecto (`-Action reopen`), la memoria importada sigue respondiendo; el badge «TM 100 %» en la columna TARGET marca los segmentos cuya traducción viene de TM.
- En esta build NO hay gestor de memorias: nada de «Crear memoria», «Usar para aprender», solo lectura, exclusión de búsquedas ni mensajes «Confirmado y aprendido…». Si tu build los tiene (origin/main), usa esas variantes y actualiza esta receta.
- La importación TMX usa el par del documento activo; con varios documentos de pares distintos, selecciona primero el documento cuyo par debe recibir la memoria.
- TMX: solo 1.4/1.4b textual; TU con códigos inline, vacío, ambiguo en idiomas o mayor a 8 MiB se rechaza. El header del TMX importado no se conserva: la exportación regenera el header con LumenCAT y revalida cada TU.
- Coincidencias: exactas por normalización NFC; fuzzy aproximado con umbral 50 %, top 8; agrupan destinos equivalentes. No es context match ni garantiza recall exhaustivo.
- El indicador «TM aplicado» de la sesión no se persiste; tras reabrir, el porcentaje histórico desaparece aunque el texto insertado persista.
- Hotkeys Ctrl+T / Ctrl+1..9 dependen del transporte background de GPUI: si `background_unavailable`, usa el botón **Apply Match** y registra el atajo como BLOCKED, no como bug.
