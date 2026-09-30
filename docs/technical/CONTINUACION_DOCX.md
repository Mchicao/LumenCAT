# Continuación de compatibilidad Word

## Objetivo

Probar y corregir el flujo real de LumenCAT: importar Word con tablas e imágenes, traducirlo completo en la interfaz, exportarlo y abrir el resultado en Microsoft Word conservando formato, tablas e imágenes. El usuario autorizó commits y push directos a `main`.

## Estado logrado en esta pasada

- Formato mixto entre runs e imágenes inline ya **no se rechazan**: los runs contiguos con `w:rPr` idéntico forman regiones; las fronteras de estilo se exponen como códigos `<g id="k">…</g>` y las imágenes estáticas embebidas como `<x id="k"/>` (decisión 011 en `docs/architecture/DECISIONES.md`). El editor GPUI existente (`insert_next_tag`) ya maneja estos códigos.
- Un DOCX sin texto traducible (solo imágenes) importa con cero segmentos y exporta idéntico; antes se rechazaba.
- La exportación valida los códigos (todos presentes, sin duplicar/anidar/desconocidos, texto por región) y un round-trip tolerante al reordenado; rechaza en vez de exportar algo corrupto.

## Evidencia con Word real (16.0.20326, COM)

- `scripts/utils/verify_docx.rs` traduce **todos** los segmentos vía SQLite (`store.edit`), exporta y reimporta; falla con salida ≠ 0 si hay REJECTED. Resultado: PASS `mammoth-tables` (6 segs), `mammoth-tiny-picture` (0 segs), `mammoth-underline` (1 seg), `word-inspection` (20 segs); partes ajenas a `word/document.xml` byte-idénticas.
- `scripts/utils/verify_word.ps1` compara original vs traducido abriéndolos en Word: `word-inspection` mantiene 2 páginas, 33 párrafos, mismas tablas (filas/celdas/anchos incluida la combinada de 435 y la franja de 18), mismas 3 imágenes (90×90 y 65×65 ×2), 0 shapes flotantes; texto en español sin códigos `<g>`/`<x>` filtrados; Word no modificó los archivos (hash). `mammoth-underline` y `mammoth-tiny-picture` también pasan.
- `scripts/utils/render_pdf.ps1` (WinRT, requiere `powershell.exe` 5.1) renderiza los PDF exportados por Word a PNG. Comparación de malla de tinta 24×24 (`logs/tests` y `.cache/verification/docx-e2e`): página 1 difiere 5,3 % promedio por reflow del texto español más largo; las zonas densas de imágenes (97 % tinta) reaparecen desplazadas verticalmente ~2 filas sin perderse; página 2 sin celdas con diferencia >35 %. Estructura y geometría intactas.

## Límites actuales declarados

Rechazados con mensaje explícito: marcado entre los runs de un párrafo (bookmarks intercalados dentro de la secuencia), runs con texto e imagen juntos, `w:tab`/`w:br`/`w:cr`, hyperlinks, campos, revisiones, content controls, `w:pict`/VML, `mc:AlternateContent`, cuadros de texto/gráficos/SmartArt dentro de dibujos, texto en headers/footers/notas/otras historias, numeración con `lvlText` textual, macros/embeddings, Strict OOXML. Los atributos `w:rsid*` de runs reescritos se pierden (sin efecto en Word). El texto alternativo de imágenes (`wp:docPr@name/descr`) no se traduce.

## Reproducir la verificación

```powershell
New-Item -ItemType Directory -Path .cache/verification/docx-e2e-next -Force
Copy-Item output/verification/word-complex/sources/*.docx .cache/verification/docx-e2e-next/  # quitar *-translated previos
cargo run --locked --example verify_docx -- .cache/verification/docx-e2e-next
& scripts/utils/verify_word.ps1 -Source <original> -Translated <exportado> -OutputDirectory <dir-nuevo>
powershell.exe -File scripts/utils/render_pdf.ps1 -Pdf <pdf> -OutputDirectory <png-dir>
```

Usar un directorio nuevo en cada pasada: el exportador nunca sobrescribe destinos existentes.

## Siguiente trabajo

1. Validación en la interfaz GPUI real (EXE) con un DOCX con códigos: insertar códigos con el atajo, confirmar y exportar; los EXE publicados ya incluyen el parser nuevo.
2. Cortes de bajo riesgo siguientes: `w:tab`/`w:br` como códigos, hyperlinks con relación preservada, headers/footers como historias con segmentos propios.
3. Corpus ampliado: listas numeradas, footnotes, campos de página, texto no latino y traducciones mucho más largas, cada uno con fixture Word real y comparación estructural + render.
