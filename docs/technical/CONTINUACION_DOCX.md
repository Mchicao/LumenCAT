# Continuación de compatibilidad Word

## Objetivo

Probar y corregir el flujo real de LumenCAT: importar Word con tablas e imágenes, traducirlo completo en la interfaz, exportarlo y abrir el resultado en Microsoft Word conservando formato, tablas e imágenes. En la sesión de validación del 30-09-2026, push y operaciones Git destructivas requieren confirmación explícita nueva; la autorización histórica no aplica.

## Estado logrado en esta pasada

- Formato mixto entre runs e imágenes inline ya **no se rechazan**: los runs contiguos con `w:rPr` idéntico forman regiones; las fronteras de estilo se exponen como códigos `<g id="k">…</g>` y las imágenes estáticas embebidas como `<x id="k"/>` (decisión 011 en `docs/architecture/DECISIONES.md`). El editor GPUI existente (`insert_next_tag`) ya maneja estos códigos.
- Un DOCX sin texto traducible (solo imágenes) importa con cero segmentos y exporta con todas las partes internas idénticas; el ZIP puede cambiar de hash al recomprimirse. Antes se rechazaba.
- La exportación valida los códigos (todos presentes, sin duplicar/anidar/desconocidos, texto por región) y un round-trip tolerante al reordenado; rechaza en vez de exportar algo corrupto.

## Evidencia con Word real (16.0.20326, COM)

- `scripts/utils/verify_docx.rs` traduce **todos** los segmentos vía SQLite (`store.edit`), exporta y reimporta; falla con salida ≠ 0 si hay REJECTED. Resultado: PASS `mammoth-tables` (6 segs), `mammoth-tiny-picture` (0 segs), `mammoth-underline` (1 seg), `word-inspection` (20 segs); partes ajenas a `word/document.xml` byte-idénticas.
- `scripts/utils/verify_word.ps1` compara original vs traducido abriéndolos en Word: `word-inspection` mantiene 2 páginas, 33 párrafos, mismas tablas (filas/celdas/anchos incluida la combinada de 435 y la franja de 18), mismas 3 imágenes (90×90 y 65×65 ×2), 0 shapes flotantes; texto en español sin códigos `<g>`/`<x>` filtrados; Word no modificó los archivos (hash). `mammoth-underline` y `mammoth-tiny-picture` también pasan.
- `scripts/utils/render_pdf.ps1` (WinRT, requiere `powershell.exe` 5.1) renderiza los PDF exportados por Word a PNG. Comparación de malla de tinta 24×24 (`logs/tests` y `.cache/verification/docx-e2e`): página 1 difiere 5,3 % promedio por reflow del texto español más largo; las zonas densas de imágenes (97 % tinta) reaparecen desplazadas verticalmente ~2 filas sin perderse; página 2 sin celdas con diferencia >35 %. Estructura y geometría intactas.

## Límites actuales declarados

Rechazados con mensaje explícito: marcado entre los runs de un párrafo (bookmarks intercalados dentro de la secuencia), runs con texto e imagen juntos, `w:tab`/`w:br`/`w:cr`, hyperlinks, campos, revisiones, content controls, `w:pict`/VML, `mc:AlternateContent`, cuadros de texto/gráficos/SmartArt dentro de dibujos, texto en headers/footers/notas/otras historias, numeración con `lvlText` textual, macros/embeddings, Strict OOXML. Los atributos `w:rsid*` de runs reescritos se pierden (sin efecto en Word). El texto alternativo de imágenes (`wp:docPr@name/descr`) no se traduce.

## Reproducir la verificación

```powershell
New-Item -ItemType Directory -Path .cache/verification/docx-e2e-next -Force
Copy-Item output/verification/word-complex/sources/*.docx .cache/verification/docx-e2e-next/  # quitar *-translated previos
cargo run --locked --example verify_docx -- .cache/verification/docx-e2e-next
& scripts/utils/verify_word.ps1 -Source <original> -Translated <exportado> -OutputDirectory <dir-nuevo>
powershell.exe -File scripts/utils/render_pdf.ps1 -Pdf <pdf> -OutputDirectory <png-dir>
```

Usar un directorio nuevo en cada pasada: el exportador nunca sobrescribe destinos existentes.

## Siguiente trabajo

