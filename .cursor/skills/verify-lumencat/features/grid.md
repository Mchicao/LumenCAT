## Sub-features

- `grid.rows`: cargar y mostrar las filas bilingües del documento activo en la cuadrícula (`# | Origen | Destino | Estado`, DataGrid «Segmentos bilingües»).
- `grid.select`: activar un segmento con clic en su fila (DataItem `segment-N`) y mover el editor inline (`active-source` + `target-editor`) a esa fila.
- `grid.virtual-scroll`: recorrer documentos grandes con filas virtualizadas/paginadas sin perder la selección.
- `grid.search-results`: mostrar y seleccionar resultados del filtro de búsqueda (ver [search](search.md) para el recorrido completo).
- `grid.progress`: la barra de progreso (`translation-progress`, botón «Ir al primer segmento pendiente») salta al primer segmento sin destino de los cargados.

## How to get to it (user POV)

Importa un documento y selecciónalo en **Documentos**: la tabla central carga una fila por segmento con número, origen, destino y estado; el corpus de launch (`source.txt`) produce 5 filas. La fila activa se distingue porque contiene el editor de destino inline y su origen se lee «Origen del segmento activo»; la celda «Estado» de cada fila indica `confirmed|draft|locked` + `editable`, y la barra lateral colorea rojo/ámbar/verde/gris.

Haz clic en cualquier fila: queda activa y el editor inline salta a ella. Navega con **Anterior**/**Siguiente** o Ctrl+↑/↓; el scroll del grid recorre documentos grandes con filas virtualizadas. La barra de progreso muestra destinos no vacíos (no confirmados); su botón «Ir al primer segmento pendiente» lleva al primer segmento sin destino que esté cargado en caché. En búsqueda el grid cambia a coincidencias; «Limpiar» o Esc regresa al grid del documento.

## Driving it with cua-driver

Con el `RunId` del controlador: `-Action snapshot` y verifica contra el árbol UIA que el número de filas DataItem «Segmento N» coincide con el corpus, que `Origen`/`Destino`/`Estado` exponen los valores correctos y que solo la fila activa contiene las Edit `active-source`/`target-editor`. Para `grid.select` usa `-Action click -Label 'Segmento N' -Role DataItem` y confirma en el snapshot posterior que el origen activo cambió. Para `grid.progress` clic en el botón «Ir al primer segmento pendiente» con al menos un segmento confirmado delante de un pendiente y comprueba el salto. El corpus de 100.000 filas se siembra con `cargo run --locked --example verify_grid -- <directorio nuevo>` (solo seed) y se abre por GUI; recorre principio/medio/fin y comprueba que la selección no se pierda.

## Gotchas

- `-Label 'Segmento 1' -Role DataItem` es único por documento; `-Label 'Origen'` o `-Label 'Destino'` sin rol se repiten en cada fila inactiva.
- GPUI virtualiza filas (páginas de 128, caché de 1.024); un clic de fila puede caer sobre «Loading segment...» mientras llega la página: espera y toma snapshot nuevo antes de actuar.
- Clic en progreso busca solo entre filas cargadas actualmente; no garantiza el primer vacío de todo el documento.
- La celda «Estado» muestra pares `estado; editable` — el lock se verifica con el prefijo, no con `editable`.
- El seed `verify_grid.rs` importa a SQLite por API y confirma extremos; jamás acredita render, scroll ni importación GUI.
- `trim_grid_cache` tiene prueba unitaria en `src/gpui_app/mod.rs`; no prueba scroll, selección ni responsive.
