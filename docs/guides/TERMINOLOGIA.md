# Terminología local

La pestaña **Términos** permite guardar vocabulario del cliente y reconocerlo en el segmento activo. Una base terminológica no es una memoria de traducción: añadir un concepto no modifica destinos, no confirma segmentos ni aprende TM.

## Añadir un concepto en GPUI

1. Abre un proyecto y selecciona **Términos** en el panel derecho.
2. Escribe un nombre y pulsa **Crear base**. Si ya existen bases, selecciona aquella donde guardarás el concepto.
3. Pulsa **Añadir concepto**, introduce origen y equivalencia destino. El par corresponde al documento activo, o a la configuración del proyecto si no hay documento.
4. El botón de estado recorre **preferido → permitido → prohibido**. Elige si se distinguen mayúsculas y añade notas opcionales.
5. Pulsa **Guardar concepto**. Puedes ocultar el formulario para consultar las coincidencias; el texto permanece para no borrar una entrada en curso.

Cada guardado crea un concepto. La interfaz inicial añade dos expresiones; el núcleo admite conceptos multilingües con varias variantes, dominio y procedencia. La gestión/edición avanzada y añadir variantes a un concepto existente todavía no tienen interfaz. No uses conceptos separados como sustituto de variantes: sentidos distintos pueden generar ambigüedad deliberada.

**Activar/Desactivar** decide qué bases se consultan, sin borrar sus datos. Seleccionar una base decide dónde se añade el siguiente concepto; no desactiva las otras bases. Se consultan todos los conceptos activos con expresiones para el par exacto, ignorando mayúsculas en los códigos de idioma pero sin mezclar variantes regionales.

## Reconocimiento y QA

| Resultado | Significado |
|---|---|
| Coincidencia | Secuencia completa de palabras; `art` no coincide dentro de `party`. Se muestran base, concepto, procedencia, variantes y notas. |
| `term-missing` | La fuente contiene el término y el destino no contiene ninguna equivalencia preferida/permitida del concepto. No exige todas sus alternativas. |
| `term-forbidden` | La fuente activa el concepto y el destino contiene una de sus formas prohibidas. |
| `term-ambiguous` | Varios conceptos coinciden en el mismo rango fuente. Se muestran las alternativas sin escoger sentido ni imponer sus reglas contradictorias. |
| `term-unavailable` | El reconocimiento falla o excede sus límites. Terminología queda no evaluada; QA textual sigue funcionando. |

Los avisos no corrigen texto ni bloquean confirmación. GPUI descarta respuestas ligadas a una selección/borrador anterior. Legacy también recibe los avisos QA del mismo worker, pero no dispone del gestor de bases/conceptos.

El motor usa tokens alfanuméricos Unicode y marcas combinantes. Espacios y puntuación separan tokens; no se exige la misma puntuación entre palabras de una frase. NFC y minúsculas son auxiliares: no cambian los datos guardados. No elimina tildes ni aplica flexión/morfología, y no segmenta lenguas sin espacios. En DOCX ignora los códigos `<g>/<x/>`; en TXT/XLIFF textual los trata como texto literal. Los rangos son bytes UTF-8 del original; aún no se resaltan en el editor.

## Límites y recuperación

- Hasta 10.000 expresiones por proyecto, 1–64 por concepto y 512 bytes por expresión. Dominio: 256 bytes; notas: 8 KiB; procedencia: 1 KiB.
- Consultas de hasta 1 MiB por texto y 512 coincidencias/incidencias. Si se exceden, no se presenta un resultado parcial como completo. El escaneo es inicial, sin latencias garantizadas; bases grandes requieren medición e indexación futura.
- Añadir un concepto es atómico y cancelable en el núcleo. La entrada manual GUI no tiene botón de cancelación; no forma parte del undo/redo de segmentos.
- TSV/TBX, preview/import/export, búsqueda manual independiente del segmento, excepciones por concepto/segmento, selección parcial y resaltado siguen pendientes. No se admite `.sdltb` ni se anuncia compatibilidad MultiTerm.

Schema v5 crea estas tablas al migrar, con respaldo SQLite previo. Un binario v4 no abre el proyecto actualizado; volver a él exige una copia del respaldo v4, sin sobrescribir el proyecto ni descartar su terminología. Ver [recuperación](RECUPERACION.md).

## Evidencia del corte F03.1

El 3 de octubre de 2026, build debug Windows aislada: **47 pruebas**, formato y Clippy aprobados. Las pruebas cubren variantes/idiomas, reinicio, bases activas, cancelación, rollback inducido, frases/ambigüedad, Unicode/case/offsets y límites que conservan QA textual. Migraciones v1–v4 conservan memoria, texto, cursor y respaldo.

En GPUI background se creó `Cliente F03`, se añadió `Segment 1 → traducción` como destino prohibido, se observó reconocimiento y `term-forbidden` en QA, y se desactivó la base. El destino del documento siguió confirmado y no cambió. Tras cerrar, SQLite de solo lectura verificó schema 5, integridad, un concepto/dos expresiones y estado desactivado; contra el respaldo v4 conservó 1 documento, 100.000 segmentos, 2 entradas de historial y 2 TU, además de textos extremos, historial, settings y TM completos.

Evidencia local ignorada por Git: `logs/tests/f03-final.log`, `f03-clippy.log`, `f03-build.log`, `f03-gui-persistence.log`; capturas en `output/verification/f03/recognized.png`, `qa-forbidden.png` y `disabled.png`. Los clics y la entrada background fueron verificados por capturas posteriores; no acreditan atajos físicos, IME/RTL, accesibilidad ni legacy interactiva.

La frontera de rollback es el módulo/modelo de terminología, migración v5, comandos compartidos, integración GPUI/input y QA legacy, sus pruebas y afirmaciones documentales. No se convierte un proyecto v5 a v4 revirtiendo código; conserva su respaldo anterior.
