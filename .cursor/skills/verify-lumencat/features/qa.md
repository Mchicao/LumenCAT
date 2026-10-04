## Sub-features

- `qa.textual`: advertir sobre traducción vacía/idéntica, espacios, números, tokens y puntuación del segmento activo.
- `qa.confirmation`: mostrar si el segmento activo sigue sin confirmar (`unconfirmed`).
- `qa.terminology`: mostrar avisos terminológicos del worker para el segmento activo (`term-missing`, `term-forbidden`, `term-ambiguous`, `term-unavailable`).
- `qa.stale-response`: no sustituir el QA del segmento/borrador actual con respuestas de una selección o serial anterior.

## How to get to it (user POV)

Con un segmento activo en el grid, abre la pestaña **Control de calidad** (`qa-tab`) del panel derecho. El panel evalúa el borrador mientras escribes y al cambiar de selección. Cada incidencia se identifica por código: `empty`, `identical`, `double-space`, `edge-space`, `numbers`, `tokens`, `punctuation`, `unconfirmed`, más los terminológicos. Sin incidencias muestra **All QA Checks Passed**. Corregir el texto retira el aviso; confirmar retira `unconfirmed` si no aparece otra incidencia. Los avisos son informativos: no corrigen texto ni bloquean confirmar.

Casos reproducibles con el corpus de launch (`source.txt`, 5 filas): destino vacío → `empty` + `unconfirmed`; destino idéntico al origen → `identical`; quitar el número del origen («Privacy notice 42.» sin «42») → `numbers`; doble espacio → `double-space`; cambiar puntuación final → `punctuation`; origen con URL/variable (`Visit https://example.test/{name}.`) sin token en destino → `tokens`.

## Driving it with cua-driver

Con el `RunId` del controlador: activa un segmento, `-Action click -Label 'Control de calidad' -Role TabItem` (la acción `select` del TabItem se acepta por ruta accessibility) y verifica el panel QA POR CAPTURA (PNG del snapshot), no por árbol UIA: las alertas no se exponen como elementos. Escribe el caso en el destino con `-Action type -Label 'Destino' -Role Edit` y espera un snapshot posterior con captura: el worker descarta respuestas de serial/selección anterior (`qa.stale-response`), así que no concluyas que QA no actualizó sin esperar a la captura siguiente. Registra código visible → corrección → desaparición del aviso. Mantén separados QA, estado confirmado y el indicador de guardado.

## Gotchas

- QA textual implementa listas de números, detección aproximada de tokens (`://`, `@`, `${`, `{`, `%`), espacios en extremos, doble espacio y puntuación final exacta. No es corrector ortográfico ni compara longitudes: el mensaje sin incidencias habla de «identity issues».
- `unconfirmed` es estado, no error de texto; un destino vacío puede acumular `empty` y `unconfirmed`.
- Terminología sin base evaluada produce `term-unavailable`; no lo presentes como QA limpio.
- El identificador del QA es la TabItem «Control de calidad» (`qa-tab`, acción select); «Memoria de traducción» (`tm-tab`) es la otra pestaña del panel.
- `tests/qa.rs` cubre números, tokens y CJK limpio; `src/worker.rs` cubre terminología que excede límites. Ninguno acredita esta vista.
