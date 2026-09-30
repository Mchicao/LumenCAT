# Continuación de compatibilidad Word

## Objetivo pendiente

Probar y corregir el flujo real de LumenCAT: importar Word con tablas e imágenes, traducirlo en la interfaz, exportarlo y abrir el resultado en Microsoft Word conservando formato, tablas e imágenes. El usuario autorizó commits y push directos a `main`, y control de foco durante esta sesión porque trabaja remotamente. Una sesión nueva debe revisar sus instrucciones vigentes.

## Estado publicado y comprobado

- `main`, repositorio `https://github.com/Mchicao/LumenCAT.git`.
- Commit `6db4564`: ejecutables Windows GUI sin consola; barra simplificada, flechas de historial, origen/destino en paralelo y atajos principales de Trados.
- `cargo test --locked`: 27 pruebas aprobadas en esta pasada. `cargo fmt --check` y `git diff --check` también aprobados. Clippy y release habían aprobado en la pasada anterior.
- Microsoft Word instalado: `C:/Program Files/Microsoft Office/root/Office16/WINWORD.EXE`, versión de archivo `16.0.20326.20158`; COM informa `Version=16.0`, `Build=16.0.20326`.
- `scripts/utils/verify_word.ps1` crea una muestra con Word real y compara fuente/exportado usando apertura read-only, tablas/celdas/anchos, tamaños de imágenes, secciones y formas. Exporta PDFs con Word y verifica que abrir/renderizar no cambie los archivos originales.
- Se ejecutó `-CreateFixture`: muestra de dos páginas y 33 párrafos, tablas con celdas combinadas, tres imágenes inline (dos dentro de celdas) y una frase con formato mixto. Se generó el PDF fuente. No se ejecutó aún la comparación con un Word traducido complejo.

## Reproducción actual

`cargo run --locked --example verify_docx -- .cache/verification/docx-handoff` produjo:

```text
PASS mammoth-tables.docx: 6 segmentos, edición SQLite, exportación y reimportación; partes ajenas idénticas
REJECTED mammoth-tiny-picture.docx: Contenido Word no soportado: drawing
REJECTED mammoth-underline.docx: Párrafo con formato mixto entre runs
REJECTED word-inspection.docx: Párrafo con formato mixto entre runs
```

La app todavía no traduce/exporta esos documentos complejos. No confundir rechazo seguro con compatibilidad. El verificador actual termina con código 0 aunque imprima `REJECTED`, traduce solo el primer segmento y crea además un fixture TXT: debe mejorarse para servir como aceptación del flujo completo. Usar un directorio nuevo en cada ejecución porque el exportador no sobrescribe destinos existentes.

## Archivos y evidencia locales

- Importador/exportador: `src/formats/docx.rs`; pruebas: `tests/docx.rs`.
- Modelo y exportación: `src/model.rs`, `src/formats.rs`, `src/storage.rs`, `src/worker.rs`.
- Editor GPUI: `src/gpui_app/mod.rs`, `components.rs`, `input.rs`; lógica de tags `<g>` y `<x/>`: `src/editing.rs`.
- Documentos fuente: `output/verification/word-complex/sources/`.
- Muestra Word: `output/verification/word-complex/sources/word-inspection.docx`.
- Baseline Word: `output/verification/word-complex/renders/baseline/source.pdf` y `word-report.json`.
- Repro independiente: `.cache/verification/docx-handoff/`; log: `logs/tests/docx-handoff.log`.
- Ejecutables de la interfaz publicada: `output/lumencat.exe` y `output/lumencat-gpui.exe`.

`output/`, `.cache/` y `logs/` están ignorados por Git. El código y este handoff se publican; esos artefactos se conservan en el PC y deben regenerarse si se continúa en otra máquina.

## Regenerar la muestra en Windows con Word

```powershell
New-Item -ItemType Directory -Path output/verification/word-complex/sources -Force
Invoke-WebRequest https://raw.githubusercontent.com/mwilliamson/mammoth.js/master/test/test-data/tiny-picture.png -OutFile output/verification/word-complex/sources/tiny-picture.png
& scripts/utils/verify_word.ps1 -CreateFixture -Source output/verification/word-complex/sources/word-inspection.docx -Image output/verification/word-complex/sources/tiny-picture.png -OutputDirectory output/verification/word-complex/renders/baseline
```

Los DOCX públicos `tiny-picture.docx`, `underline.docx` y `tables.docx` están en el mismo directorio de Mammoth en GitHub. Descargar copias a un directorio de pruebas nuevo; no modificar originales.

## Siguiente trabajo

1. Leer `docs/architecture/DECISIONES.md`, `docs/technical/DOCX_WORD_RESEARCH.md` y el flujo completo importación → SQLite → edición → exportación. Construir primero pruebas fallidas con imágenes y formato mixto.
2. `text_slots` rechaza `w:drawing` y compara todos los `w:rPr` byte a byte. No basta con quitar esos rechazos: mover todo el target al primer `w:t` perdería énfasis y posiciones de imágenes inline. Reutilizar códigos protegidos para conservar fronteras de formato e imágenes entre fragmentos de texto. Los runs exclusivamente gráficos no deben alterar el formato del texto vecino.
3. Permitir primero imágenes estáticas embebidas, conservar exactamente XML de dibujo, partes multimedia, dimensiones y relaciones. Distinguir imágenes de gráficos, SmartArt y cuadros de texto; mantener rechazo explícito donde el texto no se traduzca. Validar namespaces y relaciones. No añadir dependencias sin necesidad ni regenerar todo el paquete XML.
4. Validar pérdida/duplicación de códigos antes de exportar; probar texto traducido más largo y Unicode. Preservar propiedades de párrafo, tablas, celdas combinadas, anclajes y estilos. Headers/footers, campos, tracked changes y formatos todavía no cubiertos deben declararse como límites.
5. Traducir todos los segmentos del corpus con traducciones explícitas. Probar también importación, edición y exportación desde el EXE real. Abrir/exportar PDFs con Word usando `verify_word.ps1 -Source <original> -Translated <exportado> -OutputDirectory <nuevo>`; inspeccionar visualmente todas las páginas. La comparación estructural sola no demuestra fidelidad visual. Los PDFs e imágenes son evidencia interna.
6. Ejecutar pruebas relevantes, fmt y Clippy; actualizar límites y evidencia, reconstruir EXE, verificar subsistema GUI y publicar cambios autorizados. Evitar claims de compatibilidad universal o paridad completa con Trados.

No se hicieron cambios al parser DOCX en esta pasada. El soporte complejo sigue pendiente; se deja una reproducción real y un verificador nativo de Word para continuar.
