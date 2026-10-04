# Interfaz egui legacy (`--legacy-egui`)

Receta de la superficie mantenida en `src/app.rs` (eframe 0.32 con AccessKit). Se conduce con el mismo controlador usando `-Surface legacy`, que añade `--legacy-egui` a `--project`. Estado de esta receta: **recorrido parcialmente ejecutado** el 3 de octubre de 2026 en `output/verification/e2e-rc-001/`; el smoke quedó **BLOCKED** en la entrada de texto por dos defectos documentados en [bugs](../../../../output/verification/bugs-recovery.md): render blanco en lanzamiento sin activación y rechazo de teclado sintético en background.

## Sub-features

Reprueba del 4 de octubre, base `524046f`, `audit-main-legacy-20261004`: apertura/cierre/reapertura observados por UIA, PNG blanco y entrada de ruta BLOCKED. SetValue y `type` por elemento devolvieron accessibility/unverifiable sin cambiar el campo; no apareció Importar documento. No son los mismos hechos que `delivery_failed` de la corrida histórica; importación y funciones dependientes siguen sin acreditarse. No se tomó foreground.

- `legacy.surface-open`: ventana «LumenCAT · traducción bajo tu control» (1400×900 mín. 980×640) con proyecto abierto por `--project`; mensaje de estado «Proyecto local abierto…» o de recuperación. — **GUI PASS** (estado verificado por árbol UIA; sin canal visual por el bug de render).
- `legacy.import-txt`: importación por campo de ruta + botón **Importar documento** (sin diálogo nativo; acepta TXT/XLIFF/DOCX/TMX según extensión). — **BLOCKED** (no se puede teclear la ruta en background).
- `legacy.edit-confirm`: editor TARGET multiline del segmento activo + **Confirmar · Ctrl+Enter** o botón; estado mostrado por segmento. — **NOT RUN** (depende de importar).
- `legacy.export`: campo de ruta nueva + **Exportar documento** / **Exportar TMX**. — **NOT RUN**.
- `legacy.tm-references`: panel derecho «Memoria de traducción» con **Insertar referencia TM** por coincidencia. — **NOT RUN**.
- `legacy.search-concordance`: búsqueda source/target con **Buscar**, **Siguientes resultados** y **Concordancia**. — **NOT RUN**.
- `legacy.draft-recovery`: en error de guardado, **Reintentar guardado pendiente** y **Recuperar borrador / recargar desde disco…** con diálogo propio (`src/app.rs:758-778`). — **NOT RUN** (exige forzar fallo de guardado).
- `legacy.close-gate`: el cierre espera el commit; sin cambios pendientes sale limpio (`close()` hace checkpoint y marca `clean=1`, `src/storage.rs:437-448`). — **GUI PASS** (cierre por botón Cerrar vía UIA; proceso terminó sin ForceStop) y sin aviso de recuperación en la reapertura.

## How to get to it (user POV)

El binario es el mismo `lumencat`; la superficie legacy se entra con el flag `--legacy-egui` (el controlador lo añade con `-Surface legacy`). Layout en una ventana: **toolbar** superior con tres filas — proyecto (campo de ruta + «Abrir / crear» solo antes de abrir; el lanzamiento verificado ya abre el proyecto), idiomas Origen/Destino + campo de ruta de importación + «Importar documento»/«Importar TMX», y campo de ruta de exportación + «Exportar documento»/«Exportar TMX»/«Deshacer · Ctrl+Z»/«Rehacer · Ctrl+Y». **Panel izquierdo** «Documentos» con un botón por documento; **panel derecho** «Memoria de traducción»; **panel central** con búsqueda, lista de segmentos (`N  estado  origen  →  destino`), editor del segmento activo (SOURCE de solo lectura, TARGET editable, «Copiar source», «Confirmar · Ctrl+Enter», casilla «Bloqueado») y «Control de calidad local» con los avisos del segmento; **barra inferior** de estado con guardado pendiente y, en error de guardado, los botones de recuperación de borrador. Atajos: Ctrl+Enter confirmar, Alt+↑/↓ navegar, Ctrl+Z/Y historial (`src/app.rs:488-500`).

## Driving it with cua-driver

Con el controlador (`-RunId` propio, `-Surface legacy` en launch; doctor/snapshot/cleanup idénticos a GPUI):

