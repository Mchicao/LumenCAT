## Sub-features

- `memories.tm-tab-apply`: pestaña **TM MATCHES** del panel derecho con coincidencias exactas/fuzzy del segmento activo e inserción explícita (**Apply Match** / `tm-apply-N`, Ctrl+T o Ctrl+1..9).
- `memories.tmx-import`: importar TMX 1.4/1.4b con códigos desde **Avanzado → Importar memoria** (también en Archivo); termina con «TM importada: N unidades».
- `memories.sdltm-import` / `memories.sdltm-update-copy`: importar SDLTM (con Trados instalado usa su SDK sobre copia privada; sin Trados, lector nativo de solo lectura) y generar una copia actualizada desde **Avanzado → Actualizar SDLTM (copia, requiere Trados)**. La actualización exige Studio instalado; no se sobrescriben originales ni destinos existentes. GUI de ambos cortes todavía NOT RUN; CORE verificado (nativo contra fixtures 8.10 y 19.0) y CORE+SDK por separado.
- `memories.tmx-export`: exportar las unidades del proyecto a TMX nueva desde **Avanzado → Exportar TMX** (también en Archivo); validar el XML resultante.
- `memories.concordance-query`: concordancia sobre la memoria (botón **Concordancia** / F3) con resultados en la pestaña TM.
- `memories.create`: crear colecciones y elegir memoria de escritura compatible con el par del documento.
- `memories.learn-on-confirm`: confirmar aprende texto plano en la memoria de escritura; guardar borrador no aprende.
- `memories.readonly` / `memories.enable` / `memories.disable-learning` / `memories.suspend-restore`: alternar escritura/lectura, incluir/excluir de búsquedas, desactivar aprendizaje y suspender/restaurar contribuciones al editar/deshacer.

## How to get to it (user POV)

**Importar memoria**: pulsa **Avanzado → Importar memoria**. **Memorias del proyecto** (`memory-menu`) muestra el gestor de colecciones en el lateral. Importar abre diálogo `.tmx`/`.sdltm`; la barra informa «TM importada: N unidades». Los recursos viven dentro del `.lcat`, para el par del documento activo. SDLTM se lee en modo de solo lectura: con Trados usa su SDK sobre una copia SQLite consistente y sin Trados el lector nativo; configura el par exacto declarado, incluidas regiones.

**Aprender**: escribe «Nombre de memoria», pulsa **Crear memoria** y **Usar para aprender** en la colección compatible. Confirma una traducción y selecciona una repetición: debe ofrecerla con colección/procedencia. La memoria inicial puede requerir selección explícita. **Desactivar aprendizaje**, **Pasar a solo lectura** y **Excluir de búsquedas** no borran unidades; exportar incluye unidades activas incluso de colecciones excluidas. Véase [MEMORIAS](../../../../docs/guides/MEMORIAS.md).

**Consultar coincidencias**: al seleccionar un segmento la app consulta la memoria por su origen y la pestaña **TM MATCHES (n)** del panel derecho lista tarjetas `tm-match-N` con porcentaje, y «Src:»/«Tgt:». Sin memoria o sin parecido, muestra «Sin coincidencias para este segmento…». **Apply Match** (`tm-apply-<idx>`) reemplaza el destino del segmento activo (deshabilitado si está bloqueado); Ctrl+T aplica la primera y Ctrl+1..9 la N.

**Concordancia**: escribe consulta en `search-query` y pulsa **Concordancia** o F3 (enfocado el campo); F3 desde el editor consulta el destino solo si está todo seleccionado y si no, el origen. Resultados en la misma pestaña TM.

**Exportar TMX**: **Avanzado → Exportar TMX** abre diálogo Guardar; elige nombre nuevo. Valida TMX 1.4 parseable con header regenerado y las unidades importadas y aprendidas activas. No exijas una TU por destino único: varias contribuciones pueden tener el mismo texto.

## Driving it with cua-driver

