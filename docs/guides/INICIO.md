# Usar LumenCAT 0.1

Primer slice funcional en desarrollo; no sustituye todavía una herramienta CAT profesional en entregas de cliente. Sin cuenta, red ni IA.

## Ejecutar

Desde `C:/Proyectos/LumenCAT`, `cargo run --locked --release --bin lumencat`. El ejecutable standalone está en `output/lumencat.exe` cuando se publica la build local. Se puede ejecutar desde cualquier directorio. Opcional: `lumencat.exe --project C:/ruta/proyecto.db`. No se requiere Rust para ejecutar ese EXE en Windows compatible, pero no hay instalador firmado ni distribución verificada en otra máquina.

## Interfaz GPUI

`cargo run --locked` abre la única interfaz GPUI. Acepta `--project C:/ruta/proyecto.lcat`; no hay selector de frontend ni un segundo ejecutable de escritorio.

- **Archivo** abre una vista completa con navegación lateral azul: **Abrir**, **Documentos del proyecto** y **Nuevo** muestran sus acciones a la derecha. **Guardar** conserva el proyecto; **Guardar destino como** exporta el documento. Nuevo usa el diálogo Guardar; Abrir selecciona un proyecto existente. Ctrl+N/Ctrl+O abren esos diálogos. **Volver al editor** o Escape recupera Inicio. Los proyectos nuevos empiezan en→es; puedes cambiar y guardar los idiomas de próximas importaciones sin relabelar documentos existentes.
- **Inicio** contiene las acciones del segmento. **Revisión** permite verificar el segmento activo, consultar QA/términos, confirmar sin avanzar y cambiar el bloqueo; no hay QA global de archivo/proyecto. **Avanzado** contiene memorias del proyecto, importación/exportación TMX, concordancia y reemplazo. **Ver** muestra/oculta los paneles o restaura su disposición. **Complementos** informa del sistema de plugins todavía no implementado.
- **Configuración** permite elegir modo claro u oscuro y los temas Azul, Verde petróleo o Violeta. La elección se aplica al instante y se guarda atómicamente en `%LOCALAPPDATA%/LumenCAT/settings/appearance.json`, fuera del proyecto. Escape vuelve al editor. Si no se puede guardar, se informa el error y se conserva el tema anterior.
- Botones, iconos y selectores de apariencia usan GPUI Kit; se mantienen los comandos, proyectos y preferencias existentes. Los selectores son grupos de opciones, no menús desplegables.
- Origen y destino aparecen en paralelo, con el estado entre las columnas y memoria/QA a la derecha. Las flechas compactas de deshacer/rehacer permanecen en la barra superior; sus nombres y atajos aparecen al pasar el ratón.
- Clic en una fila o Ctrl+↑/↓ para navegar; el destino recibe foco. Ctrl+Enter confirma y avanza; Ctrl+Alt+Enter confirma sin avanzar. Ctrl+Insert o Alt+Insert copia el origen; Ctrl+L bloquea/desbloquea. Se conservan ↑/↓, Alt+↑/↓ y Alt+C como alternativas.
- Ctrl+T aplica la primera coincidencia TM; Ctrl+1…9 aplica la coincidencia correspondiente. F3 busca concordancias del destino seleccionado completo o, en su defecto, del origen; con el foco en búsqueda usa esa consulta. Ctrl+S guarda; Shift+F12 abre la exportación cuando no hay cambios pendientes.
- Ctrl+, o Ctrl+Alt+flecha inserta la siguiente etiqueta `<g>`, `</g>` o `<x/>` que falta. Los badges son representación del texto; no amplían el subset XLIFF/DOCX del importador.
- Ctrl+F busca en origen/destino, con alcance de segmento o documento. Ctrl+H abre reemplazo literal, sensible a mayúsculas, solo en destinos. El reemplazo de documento es transaccional, ignora segmentos bloqueados y conserva tags. Ctrl+Z/Y deshace/rehace cada segmento del historial.
- El progreso cuenta destinos no vacíos en todo el documento; no equivale a confirmación ni QA aprobada. Clic en la barra va al primer pendiente cargado. El editor muestra palabras y caracteres Unicode; el límite UTF-8 sigue siendo 1 MiB. El porcentaje TM aplicado se muestra durante la sesión; no se persiste como metadata.
- Autoguardado periódico sin otra pulsación. **Salir** y la X piden confirmación: **Guardar y salir**, **Salir sin guardar** o **Cancelar**. El autoguardado se pausa mientras decides. Guardar y salir espera el commit y el cierre seguro; ante error conserva ventana y texto. Salir sin guardar descarta solo lo pendiente, no revierte avances guardados ni operaciones en curso. No hay diálogo para recuperar el borrador en TXT.

