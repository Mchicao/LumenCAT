# Editor y ejecutables Windows — 30 septiembre 2026

## Cambios

Los ejecutables `lumencat` y `lumencat-gpui` usan el subsistema Windows GUI. La consola provenía del subsistema Console de los binarios anteriores; no era un proceso auxiliar. Se conserva el registro JSON del ejecutable principal en `%LOCALAPPDATA%/LumenCAT/logs/app`.

La pantalla GPUI elimina el panel permanente de atajos y las combinaciones de las etiquetas de los botones. Deshacer/rehacer usan flechas dibujadas de 18 px dentro de controles de 30 px con tooltip. Origen y destino se editan/consultan en columnas paralelas; las operaciones TMX se revelan desde «Memoria». El grid, las coincidencias TM y QA permanecen disponibles.

Se añaden Ctrl+Insert/Alt+Insert, Ctrl+T, Ctrl+1…9, F3, Ctrl+Alt+Enter y Shift+F12 a las funciones existentes. Ctrl+S funciona también con foco en búsqueda/reemplazo. Se conserva la navegación Ctrl+↑/↓ y los atajos anteriores. Fuente: [guía oficial RWS del perfil predeterminado de Studio](https://www.trados.com/media/images/translate-file-in-10-easy-steps-2022-rws-en-1_tcm234-213936.pdf).

## Evidencia

- `cargo check --locked --bins`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` y `git diff --check`: aprobados.
- `cargo test --locked --lib --test storage`: seis pruebas aprobadas.
- `cargo build --locked --release --bin lumencat --bin lumencat-gpui`: aprobado. Artefactos copiados a `output/`; hashes en `output/verification/windows-gui-build.json`.
- `scripts/utils/verify_windows_exe.ps1`: ambos artefactos finales tienen subsistema GUI (2). El mismo comando rechaza `.cache/verification/lumencat-final.exe`, cuyo subsistema Console es 3.
- Lanzamiento nativo con Cua Driver: el EXE anterior abrió GUI y consola; cada EXE final abrió una sola ventana GUI.
- Prueba sobre `.cache/verification/trados-shortcuts.lcat`, copia del proyecto de ejemplo: Ctrl+Insert y Alt+Insert copiaron origen; Ctrl+Enter confirmó y avanzó; Ctrl+Alt+Enter confirmó sin avanzar; Ctrl+↑/↓ cambió fila; Ctrl+Z deshizo una confirmación. Se verificaron estados/destinos en SQLite y navegación visual.
- En el EXE release, Ctrl+Insert guardó «Above» y Ctrl+T restauró «Arriba» con origen TM; Ctrl+1 aplicó la coincidencia y F3 abrió concordancias para «Above». Capturas en `output/verification/trados-*.png`.
- Inspección visual a 1280×850: flechas compactas, ausencia de atajos permanentes, columnas de origen/destino y panel TM/QA visibles. Detector Impeccable sin hallazgos para los dos archivos GPUI modificados.

## Alcance

Se validan los atajos principales de funciones existentes, no paridad completa con Trados. Confirmar no actualiza automáticamente la TM y avanza al segmento siguiente; QuickPlace inserta el siguiente tag. No se implementan terminología, ortografía ni configuración de atajos. Shift+F12 y Ctrl+2…9 están conectados, pero no se probaron individualmente con diálogos o nueve coincidencias. La interfaz egui anterior se conserva como alternativa; esta simplificación visual se aplica a GPUI.
