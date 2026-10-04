## Sub-features

- `editor.select-navigate`: activar segmentos desde el grid (DataItem `segment-N`), botones **Anterior**/**Siguiente** (`segment-previous`/`segment-next`) y Ctrl+↑/↓.
- `editor.edit-autosave`: escribir el destino en el editor inline de la fila activa (`target-editor`, Edit «Destino») con autoguardado visible (StatusBar `application-status` «Guardado» / «Guardando…») y Ctrl+S como forzado.
- `editor.confirm-next`: **Confirmar y avanzar** (`segment-confirm-next`); confirmar sin avanzar (Ctrl+Alt+Enter); negativos: destino vacío o segmento bloqueado no confirman.
- `editor.copy-source`: **Copiar origen** (`source-copy`) y Ctrl+Insert / Alt+Insert / Alt+C.
- `editor.insert-tag`: **Insertar siguiente etiqueta protegida** (`insert-next-tag`), Ctrl+, y Ctrl+Alt+←/↑/→/↓; agotadas muestra «Todas las etiquetas ya están insertadas».
- `editor.lock-toggle`: **Bloquear** (`segment-lock`) y Ctrl+L alternan bloqueo; un destino bloqueado rechaza edición, copia y TM.
- `editor.history`: **Deshacer**/**Rehacer** (`undo`/`redo`) y Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z sobre el destino.
- `editor.tm-apply`: aplicar coincidencia de memoria desde el panel **Coincidencias TM** (Ctrl+T o Ctrl+1…9); el destino queda marcado como procedente de memoria.
- `editor.export`: **Exportar documento** (Shift+F12) eligiendo salida nueva sin sobrescribir el original.

## How to get to it (user POV)

Abre un proyecto e importa un documento con **Importar documento**. El editor es una cuadrícula estilo Trados: cada fila es `# | Origen | Destino | Estado` (DataGrid «Segmentos bilingües»). La fila activa muestra el origen leído «Origen del segmento activo» (`active-source`) y el editor de destino inline (`target-editor`) dentro del propio grid; las filas inactivas exponen celdas de solo lectura «Origen»/«Destino»/«Estado». La barra lateral de estado colorea la fila: rojo sin destino, ámbar borrador, verde confirmado, gris bloqueado; la celda «Estado» combina `confirmed|draft|locked` con `editable`.

Sobre el grid está la toolbar **Acciones del segmento**: Anterior, Siguiente, Copiar origen, Bloquear, Insertar siguiente etiqueta protegida, Confirmar y avanzar. En la toolbar de proyecto están Deshacer y Rehacer. La barra inferior informa «Guardado» cuando todo está persistido; tras editar muestra el guardado en curso y vuelve a «Guardado». Ctrl+S fuerza el guardado; un error conserva el texto en el editor.

Selecciona una fila (clic en ella) o usa Anterior/Siguiente; Ctrl+↑/↓ navegan por teclado. Escribe en el destino: la fila pasa a ámbar (draft). **Confirmar y avanzar** exige destino no vacío y segmento desbloqueado; confirma en verde y salta al siguiente ordinal. **Copiar origen** vuelca el texto fuente al destino (también Ctrl+Insert). Ctrl+L alterna bloqueo; **Bloquear** sobre un segmento con borrador lo congela y su celda pasa a gris. **Insertar siguiente etiqueta protegida** añade el siguiente código `<g>`, `</g>` o `<x/>` pendiente del origen; si no queda ninguno aparece «Todas las etiquetas ya están insertadas» (con TXT plano sin códigos, ese es el camino observable). Deshacer/Rehacer revierten/restauran la última edición del destino. El panel derecho **Coincidencias TM** aplica la primera sugerencia (Ctrl+T o Ctrl+1…9). Shift+F12 abre **Exportar documento** si no hay cambios pendientes.

## Driving it with cua-driver

