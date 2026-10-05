## Sub-features

- `formats.import-txt`: importar TXT UTF-8; cada línea es un segmento y se conservan terminadores y BOM.
- `formats.import-xliff`: importar XLIFF 1.2 textual o con códigos protegidos, preservando el envelope.
- `formats.import-xliff-segmented`: una fila por `mrk mtype="seg"` de `seg-source`; destinos por `mid` o posición cuando todos carecen de `mid`. Núcleo/XML comprobados, GUI pendiente.
- `formats.import-docx`: importar DOCX con códigos protegidos `<g id="k">`/`<x id="k"/>` en fronteras de estilo, tablas e imágenes.
- `formats.language-conflict`: rechazar XLIFF cuyo `source-language`/`target-language` declarado contradice el par elegido, sin crear documento ni relabelar el envelope.
- `formats.import-reject`: rechazar formatos no soportados y XML inválido con mensaje explícito, sin crear documento.
- `formats.export-document`: exportar el documento activo a un archivo NUEVO (TXT/XLIFF/DOCX), nunca sobre el original ni un destino existente.
- `formats.export-missing-codes`: negativo DOCX — la exportación/confirmación exige todos los códigos del original presentes sin duplicar ni anidar; si faltan, falla con mensaje y no escribe nada.
- `formats.export-safety`: escritura atómica (temporal + `persist_noclobber`) y revalidación XLIFF reparseando; sin archivo parcial ante fallo o cancelación.

## How to get to it (user POV)

Con proyecto abierto, entra en **Archivo**, pulsa **Importar documento** (diálogo nativo con filtros `docx`, `xlf/xliff`, `txt`) y elige el archivo. El documento aparece en **DOCUMENTOS** con su par (`en → es; N segmentos`) y el grid carga sus filas. Resultado observable: conteo de segmentos igual a las líneas TXT, unidades/marcadores XLIFF o párrafos/celdas DOCX.

**Exportar documento** (o Shift+F12) exige documento seleccionado y sin cambios pendientes; el diálogo nativo guarda con filtro `docx/xlf/xliff/txt`. Resultado observable: archivo nuevo; el original queda byte a byte idéntico (compara hash antes/después). Si el destino ya existe o coincide con el original, la operación falla con mensaje y no se escribe nada.

En XLIFF exportado, el envelope se conserva salvo el contenido `target` y los atributos de estado: la exportación escribe `state="translated"` (confirmado) o `state="needs-review-translation"` (borrador) y rebaja `approved="yes"` a `no` si el contenido cambió. En TXT exportado cada destino ocupa su línea con los terminadores originales. En DOCX exportado el paquete original sigue siendo el esqueleto con los destinos reconstruidos; exige los códigos `<g>/<x/>` del original en el destino.

Negativos útiles: extensión no soportada (p. ej. `.rtf`) → mensaje pidiendo TXT UTF-8, XLIFF 1.2 o DOCX; XLIFF con `sub`, `mid` duplicado, correspondencia ambigua de segmentos o XML roto → rechazo explícito sin crear documento; exportar a un destino existente → «destino ya existe»; DOCX/XLIFF cuyo destino perdió un código → rechazo citando el código, sin archivo parcial.

## Driving it with cua-driver

Usa el controlador con un `RunId` nuevo: `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId <id> -WaitSeconds 1200`; el corpus (`source.txt` 5 segmentos, `sample.xlf` 1 unidad) queda en la carpeta del RunId. Snapshot antes/después de cada acción; deriva coordenadas de la captura actual. Los diálogos nativos son ventanas del MISMO PID: enumera `cua-driver call list_windows '{"pid":<PID>}'`, pasa `-WindowId <HWND>`, escribe la ruta en el Edit «Nombre:» (o el campo que muestre la captura) y confirma con el SplitButton **Abrir**/el botón **Guardar** visible. Tras cada diálogo, un `snapshot` nuevo confirma que el grid/LISTA refleja el resultado antes del siguiente paso.