1. La validación GUI por UIA está completada en la sección siguiente. Queda probar los atajos físicos con autorización de foco, además de TextPattern/selección parcial, lectores de pantalla y rendimiento del grid grande.
2. Cortes de bajo riesgo siguientes: `w:tab`/`w:br` como códigos, hyperlinks con relación preservada, headers/footers como historias con segmentos propios.
3. Corpus ampliado: listas numeradas, footnotes, campos de página, texto no latino y traducciones mucho más largas, cada uno con fixture Word real y comparación estructural + render.

## Validación GPUI iniciada el 30-09-2026 (parcial)

Base: `main`, commit `68cd8e8`, árbol limpio al iniciar. Corrida nueva: `.cache/verification/gpui-docx-20260930-175722/`; capturas: `output/verification/gpui-docx-20260930-175722/`. Fuentes originales sin modificar. No se hicieron cambios de código, commit ni push.

| Escenario | Estado | Evidencia / alcance |
| --- | --- | --- |
| Lanzar `output/lumencat-gpui.exe` | PASS | `01-launch.png`: ventana sin consola, grid bilingüe, editor origen/destino, TM/QA y botones visibles. |
| Crear proyecto desde el diálogo | BLOQUEADO | Clic background en «Open / Create Project» no tuvo efecto; `02-project-dialog.png` muestra la misma pantalla. UIA expone solo cinco controles de la barra de título. Se pidió autorización para entrada foreground por acción con restauración de foco, exigida por la skill `cua-driver`; aún pendiente. Esto es un límite del transporte, no prueba de un bug de la app. |
| Importar complejo, badges y 20 segmentos en GUI | PENDIENTE | No ejecutado por el bloqueo anterior. |
| Traducir, insertar códigos, confirmar, historial y buscar en GUI | PENDIENTE | No ejecutado; tampoco se verificó eliminación/corrupción de códigos desde el editor real. |
| Exportar desde GUI y comprobar con Word | PENDIENTE | No existe un exportado producido por esta sesión GUI. |
| Humo GUI underline y documento sin texto | PENDIENTE | No ejecutado. |

Comprobaciones complementarias ejecutadas, sin sustituir la GUI:

