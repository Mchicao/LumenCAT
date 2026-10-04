## Sub-features

- `project.open-create`: abrir un proyecto existente o crear uno nuevo desde el diálogo nativo (Guardar con filtro `.lcat`/`.db`/`.sqlite`).
- `project.import-document`: añadir documentos DOCX/XLIFF/TXT a un proyecto recién creado, desde diálogo nativo.
- `project.language-pair`: fijar y persistir el par de próximas importaciones (**Origen**/**Destino** + **Guardar idiomas**), sin cambiar documentos existentes.
- `project.document-selection`: elegir el documento activo en la lista **DOCUMENTS** y ver su progreso.

## How to get to it (user POV)

1. Lanza GPUI con el controlador: `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId <nuevo> -WaitSeconds 1200`. Devuelve PID, ventana, proyecto y corpus sintético (`source.txt`, `sample.xlf`, `sample.tmx`) bajo `output/verification/<RunId>/`; abre un proyecto vacío con `--project`. Exige leer `*-ready.png` antes de actuar.
2. **Abrir/crear**: el launch ya abre un proyecto, así que el botón del encabezado lee **Switch Project** (`project-open`). Al pulsarlo se abre el diálogo nativo de **Guardar**: escribe una ruta nueva (crea el proyecto) o elige un `.lcat` existente (lo abre). El nombre del proyecto nuevo aparece en la etiqueta del encabezado (`project-name`) y la lista **DOCUMENTS** queda vacía («No documents yet»).
3. **Importar**: en un proyecto nuevo pulsa **Importar documento** (`document-import`), elige un archivo sintético del RunId en el diálogo nativo (filtros `docx`, `xlf/xliff`, `txt`) y pulsa **Abrir**. El lateral muestra el documento con su par y conteo (`en → es; N segmentos`) y el grid carga sus filas; la barra inferior indica el resultado. No uses seeds SQLite como sustituto de esta importación GUI.
4. **Par de idiomas**: en el lateral reemplaza los Edit «Idioma de origen»/«Idioma de destino» con `scripts/set-field.ps1` de la skill, pulsa **Guardar idiomas** y verifica el mensaje. Importa un TXT nuevo y comprueba su par; los documentos anteriores deben conservar el suyo. Cierra/reabre y comprueba los campos persistidos.
5. **Seleccionar documento**: pulsa la fila del documento en **DOCUMENTS** (`document-<id>`); se vuelve activa (borde resaltado) y el grid carga el primer segmento como activo. La barra de progreso inferior muestra traducidos/total.

Un fallo de apertura/importación debe verse en la barra de estado (`application-status`); una apertura fallida no acredita persistencia. Véase [la guía de inicio](../../../../docs/guides/INICIO.md).

## Driving it with cua-driver

Todo pasa por el controlador y los helpers de la skill con el mismo `RunId` y `-WaitSeconds` holgado. Ejecuta `cleanup` solo de tu corrida. `-Action snapshot` devuelve árbol UIA + PNG; prefiere Label/Role únicos y deriva coordenadas de la captura actual cuando el control no sea accesible.

Diálogo nativo: enumera ventanas del MISMO PID con doctor y toma el HWND observado («Abrir»/«Guardar como» en Windows de esta pasada). Sustituye el Edit «Nombre:» con `scripts/set-field.ps1 -WindowId <HWND> -Label 'Nombre:' -Value '<ruta>'` de la skill; confirma con `-Action click -WindowId <HWND> -Label Abrir -Role SplitButton` o `-Label Guardar -Role Button`. Lee los roles actuales: Abrir también expone botones duplicados; Guardar fue único.

Evidencia por subfeature en `output/verification/<RunId>/results.md` con estados `GUI PASS | CORE PASS | FAIL | BLOCKED | NOT RUN`. Cada acción del controlador guarda before/after PNG+JSON automáticamente; añade `snapshot` extra para leer estados intermedios (barra de estado, lista DOCUMENTS).

## Gotchas

- El botón `project-open` está deshabilitado con cambios pendientes u operación ocupada: confirma o deshaz antes de cambiar de proyecto.
- Abrir/crear usa un diálogo de **Guardar**: la carpeta destino debe existir; cancelar no crea el proyecto y no se registra apertura.
- Un proyecto nuevo empieza con `en → es`; cambiarlo afecta próximas importaciones. XLIFF con par declarado incompatible se rechaza sin relabelar: véase `formats.language-conflict`.
- `type` inserta; para sustituir «Nombre:» del diálogo o un idioma usa `scripts/set-field.ps1`. Guardar `en → fr`, rechazar `sample.xlf` en→es y conservar los documentos previos fue ejecutado en `audit-main-gpui-20261004` (base `524046f`).
- Importar/guardar está bloqueado con cambios sin guardar o worker ocupado («Background worker busy…»); espera el estado final antes del siguiente paso.
- TMX no se importa desde **Importar documento** (el filtro no lo incluye): va por **Memoria ▾ → Importar TMX**; véase [memories](memories.md).
- El grid de un documento recién importado carga por páginas (128 filas); con documentos grandes, la primera selección abre el primer segmento, no todo el grid.
