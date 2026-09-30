# Usar LumenCAT 0.1

Primer slice funcional en desarrollo; no sustituye todavía una herramienta CAT profesional en entregas de cliente. Sin cuenta, red ni IA.

## Ejecutar

Desde `C:/Proyectos/LumenCAT`, `cargo run --locked --release --bin lumencat`. El ejecutable standalone está en `output/lumencat.exe` cuando se publica la build local. Se puede ejecutar desde cualquier directorio. Opcional: `lumencat.exe --project C:/ruta/proyecto.db`. No se requiere Rust para ejecutar ese EXE en Windows compatible, pero no hay instalador firmado ni distribución verificada en otra máquina.

## Flujo

1. En Proyecto indica ruta nueva `.db` o existente y abre/crea. La carpeta debe existir. No usar carpetas de sincronización activa ni aliases/hardlinks para SQLite: esta versión no valida todos los escenarios de sync/FS; WAL y DB deben permanecer juntos.
2. Define idiomas de importación (`en` / `es`, etc.). En Archivo indica TXT, XLF/XLIFF o DOCX soportado y pulsa importar. Selecciona documento y segmento.
3. Edita target; se guarda transaccionalmente a los 250 ms de pausa o máximo 500 ms de escritura continua, más cola/tiempo de commit. El footer distingue pendiente de guardado. Confirmar con Ctrl+Enter; Alt+flechas navega. Copiar fuente/insertar match son acciones explícitas. Cambiar texto invalida confirmación, lock solo cambia bloqueo.
4. TMX se importa para el par indicado; TM no recibe automáticamente tus traducciones. Matches exact/fuzzy y concordancia locales. Fuzzy es aproximado, el porcentaje mide Levenshtein sobre clave NFC; no es context match ni garantía exhaustiva.
5. Busca en fuente/target; resultados por lotes de 128, siguiente lote explícito, operación cancelable. Undo/redo por Ctrl+Z/Y persiste historial; con texto pendiente debes esperar commit y repetir. QA muestra advertencias locales, no corrige traducción.
6. Exporta indicando nombre NUEVO con extensión del formato original. El destino y la fuente no se sobrescriben. Si falla el exporte, no se publica un archivo parcial. Cerrar espera commit; si guardar falla, conserva el borrador y ofrece recuperación TXT a nombre nuevo o descarte explícito.

## DOCX / Microsoft Word

DOCX OOXML Transitional, párrafos en cuerpo y celdas, con varios fragmentos/runs únicamente cuando su formato inline es idéntico; admite estilos de párrafo y tablas simples. Conserva bytes de otras partes (styles, numbering, media, relaciones). Rechaza formato mixto, campos, tracked changes, hyperlinks, content controls, macros/OLE, saltos/tabulaciones, texto en stories secundarias y relaciones ambiguas o rotas. Muchos documentos Word reales quedan fuera de este subset: el editor de tags/runs de M2 es requisito para ampliarlo con fidelidad.

CR/LF/TAB no se convierten silenciosamente a espacios en Word: export rechazado. Namespace alternativo de Word/OPC y Strict todavía no soportados. No existe preview Word ni validación visual real. Round-trip propio y preservación ZIP no prueban por sí solos render equivalente en Microsoft Word.

## Límites y recuperación

TXT UTF-8, líneas como segmentos y terminadores preservados; XLIFF 1.2 textual sin códigos inline ni seg-source; SDLXLIFF/paquetes no soportados. TMX textual: códigos inline rechazados; metadata TU y variantes originales conservadas, header de export generado por LumenCAT (header original completo aún no almacenado). Idiomas y textos raw no normalizados al guardar, NFC solo auxiliar.

Límite persistido source/target 1 MiB por segmento; documentos generales 256 MiB, DOCX 128 MiB y partes 32 MiB/2048 entradas. Import/export de documentos actualmente usa buffer completo fuera UI, no streaming universal: ver benchmarks y memoria pendiente. UI conserva máximo 1024 filas más segmento activo; TM vive en SQLite.

Un cierre abrupto activa recuperación SQLite y aviso. Los commits confirmados son durables dentro de garantías del SO/dispositivo; puede perderse texto aún pendiente o fallar hardware. Si la integridad falla, se rechaza edición: conservar DB/WAL/SHM y restaurar respaldo. No hay reparador de corrupción ni backups automáticos todavía; no copiar solo DB durante sesión activa.

Logs JSON sin texto/prompts/keys en `%LOCALAPPDATA%/LumenCAT/logs/app`; no incluyen documentos. Para desarrollo: `scripts/utils/validate.ps1 -Release`, benchmarks `scripts/utils/benchmark.ps1`. No hacer limpieza global de Cargo: cache del proyecto en `.cache/target`, profiles debug0/incremental=false.

Modo diagnóstico explícito: variable `LUMENCAT_DIAGNOSTICS=1` antes de iniciar, para tiempos/IDs de jobs. Default INFO; revisar logs antes de compartirlos. Ejemplos sintéticos en `output/demo/source.txt`, `source-simple.docx` y `memory.tmx` (en→es); crea proyecto nuevo en esa carpeta y exporta a otro nombre. Demo no es corpus real ni prueba de render Word.