- PASS: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` (29 pruebas), `cargo build --locked --release`. Cargo advierte incompatibilidad futura de `proc-macro-error2 2.0.1`; no falla los checks actuales.
- PASS: `scripts/utils/verify_windows_exe.ps1` sobre ambos binarios release. El SHA256 de `output/lumencat-gpui.exe` coincide con la build nueva: `1683FA2C1394BC3C7A594F3CDFCA0DDE059BE5E4100E884DBBDE0BA636E315D6`; no fue necesario reemplazar el EXE abierto.
- PASS: `cargo run --locked --example verify_docx -- .cache/verification/gpui-docx-20260930-175722/parser` con copias de los tres fixtures: complejo 20 segmentos, underline 1, tiny-picture 0; salida 0, sin REJECTED.
- PASS complementario: `verify_word.ps1` sobre `parser/word-inspection-translated.docx`, generado vía SQLite, con informes/PDF en `parser-word/`: Word 16.0.20326, 2 páginas y 33 párrafos en ambos, estructura y geometría idénticas, sin códigos filtrados ni modificaciones por Word.
- PDF renderizado con `powershell.exe` 5.1 en `parser-word/source-png/` y `translated-png/`. Inspección visual directa de página 1: título y tabla traducidos, «críticos» rojo/negrita y tres imágenes conservados; reflow vertical por texto más largo. Malla 24×24 con el helper existente: diferencia media 5,3 % en página 1 y 2,8 % en página 2; no es un criterio automático de paridad visual.

Ese bloqueo histórico queda resuelto por el puente UIA de la actualización siguiente. La autorización de foco sigue pendiente solo para los atajos físicos.

## Actualización GPUI, Computer Use y validación GUI del 30-09-2026

GPUI y `gpui_platform` actualizados al [HEAD upstream verificado](https://github.com/zed-industries/zed/commit/8e7fbcc131ec0f41cc4e78fbc8c50ef41f47b382), fijado por SHA completo en Cargo.toml/Cargo.lock. El crate sigue declarando 0.2.2; la publicación crates.io no incluye este backend AccessKit Windows. Se usa `gpui_platform::application()`, se ajusta la API de foco y se añaden roles, nombres, IDs, estados y acciones accesibles a los controles existentes. Ver decisión 012 y [guía de Computer Use](../guides/COMPUTER_USE.md).

Se elimina el texto permanente de guardado/privacidad/SQLite/IA y el hint redundante bajo el progreso. Solo quedan conteo/barra de progreso y avisos de trabajo pendiente o error. «Etiqueta» y «Concordancia» invocan funciones existentes sin necesitar atajos físicos. SetValue comparte autoguardado/historial/QA; no hay API de automatización separada ni acceso directo al proyecto para traducir desde GUI.

Correcciones pequeñas encontradas: selección de filas por Invoke (Row no tiene SelectionItem en AccessKit Windows), origen de solo lectura con valor íntegro, limpieza del editor al seleccionar documentos de cero segmentos, IDs distintos para aplicar matches, navegación Tab, progreso deshabilitado sin pendientes, clasificación de errores separada de guardado y eliminación de error obsoleto después de una operación válida. `lumencat-gpui.exe` ahora acepta `--project` además de la ruta posicional: antes interpretaba el flag como nombre de proyecto.

Corrida: `.cache/verification/gpui-uia-20260930-184653/`. Proyecto nuevo creado desde el diálogo real: `ui-project.lcat`. Capturas numeradas en `output/verification/gpui-uia-20260930-184653/`. Se operó el EXE GPUI mediante cua-driver en background, principalmente Invoke/SelectionItem/ValuePattern. No se usó foreground. Los intentos de combos PostMessage se verificaron como ineficaces y se restauró el texto alterado; no cuentan como prueba del atajo.

| Escenario | Resultado | Evidencia / alcance |
| --- | --- | --- |
| Lanzar EXE y crear proyecto nuevo | PASS | `01-launch-uia.png`, `02-project-dialog-uia.png`; diálogo Guardar operado por UIA y archivos de proyecto comprobados. La versión final vuelve a abrirlo mediante `--project`. |
| Importar complejo: 20 segmentos y badges `<g>` | PASS | `03-complex-import.png`, `05-protected-translation.png`; filas y texto íntegro accesibles. Las tres imágenes del complejo están en párrafos sin texto, por lo que ese fixture no genera `<x>` en sus segmentos. |
| Traducir/insertar etiquetas/confirmar y avanzar | PASS por UIA | Se escribieron y confirmaron los 20 segmentos desde el editor. En segmento 2 se insertaron `<g id="1">` y `</g>` con «Etiqueta» y se conservó «críticos» dentro del formato. Lectura SQLite únicamente como comprobación: documento 1, veinte `confirmed`, ningún destino vacío. |
| Historial, bloqueo y origen inmutable | PASS | `06-history-redo.png`, `07-locked-uia.png`; undo/redo restauró valores distintos. Origen y destino bloqueado rechazaron SetValue; lectura posterior intacta y confirmar deshabilitado. |
| Buscar, TM y QA | PASS | `09-search.png`, `10-qa-missing-codes.png`, `13-tm-match.png`, `14-concordance.png`; consulta encontró segmento 2, TMX temporal importó una TU, coincidencia exacta aplicada al destino y concordancia recuperada. Pestaña QA mostró avisos del segmento. |
| Borrar códigos e intentar exportar | PASS (rechazo esperado) | `11-export-rejected-missing-code.png`: «traducción DOCX sin todos los códigos `<g>` del original»; `missing-codes.docx` no existe. Códigos restaurados y exportación posterior correcta. El intento anterior con destinos vacíos fue otro rechazo, sin archivo parcial. |
| Exportar complejo y comprobar Word | PASS | `word-ui-final-translated.docx`, `word-ui-final/word-report.json`, `26-final-editor-no-footer.png`. Word 16.0.20430: 2 páginas, 33 párrafos, 2 tablas, 3 imágenes, 1 sección, 0 shapes flotantes; tablas/celdas/anchos e imágenes idénticos, ningún código filtrado, hashes antes/después intactos. La primera exportación también pasó Word 16.0.20326. |
| Humo underline: un segmento, formato reordenado | PASS | `15-underline-import.png` a `17-underline-exported.png`; «El árbol del `<g id="1">atardecer</g>`», confirmado/exportado desde GUI. `underline-ui/word-report.json`: 1 página/1 párrafo, formato subrayado visible en el render. |
| Humo tiny-picture: cero segmentos | PASS | `18-zero-segments.png` a `20-final-zero-no-footer.png`; editor vacío, escritura rechazada, acciones de traducción deshabilitadas, exportación sin bloqueo. `tiny-ui/word-report.json` y `zip-parts.json`: estructura e imagen conservadas; todas las entradas ZIP tienen payload idéntico, aunque el hash del contenedor difiere. |
| Caso adicional con texto e imagen `<x/>` | PASS | Copia derivada `inline-text-picture-v2.docx` creada solo en `.cache`, sin alterar fuentes. `21-inline-image-badge.png`, `22-inline-image-translated.png`; «Etiqueta» insertó `<x id="1"/>`, traducción/exportación GUI, Word 16.0.20430 y renders con imagen inline conservada y sin códigos. |
| Recuperar estado tras rechazo | PASS | `24-recovery-error.png` y `25-recovery-success-no-footer.png`: fixture temporal con bookmark intercalado rechazado, luego `recovery.txt` importado; mensaje de éxito deja de aparecer como error. |
| Ctrl+, / Ctrl+Enter, entrada foreground | PENDIENTE | Los combos background perdieron Ctrl: Ctrl+, introdujo coma y Ctrl+Enter introdujo salto sin avanzar. Texto restaurado. La skill cua-driver exige autorización para escalar foreground; no se recibió. Las acciones equivalentes se probaron por botones UIA. |

### Comprobaciones y evidencia visual

- PASS sobre el código final: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` (29 pruebas), `cargo build --locked --release`; logs en `logs/tests/gpui-uia-*.log`.
- PASS: E2E en `parser-final-release`, cinco fixtures aceptados (20/1/0/6 segmentos originales y 1 segmento derivado), sin REJECTED y salida 0. Esta prueba traduce vía SQLite como comprobación complementaria; las traducciones del flujo GUI se introdujeron solo por UIA.
- Ambos EXE publicados coinciden con release y tienen subsistema Windows GUI verificado. SHA256 final GPUI: `4CDF66B595C42D388A06C1FB5E25BDED47FADEE9E2EBD602B4CC9D5493B8830E`.
- `scripts/utils/verify_gpui_uia.ps1` consulta la app real mediante cua-driver; resultados y snapshots JSON en `uia-initial`, `uia-tags`, `uia-locked`, `uia-all`, `uia-zero` y `uia-final`. El cliente .NET UIAutomationClient no enumeró los controles en esta máquina; no se usa como prueba de ausencia de UIA.
- PDF de Word renderizados con Windows PowerShell 5.1. Inspección visual directa: badges de la app, «críticos» rojo/negrita, tablas/celdas, imágenes, subrayado de «atardecer» e imagen entre texto en el caso `<x/>`. Hay reflow por longitud de traducción; no se afirma igualdad píxel a píxel.
- Malla de tinta 24×24 sobre el complejo final (`word-ui-final/grid-compare.txt`): diferencia media 5,1 % en página 1 y 2,5 % en página 2. Las siete celdas >35 % de diferencia en página 1 corresponden principalmente al desplazamiento de las imágenes/filas; es evidencia complementaria, no un umbral universal de aceptación.
- Todos los hashes de DOCX fuente bajo `output/verification/word-complex/sources/` siguen iguales. No se hicieron commit ni push.

### Límites y siguiente trabajo

El flujo importar→editar con códigos→confirmar→exportar queda verificado mediante el EXE y Word para estas muestras. La prueba de los atajos físicos sigue pendiente de autorización; no se declara cumplida esa parte del escenario original. No hay paridad completa con Trados ni compatibilidad Word universal.

Pendientes concretos: TextPattern/selección parcial del texto, IME/RTL y lectores de pantalla; grid sin virtualización; nombres largos de documentos pueden desbordar el badge de formato; resultados de búsqueda ya cargados pueden conservar el target/estado anterior hasta repetir la búsqueda (el editor activo sí muestra el valor actual); QA trata dígitos de IDs de códigos como números. Estos límites no impidieron exportar y conservar las muestras probadas. Los formatos rechazados declarados anteriormente siguen sin cambios.

Incidencia de arranque: la prueba con `--project` del binario anterior creó un proyecto vacío `C:/Users/matia/--project` y sus archivos auxiliares. Se comprobó que contiene cero documentos y se respaldó íntegro en `launcher-argument-error/`; el perfil PowerShell bloqueó su eliminación fuera de `C:/Proyectos`. La limpieza de esos cuatro archivos queda pendiente; no se eludió esa protección.
