## Sub-features

- `project.open-create`: abrir un proyecto existente o crear uno nuevo desde el diálogo nativo (Guardar con filtro `.lcat`/`.db`/`.sqlite`).
- `project.import-document`: añadir documentos DOCX/XLIFF/TXT a un proyecto recién creado, desde diálogo nativo.
- `project.language-pair`: fijar el par de idiomas de próximas importaciones (**Origen**/**Destino** + **Guardar idiomas**). NO implementado en la build actual del checkout principal (existe en `origin/main` 51bd864); hoy la importación usa el par por defecto del proyecto (`en → es`) y cada documento conserva el par con el que se importó.
- `project.document-selection`: elegir el documento activo en la lista **DOCUMENTS** y ver su progreso.

## How to get to it (user POV)

1. Lanza GPUI con el controlador: `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId <nuevo> -WaitSeconds 1200`. Devuelve PID, ventana, proyecto y corpus sintético (`source.txt`, `sample.xlf`, `sample.tmx`) bajo `output/verification/<RunId>/`; abre un proyecto vacío con `--project`. Exige leer `*-ready.png` antes de actuar.
2. **Abrir/crear**: el launch ya abre un proyecto, así que el botón del encabezado lee **Switch Project** (`project-open`). Al pulsarlo se abre el diálogo nativo de **Guardar**: escribe una ruta nueva (crea el proyecto) o elige un `.lcat` existente (lo abre). El nombre del proyecto nuevo aparece en la etiqueta del encabezado (`project-name`) y la lista **DOCUMENTS** queda vacía («No documents yet»).
3. **Importar**: en un proyecto nuevo pulsa **Importar documento** (`document-import`), elige un archivo sintético del RunId en el diálogo nativo (filtros `docx`, `xlf/xliff`, `txt`) y pulsa **Abrir**. El lateral muestra el documento con su par y conteo (`en → es; N segmentos`) y el grid carga sus filas; la barra inferior indica el resultado. No uses seeds SQLite como sustituto de esta importación GUI.
4. **Par de idiomas**: sin campos **Origen/Destino** visibles en esta build, no hay paso de usuario; el par efectivo es el del proyecto (`en → es`) y se comprueba en la descripción UIA del documento importado. Cuando exista la UI, irá aquí: cambiar campos, **Guardar idiomas** o Enter, y esperar el mensaje de guardado.
5. **Seleccionar documento**: pulsa la fila del documento en **DOCUMENTS** (`document-<id>`); se vuelve activa (borde resaltado) y el grid carga el primer segmento como activo. La barra de progreso inferior muestra traducidos/total.

Un fallo de apertura/importación debe verse en la barra de estado (`application-status`); una apertura fallida no acredita persistencia. Véase [la guía de inicio](../../../../docs/guides/INICIO.md).

## Driving it with cua-driver

Todo pasa por `scripts/utils/control_lumencat.ps1` con el mismo `RunId` (launch/doctor/snapshot/click/type/key/cleanup) y `-WaitSeconds` holgado; otros agentes comparten la cola `output/verification/.app-lock.json`, así que ejecuta `cleanup` al terminar cada corrida. `-Action snapshot` devuelve árbol UIA + PNG; en GPUI el interior expone pocos controles, así que deriva cada coordenada de la captura inmediatamente anterior y no reutilices coordenadas de otra ejecución.

Diálogo nativo de archivos: tras pulsar el botón de la app, enumera ventanas del MISMO PID con `cua-driver call list_windows '{"pid":<PID>}'`, toma el HWND del diálogo (título «Importar documento»/«Exportar documento»/«Guardar…») y usa `-Action type -WindowId <HWND> -Label 'Nombre:' -Role Edit -Text '<ruta>'` y `-Action click -WindowId <HWND> -Label 'Abrir' -Role SplitButton` (o el botón **Guardar** que muestre la captura; suele ser SplitButton y el rol `Button` duplica el match). Para el diálogo de proyecto el campo también es un Edit «Nombre:».

Evidencia por subfeature en `output/verification/<RunId>/results.md` con estados `GUI PASS | CORE PASS | FAIL | BLOCKED | NOT RUN`. Cada acción del controlador guarda before/after PNG+JSON automáticamente; añade `snapshot` extra para leer estados intermedios (barra de estado, lista DOCUMENTS).

## Gotchas

- El botón `project-open` está deshabilitado con cambios pendientes u operación ocupada: confirma o deshaz antes de cambiar de proyecto.
- Abrir/crear usa un diálogo de **Guardar**: la carpeta destino debe existir; cancelar no crea el proyecto y no se registra apertura.
- Un proyecto nuevo empieza con par por defecto `en → es`; en esta build no hay campos de idioma en el lateral. Un XLIFF cuyo par declarado difiera se importa con el par del proyecto (sin conflicto detectado): ese chequeo pertenece a `formats.language-conflict`, no implementado aún.
- Importar/guardar está bloqueado con cambios sin guardar o worker ocupado («Background worker busy…»); espera el estado final antes del siguiente paso.
- TMX no se importa desde **Importar documento** (el filtro no lo incluye): va por **Memoria ▾ → Importar TMX**; véase [memories](memories.md).
- El grid de un documento recién importado carga por páginas (128 filas); con documentos grandes, la primera selección abre el primer segmento, no todo el grid.