Usa el proyecto aislado del controlador con un `RunId` propio (`launch` → `doctor` → acciones → `cleanup`; para durabilidad `reopen` reutiliza el proyecto). La UIA del grid expone la estructura completa: botones con `id` (`segment-previous`, `segment-next`, `source-copy`, `segment-lock`, `insert-next-tag`, `segment-confirm-next`, `undo`, `redo`, `translation-progress`, `application-status`), Edit «Destino» (`target-editor`) y Edit «Origen del segmento activo» (`active-source`) solo en la fila activa. El controlador acepta `-Label <etiqueta> -Role <rol>` cuando la etiqueta identifica un elemento único (p. ej. `-Label 'Destino' -Role Edit`); si hay duplicados usa `-X/-Y` derivados de la captura fresca o `-Label 'Segmento N' -Role DataItem` para la fila. `type` escribe en el foco/campo indicado; verifica el valor leído del árbol UIA y el StatusBar tras cada edición. Para durabilidad: `cleanup` con cierre normal, `-Action reopen` y comprueba destino/estado persistidos en un snapshot nuevo.

Los atajos con modificador (Ctrl+↑/↓, Ctrl+Insert, Ctrl+Alt+Enter, Ctrl+L, Ctrl+Z…) se intentan con `-Action hotkey` en background; si el transporte devuelve `background_unavailable` se registra BLOCKED para ese atajo, sin imputarlo a la app y sin usar foreground. Los botones equivalentes acreditan la función. Compara valores con no-ASCII (café, 世界) dentro del mismo PowerShell que leyó el JSON, nunca por pipes de Git Bash.

## Gotchas

- `-Label 'Destino' -Role Edit` solo coincide con el editor inline de la fila activa; las celdas «Destino» inactivas son DataItem, no Edit. `-Label 'Origen'` sí es ambiguo (una celda por fila inactiva).
- Los snapshots before/after de una acción `-Label` quedan FILTRADOS a ese elemento: para acreditar el efecto completo usa `-Action snapshot` sin etiqueta entre pasos.
- Cada hotkey con modificador que el transporte no entrega degrada a una `c` literal escrita en el campo enfocado (sucia el borrador): deshaz con el botón tras cada intento y registra el atajo como BLOCKED, no como bug de la app.
- La celda «Estado» mezcla idiomas (`confirmed; editable` vs `draft; bloqueado`): verifica el bloqueo por el sufijo y el badge «Locked»/«Desbloquear» de la toolbar, reportado como hallazgo cosmético.
- Al bloquear, el botón «Bloquear» desaparece y aparece «Desbloquear»; Copiar origen, Etiqueta y Confirmar y avanzar se deshabilitan. El Edit «Destino» sigue expuesto en UIA pero rechaza escritura.
- La confirmación requiere destino no vacío y segmento desbloqueado; vacío/bloqueado muestra aviso y no confirma. Ctrl+Enter no equivale a guardar borrador.
- Confirmar aprende texto plano solo si hay memoria de escritura compatible; DOCX con códigos valida pero no aprende esos códigos como TMX textual.
- Un destino bloqueado no admite edición, copia ni aplicación de TM; el botón permanece disponible para desbloquear (etiqueta «Desbloquear»).
- La navegación guarda primero los cambios pendientes; un error de guardado no debe tratarse como confirmación durable.
- Con TXT plano `insert-next-tag` no inserta nada: «Todas las etiquetas ya están insertadas» aparece en la StatusBar. Para insertado real necesitas un documento con códigos protegidos (XLIFF/DOCX).
- Los códigos visibles son texto protegido, no QuickPlace con candidatos; no alteres su orden en un recorrido de exportación.
- Selección parcial y colocación de cursor por clic siguen sin implementarse según `docs/guides/INICIO.md`; no declares IME/RTL/accesibilidad probados.
- `tests/storage.rs`, `tests/tm_learning.rs`, `tests/docx.rs` y `editing::tests` cubren el núcleo, no este recorrido GPUI.
