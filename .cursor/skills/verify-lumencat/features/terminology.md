# Estado: termbase NO implementada — sin verificación GUI posible

Actualizado: 3 de octubre de 2026. La terminología de LumenCAT **no está implementada**: no existe termbase, reconocimiento de términos, panel de términos ni QA terminológico. [`docs/technical/BRECHAS_TRADOS.md`](../../../../docs/technical/BRECHAS_TRADOS.md) la registra como brecha **Ausente** (fila «Bases terminológicas») y su paquete **F03** es el plan pendiente para implementarla. El QA actual (`src/qa.rs`) solo evalúa vacío, identidad, espacios, dígitos, tokens, puntuación y confirmación del segmento activo; no evalúa vocabulario.

La receta anterior de este archivo describía una pestaña **Términos**, formularios de conceptos, bases activables y avisos `term-missing`/`term-forbidden`/`term-ambiguous`/`term-unavailable` que **no existen** en el árbol: no hay `src/terminology.rs`, ningún panel en `src/gpui_app/`, ni tablas de conceptos en el esquema de `src/storage.rs`. Era una guía escrita por adelantado, no una capacidad: no la uses como base de recorridos ni la cuentes como cobertura.

## Sub-features

Ninguna verificable hoy. Cuando se implemente el paquete F03, este archivo debe definir al menos estos ID antes de ejecutar cualquier PASS:

- `terminology.create-base` y `terminology.select-base`: crear y elegir la base destino en la GUI.
- `terminology.add-concept`: formulario con expresión origen + equivalencia destino, estado y notas.
- `terminology.recognition`: lista «Reconocidos en el origen» para el segmento activo.
- `terminology.qa-issues`: avisos terminológicos en la pestaña QA, con negativos prohibido/ambiguo.
- `terminology.persistence`: bases, conceptos y estado de activación sobreviven a `-Action reopen`.

## How to get to it (user POV)

Hoy no hay ruta de usuario: ningún panel, pestaña, menú ni comando de terminología existe en GPUI ni en legacy (`src/app.rs` recibe avisos del worker, pero no hay proveedor de términos). Cuando F03 exista, la entrada esperada es un panel de recursos con pestaña propia; la receta se reescribirá entonces con controles reales y capturas de esa build, no con este texto.

## Driving it with cua-driver

Nada que conducir en esta entrega: cualquier intento sería fabricación. Cuando exista, cada PASS exigirá el nivel [GUI PASS](../references/coverage.md): entrada real por teclado/ratón en la build nueva, capturas antes/después, reconocimiento visible con el segmento activo y persistencia comprobada con `-Action reopen`. Las pruebas del núcleo que acompañen a F03 (`cargo test --locked --test <suite>`) serán evidencia auxiliar, nunca GUI PASS. Los negativos mínimos a cubrir entonces: término prohibido en destino, término ambiguo, y desactivar la base vaciando la lista sin borrar datos.

## Gotchas

- No resucites la receta antigua de este archivo: sus controles, coordenadas y referencias (`docs/guides/TERMINOLOGIA.md`, `tests/terminology.rs`, `src/gpui_app/terminology.rs`) no existen. Verifica el árbol antes de reactivar cualquier texto.
- La fila de `features/README.md` que lista esta receta («Bases, conceptos, reconocimiento, estados · GPUI; variantes avanzadas solo núcleo») quedó redactada como si la función existiera; el estado real es **pendiente (F03)** y el índice debe leerse con ese matiz hasta corregirse.
- TBX, MultiTerm, `.sdltb` y variantes multilingües avanzadas siguen fuera del producto incluso después de un primer corte F03: requieren sus propios paquetes y no se acreditan por tener una termbase local básica.
- Si aparece una implementación parcial, verifica solo lo expuesto por la GUI de esa build y marca el resto NOT RUN; no agregues cobertura por código núcleo.
