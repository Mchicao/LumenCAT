# Mapa de verificación de LumenCAT

Lee [la skill](../SKILL.md), ejecuta launch/doctor y elige la receta. Este mapa cubre capacidades **implementadas**, no features futuras de `BRECHAS_TRADOS.md`. Una receta no acredita que se haya ejecutado.

## Índice completo

| Área | Receta | Superficie y prueba |
|---|---|---|
| Proyectos, idiomas, documentos | [project](project.md) | GPUI, selección e importación desde diálogo |
| Cinta, Archivo, paneles y apariencia | [ribbon](ribbon.md) | GPUI, comandos reales y temas persistidos |
| Editor, navegación, confirmación, locks, historial | [editor](editor.md) | GPUI, guardado y reapertura |
| Búsqueda, reemplazo, concordancia | [search](search.md) | GPUI, resultados y undo agrupado |
| QA textual | [qa](qa.md) | GPUI, incidencias del segmento activo |
| Entrada, Unicode, foco, clipboard, atajos | [input](input.md) | GPUI, con bloqueo explícito de transporte |
| Grid, virtualización, progreso | [grid](grid.md) | GPUI, corpus pequeño y 100.000 filas |
| TXT, XLIFF 1.2, DOCX, exportación protegida | [formats](formats.md) | GPUI y artefactos exportados; Word separado |
| TMX, SDLTM opcional, exact/fuzzy, concordancia | [memories](memories.md) | GPUI, import/export y aplicación de coincidencias; SDLTM requiere SDK y su GUI está pendiente |
| Terminología | [terminology](terminology.md) | GPUI: bases, conceptos, reconocimiento y QA; TBX fuera del corte |
| Cierre, interrupción, respaldo, migración | [recovery](recovery.md) | GUI donde existe; API/proceso para resto |

## Precondiciones comunes

1. Construye con `cargo build --locked --bin lumencat`; no uses builds del checkout diario ni `output/lumencat.exe` preexistente.
2. Ejecuta `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId e2e-nuevo`. Obtén PID/ventana y lee `*-ready.png`.
3. El corpus de launch contiene `source.txt` (5 filas), `sample.xlf` (1 unidad) y `sample.tmx` (Hello world → Hola mundo). El proyecto empieza sin documentos; importa mediante UI.
4. Para diálogo nativo enumera `cua-driver list_windows '{"pid":<PID>}'`; selecciona el HWND de ese mismo PID, nunca otra app.
5. Ejecuta doctor antes de conducir. Si `commandLineVerified=false`, comprueba proyecto visible y ruta del manifiesto; no lo presentes como línea de comandos validada.

## Conducción y evidencia

- `-Action snapshot` conserva árbol y PNG; `-Action click/type/key/hotkey` conserva antes, respuesta del driver y después. Usa mismo `RunId`.
- Para diálogo nativo usa `-WindowId <HWND> -Label 'Nombre:' -Role Edit` al escribir y `-Label Abrir -Role SplitButton` al abrir. Las etiquetas dependen del idioma de Windows: léelas de la captura actual.
- Para GPUI sin UIA usa coordenadas de la captura nueva. No pegues coordenadas fijas de otra ejecución.
- Marca cada ID de subfunción como `GUI PASS`, `CORE PASS`, `FAIL`, `BLOCKED` o `NOT RUN`, con expectativa y evidencia. No agregues varios recorridos como PASS si falta una entrada/negativo.
- Cierra con cleanup. La evidencia queda intacta; reapertura con `-Action reopen` conserva el proyecto. No borres corpus ni capturas para limpiar.

## Estado de cobertura

El [contrato de evidencia](../references/coverage.md) y el [registro de ejecución](../../../../docs/technical/VERIFICACION_E2E.md) distinguen cobertura escrita, pruebas de núcleo y recorridos reales. Las funciones no implementadas (TSV/TBX, split/merge, QA global, IA, paquetes Trados, etc.) permanecen fuera de PASS y requieren extender este mapa cuando existan.
