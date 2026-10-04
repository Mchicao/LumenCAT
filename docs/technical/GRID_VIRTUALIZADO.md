# Grid GPUI virtualizado: alcance comprobado

F17.1 sustituye la construcción de todas las filas por `UniformList` de GPUI 0.2.2. El 3 de octubre de 2026 se comprobó un proyecto sintético de 100.000 segmentos en una build debug aislada para Windows. No es un benchmark comparativo ni acredita todo F17.

## Comportamiento

- La lista construye el rango visible que proporciona GPUI, más una fila de medición.
- `cx.defer` solicita páginas de 128 segmentos fuera del render, con precarga de hasta una página antes y después del rango. Las respuestas de generaciones anteriores no entran en el grid actual.
- La caché retiene hasta 1.024 filas próximas al viewport y expulsa las regiones anteriores. El borrador activo vive fuera de esa caché.
- Seleccionar un segmento o restaurarlo desde el historial lo lleva al área visible por índice; al limpiar la búsqueda se recupera el ordinal del activo.

La búsqueda usa la misma lista virtualizada, pero todavía almacena todos sus resultados. La precarga y el límite de filas no garantizan un límite de RAM en bytes: los textos pueden tener tamaños distintos.

## Evidencia

| Check | Resultado observado |
|---|---|
| `cargo test --locked` | PASS: 42 pruebas, incluidas expulsión de caché y conservación de filas cercanas tras saltar entre regiones. |
| `cargo fmt --check` | PASS. |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS. Persiste el aviso previo de incompatibilidad futura de `proc-macro-error2 2.0.1`. |
| `cargo build --locked --bin lumencat --bin lumencat-gpui` | PASS debug; no se reemplazó el EXE de uso diario. |
| `verify_grid` | PASS: importar 100.000 líneas Unicode, confirmar ambos extremos, cerrar y reabrir conserva traducciones. |
| GPUI nativa | PASS: abrir el proyecto grande, seleccionar fila 2, buscar `Segment 100000`, seleccionar el resultado y limpiar la búsqueda muestra las filas 99.995–100.000 con el activo correcto. |
| Edición y confirmación | PASS: añadir ` · F17` en el último destino, observar autoguardado y Draft, confirmar por botón y observar la coincidencia exacta aprendida. |
| Persistencia independiente | PASS: tras cerrar la ventana, conexión SQLite de solo lectura confirma 100.000 filas, integridad `ok` y destino `Última traducción café 世界 · F17`, estado `confirmed`, revisión 3. |

Las acciones usaron `cua-driver` background y lecturas visuales frescas. El transporte devolvió `unverifiable` en clics/escritura; las capturas posteriores acreditan el resultado, no ese mensaje del actuador. No se tomó el foco del usuario.

Artefactos locales ignorados por Git: `logs/tests/f17-final*.log`, `logs/tests/f17-gui-persistence.log` y capturas en `output/verification/f17/`. `large-open.png`, `last-viewport.png`, `last-edited.png` y `last-confirmed.png` registran los estados principales.

## Reproducir el corpus

Desde la raíz, usar un directorio nuevo:

```powershell
cargo run --locked --example verify_grid -- output/verification/f17-nueva-corrida
```

Abrir `large.lcat` con el binario GPUI aislado. Buscar `Segment 100000`, seleccionar la fila, limpiar la búsqueda y editar su destino. Cada acción GUI requiere observación previa y posterior. Cerrar antes de inspeccionar persistencia con otra conexión.

## Pendiente y reversibilidad

No se midieron p50/p95, RAM, cold/warm ni un recorrido continuo de scroll. Atajos físicos, selección parcial, IME, RTL y accesibilidad siguen sin acreditarse; el baseline GPUI solo expone controles de ventana en UIA. La edición background sí quedó comprobada, aunque los atajos combinados fueron rechazados por el transporte en la verificación anterior.

Este corte no cambia esquema, worker, formatos ni la interfaz legacy. Su límite de rollback es la lista/caché en `src/gpui_app/mod.rs`, el ejemplo `verify_grid` y su entrada en `Cargo.toml`, junto a estas afirmaciones documentales; no afecta las entregas previas de idiomas y TM.
