# XLIFF 1.2 con códigos protegidos

Estado: núcleo implementado y comprobado con archivos sintéticos; recorrido GPUI y apertura en Trados de esta nueva capacidad todavía no ejecutados. No acredita paridad documental completa.

## Comportamiento

- Importa grupos anidados `g`/`mrk` y códigos `x`, `ph`, `bpt`, `ept`, `it`, `bx` y `ex`.
- El editor usa sus códigos existentes `<g id="N">`, `</g>` y `<x id="N"/>`. El contenido nativo de `bpt`/`ept`/`ph`/`it` no se expone como texto traducible.
- Exporta recuperando las etiquetas nativas y conservando sus atributos, contenido, notas, cabecera y estructura ajena al destino. Cuando ya hay destino, sus etiquetas nativas tienen preferencia sobre las del origen.
- Una traducción sin cambios conserva el XML original byte a byte. El archivo de entrada nunca se sobrescribe.
- Confirmar verifica conservación de códigos y balance de grupos; exportar también comprueba el orden y anidación de pares nativos. Los segmentos con códigos no se aprenden todavía en la memoria textual.

## Evidencia

`tests/xliff_inline.rs` verifica importación, edición, exportación, reapertura, metadatos, original intacto, grupos anidados, marcadores y códigos nativos. También comprueba que una confirmación inválida no cambia la revisión persistida.

Se ejecutaron primero las tres pruebas contra el código anterior: las tres fallaron por rechazo de etiquetas internas (`logs/tests/xliff-inline-red.log`). Tras la implementación pasaron, junto con formato, Clippy all-targets con warnings como errores y la suite completa (`logs/tests/xliff-inline-all-validation.log`).

No se conduce ni reemplaza la instancia entregada al usuario en `file-menu-final-20261005`. Su ejecutable anterior no incluye esta capacidad.

## Límites abiertos

`seg-source`, `sub`, extensiones inline específicas, comentarios dentro de campos traducibles, literal `<g>`/`<x>` ambiguo, XLIFF 2.x y SDLXLIFF requieren cortes adicionales. No se aplanan silenciosamente. La conservación del balance de grupos al confirmar no sustituye la validación de pares nativos al exportar.

Los límites de documentos y segmentos siguen vigentes; no es un adaptador universal ni una validación completa contra XSD.
