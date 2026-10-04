# Terminología local en GPUI

La pestaña visual **Términos** (TabItem UIA «Terminología») administra bases y conceptos del proyecto. Implementación: `src/gpui_app/terminology.rs`, `src/terminology.rs` y esquema v5 en `src/storage/migrations.rs`. No es interoperabilidad MultiTerm.

## Sub-features

- `terminology.create-base` y `terminology.select-base`: crear y elegir la base destino en la GUI.
- `terminology.add-concept`: formulario con expresión origen + equivalencia destino, estado y notas.
- `terminology.recognition`: lista «Reconocidos en el origen» para el segmento activo.
- `terminology.qa-issues`: avisos terminológicos en la pestaña QA, con negativos prohibido/ambiguo.
- `terminology.persistence`: bases, conceptos y estado de activación sobreviven a `-Action reopen`.
- `terminology.enable`: desactivar/activar sin borrar conceptos; una base desactivada deja de reconocer.

## How to get to it (user POV)

Abre **Términos**, escribe «Nombre de base terminológica» y pulsa **Crear base**. Selecciona su botón y **Añadir concepto**. Completa «Término de origen», «Equivalencia de destino» y notas opcionales; **Destino: preferido** alterna a permitido, prohibido y preferido. **Mayúsculas: ignorar/distinguir** configura el reconocimiento. **Guardar concepto** lo persiste para el par del documento activo; **Ocultar formulario** permite ver resultados.

La lista reconoce frases completas en las bases activas del par del documento. Muestra concepto, procedencia, equivalentes y estado; **Desactivar** elimina resultados sin borrar datos. QA muestra `term-missing`, `term-forbidden`, `term-ambiguous` o `term-unavailable` cuando corresponde. Los avisos no bloquean la confirmación.

## Driving it with cua-driver

Con `source.txt` importado, abre la pestaña por `-Action click -Label Terminología -Role TabItem`. Reemplaza campos completos con `scripts/set-field.ps1` de la skill. Crea una base, añade `Privacy → privacidad`, alterna el destino hasta **prohibido** y guarda. En una fila cuyo destino contenga «privacidad», comprueba por captura reconocimiento y QA `term-forbidden`; los textos de esas tarjetas no están en el árbol UIA. Desactiva: la lista queda sin reconocidos. Cierra/reabre, comprueba el botón **Activar**, actívala y verifica que concepto e incidencia vuelven.

Recorrido ejecutado en `audit-main-gpui-20261004` (`524046f`): creación, concepto prohibido, reconocimiento, QA, desactivación y persistencia/reactivación. Ambigüedad, notas, mayúsculas y equivalentes permitidos no se condujeron en esa pasada. `cargo test --locked --test terminology` acredita solo el núcleo.

## Gotchas

- Con varias bases, «Activar»/«Desactivar» deja de ser único: resuelve el control desde la captura actual.
- No hay inserción del equivalente mediante botón en el panel, flexión lingüística ni segmentación para idiomas sin espacios. Un concepto guardado no es una unidad TM.
- TBX, MultiTerm y `.sdltb` quedan fuera del corte. Consulta [TERMINOLOGIA](../../../../docs/guides/TERMINOLOGIA.md) para los límites del modelo; no extrapoles variantes multilingües del núcleo al formulario bilingüe.
