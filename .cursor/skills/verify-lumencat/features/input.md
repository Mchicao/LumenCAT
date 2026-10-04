## Sub-features

- `input.target`: escribir/borrar en el editor de destino inline (`target-editor`, Edit «Destino» de la fila activa) mediante teclado real.
- `input.clipboard-unicode`: introducir texto con teclado/portapapeles conservando caracteres Unicode (diacríticos, CJK, tokens) en el destino.
- `input.focus-shortcuts`: dirigir teclas (←/→, Inicio/Fin, Backspace/Supr, Enter) al campo enfocado sin que los atajos globales las intercepten.
- `input.search-replace`: escribir consulta y reemplazo (recorrido completo en [search](search.md)).
- `input.project-languages`: editar códigos de idioma y guardarlos (recorrido en [project](project.md)).

## How to get to it (user POV)

El editor de destino vive dentro del grid: activa una fila y escribe directamente en la fila activa (`target-editor`). El campo acepta texto multilínea (Enter inserta salto de línea), muestra palabras y caracteres Unicode, y la fila pasa a ámbar (draft) con autoguardado hacia «Guardado». ←/→ mueven el cursor, Inicio/Fin saltan a los extremos, Backspace/Supr borran; Ctrl+A selecciona todo y escribir lo sustituye; Ctrl+C/V copian/pegan (pegar normaliza CRLF a LF). Con el segmento bloqueado, el destino rechaza la edición.

Otras cajas comparten el mismo `InputModel`: búsqueda («Buscar en origen y destino», `search-query`), «Reemplazar por...», idiomas de proyecto y formularios de memoria/terminología. Enter cambia de comportamiento por campo: en destino inserta salto de línea, en consulta busca, en idiomas guarda el par. Esc devuelve el foco al destino desde búsqueda.

Para Unicode real, escribe (o pega) una cadena con diacríticos, CJK y símbolos —p. ej. `Unicode café 世界`— y verifica el texto final y su guardado. Resultado negativo: escribir sobre destino bloqueado no cambia el texto.

## Driving it with cua-driver

Con el `RunId` del controlador: `-Action type -Label 'Destino' -Role Edit -Text '<texto>'` escribe en el editor inline por la ruta UIA; `-Action type -X <x> -Y <y>` sobre el cuadro del destino ejercita la ruta de teclado real (synthetic_events) — acredita `input.target` con entrada física sintética. `-Action key -Keys backspace` o `-Keys home` ejercitan el foco. Captura snapshot antes/después y verifica el `value` del Edit UIA, no la posición gráfica del cursor. Para Unicode, escribe la cadena y compara dentro del mismo PowerShell que leyó el JSON fijando `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` antes de invocar al controlador/driver: con el codepage OEM heredado los JSON de evidencia salen con doble codificación y la comparación falla aunque la app esté correcta. El pegado Ctrl+V y los atajos con modificador se intentan con `-Action hotkey`; si el transporte degrada el combo (aparece una `c` literal), marca ese camino BLOCKED, deshaz con el botón y acredita el campo con escritura directa.

## Gotchas

- `-Label 'Destino' -Role Edit` solo existe en la fila activa; en filas inactivas «Destino» es una celda DataItem de solo lectura.
- El campo destino bloqueado puede seguir expuesto como Edit en UIA pero la app ignora la edición; verifica por texto final, no por ausencia de error del driver.
- Cada hotkey con modificador rechazado/degradado puede insertar una `c` literal en el campo enfocado; comprueba el destino tras cada intento y deshaz.
- Los offsets del puente nativo convierten UTF-16 ↔ UTF-8: valida emoji/suplementarios y diacríticos por el texto final.
- La colocación de cursor por clic y la selección parcial con ratón no están implementadas (`character_index_for_point` devuelve `None`); no las declares probadas.
- Ctrl+A + escritura sustituye el contenido; Ctrl+C/V es clipboard del sistema — no introduzcas contenido sensible.
- `components::InputModel` no tenía prueba E2E previa; `editing::tests` cubre conversión de offsets, no conducción GPUI.
