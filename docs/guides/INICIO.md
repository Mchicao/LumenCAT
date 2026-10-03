# Usar LumenCAT 0.1

Primer slice funcional en desarrollo; no sustituye todavía una herramienta CAT profesional en entregas de cliente. Sin cuenta, red ni IA.

## Ejecutar

Desde `C:/Proyectos/LumenCAT`, `cargo run --locked --release --bin lumencat`. El ejecutable standalone está en `output/lumencat.exe` cuando se publica la build local. Se puede ejecutar desde cualquier directorio. Opcional: `lumencat.exe --project C:/ruta/proyecto.db`. No se requiere Rust para ejecutar ese EXE en Windows compatible, pero no hay instalador firmado ni distribución verificada en otra máquina.

## Interfaz GPUI (predeterminada)

`cargo run --locked` abre GPUI. Para la interfaz anterior: `cargo run --locked -- --legacy-egui`. Ambas aceptan `--project C:/ruta/proyecto.lcat`.

- Abre/crea un proyecto, importa documentos con los botones del encabezado y selecciona un documento en la barra lateral. Los proyectos nuevos GPUI empiezan en→es; puedes cambiar y guardar los idiomas de próximas importaciones. Los documentos existentes conservan su par.
- Origen y destino aparecen en paralelo, con memoria y QA a la derecha. La barra usa flechas compactas para deshacer/rehacer; sus nombres y atajos aparecen al pasar el ratón. Importar/exportar TMX está bajo «Memoria».
- Clic en una fila o Ctrl+↑/↓ para navegar; el destino recibe foco. Ctrl+Enter confirma y avanza; Ctrl+Alt+Enter confirma sin avanzar. Ctrl+Insert o Alt+Insert copia el origen; Ctrl+L bloquea/desbloquea. Se conservan ↑/↓, Alt+↑/↓ y Alt+C como alternativas.
- Ctrl+T aplica la primera coincidencia TM; Ctrl+1…9 aplica la coincidencia correspondiente. F3 busca concordancias del destino seleccionado completo o, en su defecto, del origen; con el foco en búsqueda usa esa consulta. Ctrl+S guarda; Shift+F12 abre la exportación cuando no hay cambios pendientes.
- Ctrl+, o Ctrl+Alt+flecha inserta la siguiente etiqueta `<g>`, `</g>` o `<x/>` que falta. Los badges son representación del texto; no amplían el subset XLIFF/DOCX del importador.
- Ctrl+F busca en origen/destino, con alcance de segmento o documento. Ctrl+H abre reemplazo literal, sensible a mayúsculas, solo en destinos. El reemplazo de documento es transaccional, ignora segmentos bloqueados y conserva tags. Ctrl+Z/Y deshace/rehace cada segmento del historial.
- El progreso cuenta destinos no vacíos en todo el documento; no equivale a confirmación ni QA aprobada. Clic en la barra va al primer pendiente cargado. El editor muestra palabras y caracteres Unicode; el límite UTF-8 sigue siendo 1 MiB. El porcentaje TM aplicado se muestra durante la sesión; no se persiste como metadata.
- Autoguardado periódico sin otra pulsación. El cierre espera el commit y el cierre del worker; un error impide cerrar para conservar el borrador. La recuperación TXT mediante diálogo pertenece a la interfaz anterior.

GPUI virtualiza las filas del grid y solicita páginas según el área visible, con precarga alrededor y hasta 1.024 filas en caché. El editor conserva el texto completo del segmento activo. La selección parcial por ratón y la posición del cursor por clic siguen pendientes. El puente de entrada nativa está conectado; no se afirma validación completa de IME, RTL o accesibilidad. La tabla bilingüe representa segmentos, no el diseño visual de Word.