Recorrido mínimo acreditable: importar `source.txt` → editar destino del segmento 1 y confirmar («Confirmar y avanzar») → exportar a nombre nuevo → comparar el TXT resultante con el esperado (destinos con CRLF) y verificar por hash que el original no cambió. Añade: importar `sample.xlf` (1 unidad, confirma el envelope), importar un DOCX de prueba con códigos, exportarlo y reabrirlo, y los negativos de arriba. Verifica archivos exportados con PowerShell/Python (zip válido para DOCX, XML parseable para XLIFF/TMX, contenido esperado para TXT): esa comprobación es parte de la evidencia.

Corpus extra: créalo TÚ dentro de la carpeta del RunId tras el launch (no modifica el proyecto): un `.rtf` de una línea, un `.xlf` con XML roto y un `.xlf` 1.2 con pares `<bpt>/<ept>` válidos como positivo y otro par incoherente como negativo. Para DOCX de prueba, genera un paquete OPC mínimo con Python (zip con `[Content_Types].xml`, `_rels/.rels`, `word/document.xml`) o reutiliza fixtures de verificación previa; documenta cuál usaste.

## Gotchas

- Límites: documento general 256 MiB, DOCX 128 MiB y parte 32 MiB, texto de segmento 4 MiB al importar y 1 MiB persistido; superarlos da error explícito, no truncado.
- XLIFF: versión `1.2`, namespace OASIS o sin namespace como el corpus de launch; namespace declarado incorrecto se rechaza. Admite `g`, `mrk`, `x`, `ph`, `bpt`, `ept`, `it`, `bx`, `ex` y `seg-source`. DTD/entidades externas, `sub` y extensiones inline rechazados; `trans-unit` sin `id`/`source` o con `id` duplicado por `file` se rechazan.
- Segmentación: el estado exportado es conservador por unidad, no por fila; bloqueos distintos entre sus filas impiden exportar. Destinos ausentes/vacíos usan la estructura segmentada del origen; destinos parcialmente segmentados, `mid` duplicados/parciales y pares nativos entre filas se rechazan. Recorrido GUI disponible pero NO ejecutado: importar el `segmented.xlf` de evidencia en proyecto en→es, comprobar 3 filas, editar/confirmar las dos primeras, guardar/reabrir y exportar a nombre nuevo. Contrastar los `mid` y sus textos con un consumidor XML independiente.
- El par de próximas importaciones se guarda desde el lateral; los documentos existentes conservan el suyo. Negativo probado: proyecto en→fr + `sample.xlf` en→es produce «Conflicto de idioma XLIFF» y no añade documento. XLIFF sin atributos de idioma usa el par elegido.
- TXT: un destino con `\r\n` añadido falla la exportación («target TXT no puede agregar líneas estructurales»); BOM y finales mixtos CRLF/LF se preservan.
- DOCX: sin un código del original en el destino la reconstrucción falla («código…»), sin duplicar ni anidar `<g>`; estructuras no soportadas (campos, tracked changes, `w:tab/w:br`, headers/footers, SmartArt) se rechazan con mensaje al importar, no se silencian.
- Exportación atómica: temporal + `persist_noclobber`; cancelar a mitad deja sin destino; la exportación XLIFF revalida el archivo reparseándolo. Nunca sobrescribe original ni destino existente.
- El diálogo Guardar de exportación NO avisa de sobrescritura por sí solo: al elegir un archivo existente, el SO muestra «Confirmar Guardar como»; tras aceptar, la app rechaza con «destino ya existe; elija un archivo nuevo» y no escribe nada. El botón **Guardar** de ese diálogo tiene rol `Button` (no SplitButton como **Abrir**), y Enter en la confirmación del SO elige «No» (cancel).
- `type` inserta en «Nombre:»; sustituye el campo completo con `scripts/set-field.ps1 -WindowId <HWND> -Label 'Nombre:' -Value '<ruta nueva>'` de la skill y verifica lectura exacta antes de Abrir/Guardar.
- Con estado sucio, el PRIMER clic en «Exportar documento» autoguarda y no abre el diálogo; pulsa de nuevo. No pulses «Exportar documento» con un diálogo abierto: se apilan diálogos del mismo PID (bug menor registrado en `output/verification/bugs-formats.md`).
- Pendiente declarado: SDLXLIFF/paquetes, XLIFF 2.x, ampliación de segmentación y recorridos GUI del nuevo soporte inline/segmentado. No hay arrastrar y soltar ni observador de carpeta. Pruebas de núcleo no acreditan GUI ni compatibilidad universal con Word/Trados.