Con el controlador (`launch -WaitSeconds 1200`, mismo `RunId` en cada acción): importa `source.txt`, importa `sample.tmx` (corpus del controlador: Hello world → Hola mundo) y verifica «TM importada: 1 unidades» en la barra de estado (`application-status`). Muévete al segmento 2 («Hello world»): la pestaña **TM** debe listar un exacto 100 % con «Tgt: Hola mundo» y procedencia importada; pulsa **Apply Match** y comprueba que el destino del editor pasa a «Hola mundo». Exporta TMX a nombre nuevo del RunId y valida el XML con Python (ElementTree) o PowerShell: `<tmx version="1.4">` y la unidad importada presente.

Negativos en UI: segmento bloqueado → el botón **Apply Match** está deshabilitado (clic sin efecto y token `enabled=false` en el árbol); TMX con versión inválida o pares nativos incoherentes → mensaje de rechazo y 0 unidades; exportar TMX sobre un destino existente → error «destino ya existe». Persistencia: `-Action reopen` reutiliza el proyecto; la memoria importada sigue respondiendo al segmento 2 tras reabrir.

Para SDLTM usa únicamente copias de las muestras públicas de la instalación dentro del RunId: importa una con el par exacto, confirma una traducción nueva y ejecuta **Actualizar SDLTM (copia, requiere Trados)** eligiendo original y destino nuevo. Verifica en API/Trados la búsqueda exacta/fuzzy de la copia y hashes intactos del original. Negativos: otro par, destino existente y ausencia del SDK. No uses una memoria personal ni conduzcas una instancia del usuario.

Evidencia: capturas before/after de cada clic (el controlador las guarda) y el TMX exportado dentro del RunId. No acredites consultas ni exportación solo con `tests/tm_retrieval.rs` o `tests/tm_learning.rs`: son núcleo.

## Gotchas

- En **Avanzado** los botones TMX están en la cinta; el gestor del lateral se muestra/oculta con **Memorias del proyecto**. No uses coordenadas del encabezado anterior.
- Tras reabrir el proyecto (`-Action reopen`), la memoria importada sigue respondiendo; el badge «TM 100 %» en la columna TARGET marca los segmentos cuya traducción viene de TM.
- En `audit-main-gpui-20261004` (base `524046f`) se seleccionó memoria de escritura, confirmó, reutilizó el aprendido en otra fila y exportó junto a la TMX importada. La creación y los toggles avanzados no se condujeron en esa pasada; `tests/tm_learning.rs` es evidencia CORE, no GUI.
- La importación TMX usa el par del documento activo; con varios documentos de pares distintos, selecciona primero el documento cuyo par debe recibir la memoria.
- TMX: 1.4/1.4b en UTF-8 o UTF-16 con BOM (LE/BE; la declaración `encoding` debe coincidir con los bytes), texto y `bpt/ept`, `ph`, `it`, `hi`, `ut`; `sub`, pares incoherentes, idiomas ambiguos o TU mayor a 8 MiB se rechazan. El header importado no se conserva; propiedades/atributos TU y otras variantes sí se preservan. Un formato exclusivo del destino no inventa etiquetas en el origen.
- SDLTM: ruta opcional SDK Windows, no escritor SQL portable ni redistribución de DLL RWS. Instancia/recorrido GUI pendientes; `tests/sdltm_sdk.rs` se ignora en la suite ordinaria y debe ejecutarse explícitamente con `--ignored` y un directorio de evidencia nuevo.
- Coincidencias: exactas por normalización NFC; fuzzy aproximado con umbral 50 %, top 8; agrupan destinos equivalentes. No es context match ni garantiza recall exhaustivo.
- El indicador «TM aplicado» de la sesión no se persiste; tras reabrir, el porcentaje histórico desaparece aunque el texto insertado persista.
- Hotkeys Ctrl+T / Ctrl+1..9 dependen del transporte background de GPUI: si `background_unavailable`, usa el botón **Apply Match** y registra el atajo como BLOCKED, no como bug.