GPUI virtualiza las filas del grid (solo se construyen las filas visibles, con precarga y caché acotada) y el editor Trados mantiene el texto completo del segmento activo. El puente de entrada nativa y AccessKit/UIA están conectados; edición y acciones por cua-driver se verificaron en Windows, incluida la escritura por UIA en proyectos de 2.000 segmentos. La selección parcial por ratón, la posición del cursor por clic y la validación completa de IME, RTL y lectores de pantalla siguen pendientes. Ver [Computer Use](COMPUTER_USE.md). La tabla bilingüe representa segmentos, no el diseño visual de Word.

Los atajos principales se basan en el [perfil predeterminado documentado por RWS](https://www.trados.com/media/images/translate-file-in-10-easy-steps-2022-rws-en-1_tcm234-213936.pdf). No hay paridad completa con Trados: confirmar y avanzar va al segmento siguiente, no al siguiente pendiente; QuickPlace inserta el siguiente tag, sin desplegar candidatos. Confirmar aprende texto plano en la memoria de escritura seleccionada; guardar borradores no aprende. Ver [memorias](MEMORIAS.md). La pestaña «Términos» permite crear bases y conceptos manuales; consulta [terminología y sus límites](TERMINOLOGIA.md). Importación de glosarios, corrección ortográfica y configuración de atajos siguen pendientes.

## DOCX / Microsoft Word

DOCX OOXML Transitional con párrafos en cuerpo y celdas, tablas (incluidas celdas combinadas) y marcadores alrededor de la secuencia de runs. Los runs contiguos con formato inline idéntico forman regiones; las fronteras de estilo se exponen como códigos protegidos `<g id="k">…</g>` y las imágenes estáticas embebidas como `<x id="k"/>` (mismo modelo de tags del editor). La exportación exige todos los códigos presentes, sin duplicar ni anidar, y reconstruye un run por fragmento con su `w:rPr` original; las imágenes y todas las partes ajenas se copian byte a byte. Un DOCX sin texto traducible importa con cero segmentos y exporta con todas las partes internas idénticas; el contenedor ZIP puede cambiar de hash al recomprimirse.

Sigue rechazando con mensaje explícito: campos, tracked changes, hyperlinks, content controls, macros/OLE, saltos/tabulaciones (`w:tab`/`w:br`/`w:cr`), cuadros de texto/gráficos/SmartArt/VML dentro de dibujos, runs con texto e imagen juntos, marcado intercalado entre runs de un párrafo, texto en stories secundarias (headers/footers/notas) y relaciones ambiguas o rotas. Los atributos `w:rsid*` de los runs reescritos se pierden sin efecto en Word.

CR/LF/TAB no se convierten silenciosamente a espacios en Word: export rechazado. Namespace alternativo de Word/OPC y Strict todavía no soportados. No existe preview Word dentro de la app; la fidelidad verificada hasta ahora usa Word real vía COM (estructura) y comparación de render PDF, ver `docs/technical/CONTINUACION_DOCX.md`.

## Límites y recuperación

TXT UTF-8, líneas como segmentos y terminadores preservados; XLIFF 1.2 textual sin códigos inline ni seg-source; SDLXLIFF/paquetes no soportados. TMX textual: códigos inline rechazados; metadata TU y variantes originales conservadas, header de export generado por LumenCAT (header original completo aún no almacenado). Idiomas y textos raw no normalizados al guardar, NFC solo auxiliar.

Límite persistido source/target 1 MiB por segmento; documentos generales 256 MiB, DOCX 128 MiB y partes 32 MiB/2048 entradas. Import/export de documentos actualmente usa buffer completo fuera UI, no streaming universal: ver benchmarks y memoria pendiente. GPUI conserva hasta 1.024 filas del grid más segmento activo; la búsqueda reúne los resultados completos. El límite de filas no equivale a un límite de RAM en bytes. TM vive en SQLite.

Un cierre abrupto activa recuperación SQLite y aviso. Los commits confirmados son durables dentro de garantías del SO/dispositivo; puede perderse texto aún pendiente o fallar hardware. Si la integridad falla, se rechaza edición: conservar DB/WAL/SHM y restaurar respaldo. No hay reparador de corrupción ni backups automáticos todavía; no copiar solo DB durante sesión activa.

Logs JSON sin texto/prompts/keys en `%LOCALAPPDATA%/LumenCAT/logs/app`; no incluyen documentos. Para desarrollo: `scripts/utils/validate.ps1 -Release`, benchmarks `scripts/utils/benchmark.ps1`. No hacer limpieza global de Cargo: cache del proyecto en `.cache/target`, profiles debug0/incremental=false.

Modo diagnóstico explícito: variable `LUMENCAT_DIAGNOSTICS=1` antes de iniciar, para tiempos/IDs de jobs. Default INFO; revisar logs antes de compartirlos. Ejemplos sintéticos en `output/demo/source.txt`, `source-simple.docx` y `memory.tmx` (en→es); crea proyecto nuevo en esa carpeta y exporta a otro nombre. Demo no es corpus real ni prueba de render Word.
