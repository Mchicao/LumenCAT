# Editor y ejecutables Windows — 30 septiembre 2026

## Cambios

El ejecutable `lumencat` usa el subsistema Windows GUI. La consola provenía del subsistema Console de los binarios anteriores; no era un proceso auxiliar. Se conserva el registro JSON en `%LOCALAPPDATA%/LumenCAT/logs/app`.

La pantalla GPUI elimina el panel permanente de atajos y las combinaciones de las etiquetas de los botones. Deshacer/rehacer usan flechas dibujadas de 18 px dentro de controles de 30 px con tooltip. Origen y destino se editan/consultan en columnas paralelas; las operaciones TMX se revelan desde «Memoria». El grid, las coincidencias TM y QA permanecen disponibles.

Se añaden Ctrl+Insert/Alt+Insert, Ctrl+T, Ctrl+1…9, F3, Ctrl+Alt+Enter y Shift+F12 a las funciones existentes. Ctrl+S funciona también con foco en búsqueda/reemplazo. Se conserva la navegación Ctrl+↑/↓ y los atajos anteriores. Fuente: [guía oficial RWS del perfil predeterminado de Studio](https://www.trados.com/media/images/translate-file-in-10-easy-steps-2022-rws-en-1_tcm234-213936.pdf).

## Evidencia

- `cargo check --locked --bins`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` y `git diff --check`: aprobados.
- `cargo test --locked --lib --test storage`: seis pruebas aprobadas.
- Compilación release aprobada en esa entrega, incluido el binario principal `lumencat`. Artefactos de entonces copiados a `output/`; hashes históricos en `output/verification/windows-gui-build.json`. El código vigente define un único lanzador de escritorio.
- `scripts/utils/verify_windows_exe.ps1`: el artefacto final tiene subsistema GUI (2). El mismo comando rechaza `.cache/verification/lumencat-final.exe`, cuyo subsistema Console es 3.
- Lanzamiento nativo con Cua Driver: el EXE anterior abrió GUI y consola; el EXE final abrió una sola ventana GUI.
- Prueba sobre `.cache/verification/trados-shortcuts.lcat`, copia del proyecto de ejemplo: Ctrl+Insert y Alt+Insert copiaron origen; Ctrl+Enter confirmó y avanzó; Ctrl+Alt+Enter confirmó sin avanzar; Ctrl+↑/↓ cambió fila; Ctrl+Z deshizo una confirmación. Se verificaron estados/destinos en SQLite y navegación visual.
- En el EXE release, Ctrl+Insert guardó «Above» y Ctrl+T restauró «Arriba» con origen TM; Ctrl+1 aplicó la coincidencia y F3 abrió concordancias para «Above». Capturas en `output/verification/trados-*.png`.
- Inspección visual a 1280×850: flechas compactas, ausencia de atajos permanentes, columnas de origen/destino y panel TM/QA visibles. Detector Impeccable sin hallazgos para los dos archivos GPUI modificados.

## Alcance

Se validaron los atajos principales de funciones existentes en esa fecha, no paridad completa con Trados. Confirmar todavía no actualizaba la TM; posteriormente se añadió aprendizaje y terminología. QuickPlace inserta el siguiente tag. Shift+F12 y Ctrl+2…9 estaban conectados, pero no se probaron individualmente con diálogos o nueve coincidencias. Consulta las guías actuales para el alcance vigente.

## Actualización 3 de octubre de 2026 — cuadrícula como editor

El «Translation Studio» con tarjetas ORIGEN/DESTINO se eliminó: el grid bilingüe es ahora el editor, al modo de Trados. Cada fila muestra `# | origen | destino | barra de estado` (rojo sin destino, ámbar borrador, verde confirmado, gris bloqueado); la fila activa contiene el editor de destino dentro del propio grid y su celda de origen expone «Origen del segmento activo». El texto se envuelve (filas de altura variable), la fila activa se resalta con borde lateral y la toolbar «Acciones del segmento» (Anterior, Siguiente, Copiar origen, Bloquear, Etiqueta, Confirmar y avanzar) envuelve en varias líneas. Los AutomationId del modelo anterior se conservan (`target-editor`, `active-source`, `segment-*`, etc.), por lo que la guía COMPUTER_USE.md sigue siendo válida. Verificación de la nueva UI en `output/verification/e2e-trados-001/results.md` y en [VERIFICACION_E2E](VERIFICACION_E2E.md).
