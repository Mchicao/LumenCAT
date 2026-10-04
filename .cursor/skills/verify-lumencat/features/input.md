## Sub-features

- `input.target`: escribir/borrar en el editor inline (`target-editor`, Edit «Destino»); registrar la ruta utilizada, separando UIA, teclado sintético y teclado físico.
- `input.clipboard-unicode`: introducir texto con teclado/portapapeles conservando caracteres Unicode (diacríticos, CJK, tokens) en el destino.
- `input.focus-shortcuts`: dirigir teclas (←/→, Inicio/Fin, Backspace/Supr, Enter) al campo enfocado sin que los atajos globales las intercepten.
- `input.search-replace`: escribir consulta y reemplazo (recorrido completo en [search](search.md)).
- `input.project-languages`: editar códigos de idioma y guardarlos (recorrido en [project](project.md)).

## How to get to it (user POV)

El editor de destino vive dentro del grid: activa una fila y escribe directamente en la fila activa (`target-editor`). El campo acepta texto multilínea (Enter inserta salto de línea), muestra palabras y caracteres Unicode, y la fila pasa a ámbar (draft) con autoguardado hacia «Guardado». ←/→ mueven el cursor, Inicio/Fin saltan a los extremos, Backspace/Supr borran; Ctrl+A selecciona todo y escribir lo sustituye; Ctrl+C/V copian/pegan (pegar normaliza CRLF a LF). Con el segmento bloqueado, el destino rechaza la edición.

Otras cajas comparten el mismo `InputModel`: búsqueda («Buscar en origen y destino», `search-query`), «Reemplazar por...», idiomas de proyecto y formularios de memoria/terminología. Enter cambia de comportamiento por campo: en destino inserta salto de línea, en consulta busca, en idiomas guarda el par. Esc devuelve el foco al destino desde búsqueda.

Para Unicode real, escribe (o pega) una cadena con diacríticos, CJK y símbolos —p. ej. `Unicode café 世界`— y verifica el texto final y su guardado. Resultado negativo: escribir sobre destino bloqueado no cambia el texto.

## Driving it with cua-driver

Con el `RunId` del controlador, `-Action type -Label 'Destino' -Role Edit -Text '<texto>'` intenta la ruta accesible; `-Action type -X <x> -Y <y>` intenta entrada sintética. Lee la ruta devuelta: ninguna acredita teclado físico. `-Action key -Keys backspace` o `-Keys home` ejercitan el foco si se entregan. Verifica valor y captura posterior, no solo la respuesta del driver. Fija UTF-8 en `[Console]::OutputEncoding` y `$OutputEncoding` antes de llamadas directas y compara Unicode dentro del mismo PowerShell. Ctrl+V y modificadores se intentan con `-Action hotkey`; rechazo/degradación se registra BLOCKED, se repite doctor y se restaura un estado conocido. SetValue mediante `scripts/set-field.ps1` sustituye un campo completo sin acreditar clipboard ni teclado.

## Gotchas

- `-Label 'Destino' -Role Edit` solo existe en la fila activa; en filas inactivas «Destino» es una celda DataItem de solo lectura.
- El campo destino bloqueado puede seguir expuesto como Edit en UIA pero la app ignora la edición; verifica por texto final, no por ausencia de error del driver.
- Cada hotkey con modificador rechazado/degradado puede insertar una `c` literal en el campo enfocado; comprueba el destino tras cada intento y deshaz.
- Los offsets del puente nativo convierten UTF-16 ↔ UTF-8: valida emoji/suplementarios y diacríticos por el texto final.
- La colocación de cursor por clic y la selección parcial con ratón no están implementadas (`character_index_for_point` devuelve `None`); no las declares probadas.
- Ctrl+A + escritura sustituye el contenido; Ctrl+C/V es clipboard del sistema — no introduzcas contenido sensible.
- `components::InputModel` no tenía prueba E2E previa; `editing::tests` cubre conversión de offsets, no conducción GPUI.
- En `audit-main-gpui-20261004` (`524046f`) el teclado sintético convirtió 😀 en U+F600; SetValue conservó exactamente `café😀`. Registra la ruta sintética BLOCKED y no concluyas fallo de Unicode de la app sin otra ruta independiente. SetValue acredita el campo, no teclado físico ni clipboard.
