# Revisiones adversariales y disposición

OpenCode 1.18.33, `zai-coding-plan/glm-5.3 --variant max --agent plan`, prompts y respuestas en `logs/swarm/`; extractos en `output/glm-*-review.md`. Revisores solo lectura, sin ejecución de tests. Root verificó cambios mediante cargo y workflows, no por la conclusión del revisor.

Primera revisión arquitectónica quedó incompleta al intentar leer registry fuera del ámbito; OpenCode rechazó ese permiso. Se repitió con ámbito solo repo y se obtuvieron dos informes de código/Word y una revisión final. No es un rechazo de cambios del producto ni una aprobación pendiente del usuario.

## Corregido

Guarda stale de concordancia/selección; QA fuera UI y conectado a pantalla; búsqueda cancelable mediante SQLite progress handler; locks preservan origen/estado; texto fallido conserva draft y ofrece recuperación/descarte explícito; idiomas de documento persistidos; reimportación TM idéntica deduplicada preservando variantes con metadata distinta. XLIFF export elimina aprobaciones obsoletas y conserva estados humanos, no normaliza finalidades desconocidas sin aviso.

Word: saltos/tabulaciones no modelados rechazados; comments/PI dentro de texto rechazados; comprobaciones de partes case-insensitive, stories secundarias y numbering traducible rechazados; namespaces OPC/relaciones/IDs/targets validan subset. No promesa de esquema OOXML completo ni fidelidad visual Word.

Última revisión encontró mutador TM sin gate busy y cancelación ausente de consultas TM: correcciones integradas a través del mismo dispatcher/SQLite hook. Se eliminó chequeo de dedup redundante en scoring.

## No aplicadas mecánicamente

Fusionar todas las pasadas XML podría ahorrar líneas, pero no hay medida que justifique introducir riesgo en parser recién validado; revisar con perfil/corpus después. La sugerencia de eliminar chunks del backup de draft suponía que no había UI entre chunks: el trabajo sucede en worker y el token atómico puede cambiar desde UI; conservar checks sí aporta cancelación. Ningún hallazgo sobre latencia se tomó como benchmark; se midió por separado.

## Defectos de rendimiento reproducidos

Recuperación OR inicial perdió candidato específico y concordancia p95 llegó a750ms en10k. Test rojo reprodujo pérdida; routes raras+CROSS JOIN la corrigieron. Frecuencia exacta de términos comunes escaló fuzzy p95 a226ms en3M; contador saturado de512 ocurrencias vía fts5vocab instance limita ese trabajo, sin alterar scoring ni llamar exact a texto normalizado diferente. CSV iniciales y finales permiten comparar. Recall probado es un caso sintético, no corpus exhaustivo.

## DOCX: revisión adversarial GLM-5.3 Max, 30-09-2026

La segunda pasada limitada al workspace encontró una lectura doble de `word/document.xml`, un `replace` CRLF sin efecto porque los saltos se rechazan, un límite de lectura vulnerable a crecimiento concurrente, rechazo de `w:pPr/w:rPr` (formato de la marca de párrafo) y decodificación incompleta de referencias numéricas en `w:t`. Corregidos: una sola validación por parte XML, lectura DOCX con `take(MAX_TOTAL + 1)`, tolerancia de formato de marca del párrafo y decodificación a través del helper XML compartido. Se añadió round-trip con varios `w:t`/runs del mismo formato, `w:pPr/w:rPr`, referencias numéricas, rechazo de formato mixto y caracteres XML 1.0 realmente inválidos. Se mantiene el rechazo explícito de formato mixto hasta tener códigos inline protegidos.

El revisor reportó que U+0085/U+009F eran caracteres XML 1.0 inválidos. Se verificó contra la producción normativa [`Char`](https://www.w3.org/TR/xml/#charsets): ambos están dentro del rango permitido U+0020–U+D7FF (aunque algunas franjas de control se desaconsejan), así que se descartó ese finding; las pruebas cubren U+0001/U+001F, que sí son ilegales. La revisión no abrió archivos con Word y no sustituye una comprobación de interoperabilidad real.