1. **Launch** — `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId <id> -Surface legacy -WaitSeconds 1200`. Ejecutado: PID/ventana correctos y mensaje «Proyecto local abierto. Los cambios se guardan de forma transaccional.» legible en el árbol UIA (elemento Text de la barra de estado).
2. **Snapshot** — el árbol UIA de egui es utilizable (AccessKit): textos, campos Edit con su contenido o placeholder, y SOLO los botones habilitados. Úsalo para verificar estado (barra de estado, valores de campos) aunque la captura salga en blanco.
3. **Importar TXT** — escribir la ruta en el campo con placeholder `Ruta de TXT / XLIFF 1.2 / DOCX / TMX` y pulsar **Importar documento** (aparece en el árbol solo cuando el campo tiene texto). **BLOCKED**: `-Action type` (por elemento o por píxel) y `-Action key` devuelven `delivery_failed` con escalación a `foreground`; sin foreground autorizado no hay entrada de teclado. Evidencia: `*-action-type.json`, `*-action-key.json` de `e2e-rc-001`.
4. **Editar y confirmar** — click de píxel sobre la lista para seleccionar segmento, click en TARGET, escribir, confirmar. **NOT RUN**: sin importación no hay segmentos.
5. **Exportar** — ruta nueva en el campo de exportación + **Exportar documento**; verificar archivo y contenido. **NOT RUN**.
6. **Cleanup** — `-Action cleanup` hace click por UIA en **Cerrar** (etiqueta ASCII) y el proceso termina sin ForceStop. Ejecutado: cierre limpio; `-Action reopen` posterior abrió sin aviso de recuperación.

Coordenadas de la captura fresca (snapshot `20261003-042803-0691-before-snapshot.png`, ventana en x=189,y=182, 1402×932; los `frame` del árbol son pantallas absolutas: resta el origen de la ventana para `-X/-Y`):

| Control | frame pantalla | píxel ventana-local |
|---|---|---|
| Campo ruta de importación (Edit, placeholder «Ruta de TXT…») | 418,243 388×18 | 229,61 |
| Campo idioma Origen / Destino | 245,243 / 357,243 | 56,61 / 168,61 |
| Campo ruta de exportación | 198,268 388×18 | 9,86 |
| Campo búsqueda «Buscar source / target» | 555,298 248×18 | 366,116 |
| Lista de segmentos (zona) | desde 388,344 | 199,162 |

Las posiciones de «Importar documento», «Exportar documento», TARGET y botones del editor dependen del contenido: derívalas del snapshot de la ejecución, nunca de esta tabla.

## Gotchas

- **BUG (reportado): render en background.** Lanzada sin activación (`launch_app` con SW_SHOWNOACTIVATE), la ventana egui no presenta ningún frame: captura y pantalla real en blanco (`20261003-042539-6604-ready.png`, `20261003-desktop-crop.png`); al maximizar, el área nueva sale negra con un bloque blanco obsoleto (`20261003-043748-2052-after-click.png`). El árbol UIA sigue vivo. Lanzada con activación normal sí pinta (`output/verification/legacy-egui.png` de una corrida manual). Mientras no se corrija, la verificación legacy no tiene canal visual en background.
- **BUG (reportado): teclado sintético rechazado.** `-Action type/key/hotkey` sobre la ventana egui termina en `delivery_failed` con petición de foreground; los clicks sintéticos (mouse) sí se entregan (el maximizado por click funcionó). GPUI sí aceptó escritura en background en `e2e-trados-001`: el rechazo es específico de esta superficie. No es un fallo acreditado de la app: el transporte exige foreground que esta verificación no puede tomar.
- **Codificación de consola.** El controlador integrado fija UTF-8. Antes de llamadas Cua directas fija también `[Console]::OutputEncoding` y `$OutputEncoding` a `[System.Text.Encoding]::UTF8`; verifica etiquetas/valores en el mismo PowerShell, sin extrapolar el mojibake histórico.
- Los botones deshabilitados no aparecen en el árbol UIA de egui: si un botón esperado falta, comprueba su condición de habilitado (por ejemplo «Importar documento» exige campo de ruta con texto y sin cambios pendientes) antes de hablar de bug.
- Legacy no usa diálogo nativo de archivos: las recetas GPUI de importación por diálogo (`-WindowId … -Label 'Nombre:' -Role Edit`) no aplican aquí; la importación es campo de ruta + botón.
- El editor legacy es de un segmento activo a la vez y la lista muestra `LOCK/OK/·` por fila; el QA es «Control de calidad local» bajo el editor, alimentado por el mismo worker que GPUI.
