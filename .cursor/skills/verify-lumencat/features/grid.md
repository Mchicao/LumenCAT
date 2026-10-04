## Sub-features

- `grid.rows`: cargar y mostrar las filas bilingües del documento activo en la cuadrícula (`# | Origen | Destino | Estado`, DataGrid «Segmentos bilingües»).
- `grid.select`: activar un segmento con clic en su fila (DataItem `segment-N`) y mover el editor inline (`active-source` + `target-editor`) a esa fila.
- `grid.virtual-scroll`: recorrer documentos grandes con filas virtualizadas/paginadas sin perder la selección.
- `grid.search-results`: mostrar y seleccionar resultados del filtro de búsqueda (ver [search](search.md) para el recorrido completo).
- `grid.progress`: la barra de progreso (`translation-progress`, botón «Ir al primer segmento pendiente») salta al primer segmento sin destino de los cargados.

## How to get to it (user POV)

Importa un documento y selecciónalo en **Documentos**: la tabla central representa sus segmentos con número, origen, destino y estado; `source.txt` produce 5 filas. La activa contiene el editor inline y «Origen del segmento activo». Estado UIA: `confirmado|borrador; editable|bloqueado`; la barra lateral colorea rojo/ámbar/verde/gris. En documentos grandes solo se exponen las filas del viewport, no todas las del corpus.

Haz clic en cualquier fila: queda activa y el editor inline salta a ella. Navega con **Anterior**/**Siguiente** o Ctrl+↑/↓; el scroll del grid recorre documentos grandes con filas virtualizadas. La barra de progreso muestra destinos no vacíos (no confirmados); su botón «Ir al primer segmento pendiente» lleva al primer segmento sin destino que esté cargado en caché. En búsqueda el grid cambia a coincidencias; «Limpiar» o Esc regresa al grid del documento.

## Driving it with cua-driver

Con el `RunId` del controlador, snapshot completo: para `source.txt` deben verse cinco DataItem «Segmento N» y solo un editor activo; comprueba valores de Origen/Destino/Estado. Para `grid.select` pulsa `-Label 'Segmento N' -Role DataItem` y verifica el origen activo. Para progreso, confirma una fila delante de otra sin destino y comprueba el salto por botón. El corpus grande puede prepararse con `cargo run --locked --example verify_grid -- <directorio nuevo>` (solo seed); alternativamente genera 100.000 líneas y las importa por diálogo. Comprueba total en el lateral y render/selección de principio/medio/fin, sin exigir 100.000 elementos UIA simultáneos.

## Gotchas

- `-Label 'Segmento 1' -Role DataItem` es único por documento; `-Label 'Origen'` o `-Label 'Destino'` sin rol se repiten en cada fila inactiva.
- GPUI virtualiza filas (páginas de 128, caché de 1.024); un clic de fila puede caer sobre «Loading segment...» mientras llega la página: espera y toma snapshot nuevo antes de actuar.
- Clic en progreso busca solo entre filas cargadas actualmente; no garantiza el primer vacío de todo el documento.
- La celda «Estado» muestra `confirmado|borrador; editable|bloqueado`: comprueba el sufijo y el botón «Desbloquear», no un supuesto prefijo `locked`.
- Pasada `audit-main-gpui-20261004` (`524046f`): importación GUI de 100.000 líneas y selección/render de filas 50.000/100.000 mediante búsqueda. Seleccionar un resultado vuelve al grid normal; no pulses Limpiar después. Esto no prueba rueda/scroll manual: `cua-driver scroll` background no cambió las filas visibles y quedó BLOCKED.
- El seed `verify_grid.rs` importa a SQLite por API y confirma extremos; jamás acredita render, scroll ni importación GUI.
- `trim_grid_cache` tiene prueba unitaria en `src/gpui_app/mod.rs`; no prueba scroll, selección ni responsive.