Los atajos principales se basan en el [perfil predeterminado documentado por RWS](https://www.trados.com/media/images/translate-file-in-10-easy-steps-2022-rws-en-1_tcm234-213936.pdf). No hay paridad completa con Trados: confirmar y avanzar va al segmento siguiente, no al siguiente pendiente; QuickPlace inserta el siguiente tag, sin desplegar candidatos. Confirmar aprende texto plano en la memoria de escritura seleccionada; guardar borradores no aprende. Ver [memorias](MEMORIAS.md). Terminología, corrección ortográfica y configuración de atajos siguen pendientes.

## Flujo de la interfaz anterior

1. En Proyecto indica ruta nueva `.db` o existente y abre/crea. La carpeta debe existir. No usar carpetas de sincronización activa ni aliases/hardlinks para SQLite: esta versión no valida todos los escenarios de sync/FS; WAL y DB deben permanecer juntos.
2. Define idiomas de importación (`en` / `es`, etc.). En Archivo indica TXT, XLF/XLIFF o DOCX soportado y pulsa importar. Selecciona documento y segmento.
3. Edita target; se guarda transaccionalmente a los 250 ms de pausa o máximo 500 ms de escritura continua, más cola/tiempo de commit. El footer distingue pendiente de guardado. Confirmar con Ctrl+Enter; Alt+flechas navega. Copiar fuente/insertar match son acciones explícitas. Cambiar texto invalida confirmación, lock solo cambia bloqueo.
4. TMX se importa para el par indicado; confirmar aprende texto plano si hay una memoria de escritura habilitada. Corregir el destino suspende la contribución anterior hasta reconfirmar; undo/redo restaura también ese efecto. Matches exact/fuzzy y concordancia locales. Fuzzy es aproximado, el porcentaje mide Levenshtein sobre clave NFC; no es context match ni garantía exhaustiva.
5. Busca en fuente/target; resultados por lotes de 128, siguiente lote explícito, operación cancelable. Undo/redo por Ctrl+Z/Y persiste historial; con texto pendiente debes esperar commit y repetir. QA muestra advertencias locales, no corrige traducción.
6. Exporta indicando nombre NUEVO con extensión del formato original. El destino y la fuente no se sobrescriben. Si falla el exporte, no se publica un archivo parcial. Cerrar espera commit; si guardar falla, conserva el borrador y ofrece recuperación TXT a nombre nuevo o descarte explícito.

## DOCX / Microsoft Word

DOCX OOXML Transitional con párrafos en cuerpo y celdas, tablas (incluidas celdas combinadas) y marcadores alrededor de la secuencia de runs. Los runs contiguos con formato inline idéntico forman regiones; las fronteras de estilo se exponen como códigos protegidos `<g id="k">…</g>` y las imágenes estáticas embebidas como `<x id="k"/>` (mismo modelo de tags del editor). La exportación exige todos los códigos presentes, sin duplicar ni anidar, y reconstruye un run por fragmento con su `w:rPr` original; las imágenes y todas las partes ajenas se copian byte a byte. Un DOCX sin texto traducible importa con cero segmentos y exporta idéntico.

Sigue rechazando con mensaje explícito: campos, tracked changes, hyperlinks, content controls, macros/OLE, saltos/tabulaciones (`w:tab`/`w:br`/`w:cr`), cuadros de texto/gráficos/SmartArt/VML dentro de dibujos, runs con texto e imagen juntos, marcado intercalado entre runs de un párrafo, texto en stories secundarias (headers/footers/notas) y relaciones ambiguas o rotas. Los atributos `w:rsid*` de los runs reescritos se pierden sin efecto en Word.

CR/LF/TAB no se convierten silenciosamente a espacios en Word: export rechazado. Namespace alternativo de Word/OPC y Strict todavía no soportados. No existe preview Word dentro de la app; la fidelidad verificada hasta ahora usa Word real vía COM (estructura) y comparación de render PDF, ver `docs/technical/CONTINUACION_DOCX.md`.

## Límites y recuperación

TXT UTF-8, líneas como segmentos y terminadores preservados; XLIFF 1.2 textual sin códigos inline ni seg-source; SDLXLIFF/paquetes no soportados. TMX textual: códigos inline rechazados; metadata TU y variantes originales conservadas, header de export generado por LumenCAT (header original completo aún no almacenado). Idiomas y textos raw no normalizados al guardar, NFC solo auxiliar.

Límite persistido source/target 1 MiB por segmento; documentos generales 256 MiB, DOCX 128 MiB y partes 32 MiB/2048 entradas. Import/export de documentos actualmente usa buffer completo fuera UI, no streaming universal: ver benchmarks y memoria pendiente. Ambas interfaces conservan hasta 1.024 filas del grid más segmento activo; la búsqueda GPUI reúne los resultados completos. El límite de filas no equivale a un límite de RAM en bytes. TM vive en SQLite.

Un cierre abrupto activa recuperación SQLite y aviso. Los commits confirmados son durables dentro de garantías del SO/dispositivo; puede perderse texto aún pendiente o fallar hardware. Si la integridad falla, se rechaza edición: conservar DB/WAL/SHM y restaurar respaldo. No hay reparador de corrupción ni backups automáticos todavía; no copiar solo DB durante sesión activa.

Logs JSON sin texto/prompts/keys en `%LOCALAPPDATA%/LumenCAT/logs/app`; no incluyen documentos. Para desarrollo: `scripts/utils/validate.ps1 -Release`, benchmarks `scripts/utils/benchmark.ps1`. No hacer limpieza global de Cargo: cache del proyecto en `.cache/target`, profiles debug0/incremental=false.

Modo diagnóstico explícito: variable `LUMENCAT_DIAGNOSTICS=1` antes de iniciar, para tiempos/IDs de jobs. Default INFO; revisar logs antes de compartirlos. Ejemplos sintéticos en `output/demo/source.txt`, `source-simple.docx` y `memory.tmx` (en→es); crea proyecto nuevo en esa carpeta y exporta a otro nombre. Demo no es corpus real ni prueba de render Word.
