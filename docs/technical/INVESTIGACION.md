# Conclusiones comparativas — 30 septiembre 2026

Investigación de fuentes documentales por dos subagentes GPT-6 Luna high; no se ejecutaron productos comparables. Las recomendaciones son síntesis, no resultados de pruebas comparativas. No se reutiliza código.

| Herramienta | Qué aprender | Límites / cautelas verificadas |
|---|---|---|
| [Trados Studio](https://www.trados.com/support/getting-started-with-trados-studio/) | Proyecto, editor segmentado, TM/términos visibles, confirmación, QA y retorno a cliente | Producto propietario; [RWS](https://www.trados.com/product/studio/FAQ/subscription-or-perpetual-which-Trados-Studio-license-is-right-for-you/) describe suscripción para nuevas licencias. No hay evidencia primaria suficiente aquí para cuantificar RAM, lentitud o corrupción; esas preocupaciones serán requisitos propios, no acusaciones. |
| [OmegaT](https://omegat.org/) | Local, fuzzy, concordancia, glosarios, varias TMs, propagación y formatos abiertos | GPL-3.0; JVM contraviene nuestra restricción. Aprender conceptos sin portar código. |
| [Supervertaler Workbench](https://supervertaler.com/workbench/) | TM, terminología y consultas IA como recursos del editor | [MIT](https://github.com/Supervertaler/Supervertaler-Workbench); autor declara que ya no se desarrolla activamente y señala coste de round-trip universal. |
| [Swordfish](https://maxprograms.com/products/swordfish.html) | XLIFF como frontera documental, filtros, paquetes y proofreading | Publica fuentes, pero instaladores oficiales requieren licencia tras prueba; no inferir licencia OSI solo de acceso al código. |
| [Open TLC](https://opentlc.org/about) | Local-first, detección de formato y validaciones específicas | Navegador/IndexedDB no encaja aquí. Exportes PDF/PSD/legacy tienen conversiones declaradas; licencia del sitio actual no vinculada concluyentemente al antiguo repo MPL-2.0. |
| [Verbalis](https://github.com/pedrobritx/verbalis) | Offline, TM/glosario, semántica opcional | README indexado describe licencia propia con uso individual y licencia comercial para organizaciones; no tratar como OSI. Acceso web directo al repo no fue concluyente: verificar LICENSE antes de reutilizar cualquier material. |
| [memoQ](https://docs.memoq.com/current/en/webNext-help/memoQeditor-sidepanel.html) | Grid, resultados, comentarios, QA y concordancia en panel contextual | Propietario; [exportar XLIFF genérico](https://docs.memoq.com/current/en/Workspace/mqxliff-export-settings.html) puede perder skeleton/historial/previews de su flavor. |

Expectativas prioritarias: edición por teclado, estados/locks, TM exacta y fuzzy, concordancia, términos, QA, tags íntegros, deshacer, persistencia transparente, exporte reproducible. La ventaja conceptual propuesta es una fila activa con contexto visible, navegación virtualizada y evidencia de durabilidad. No hay llamadas IA al escribir o confirmar.

## Estándares y frontera segura

* [TMX 1.4b](https://www.ttt.org/oscarStandards/tmx/tmx14b.html): intercambio de memorias; Level 1 texto, Level 2 códigos inline. Preservar variantes lingüísticas y metadata, no convertir códigos a texto sin declarar pérdida. Primer corte: pares de texto plano, original de cada TU retenido; rechazar inline codes.
* [XLIFF 1.2](https://docs.oasis-open.org/xliff/v1.2/cs01/xliff-core.html): unidades, source/target, skeleton y extensiones. Primer corte: una unidad textual por segmento; envelope preservado al actualizar target; rechazar segmentación interna y códigos hasta su editor protegido.
* [XLIFF 2.1](https://docs.oasis-open.org/xliff/xliff-core/v2.1/os/xliff-core-v2.1-os.html): modelo distinto, códigos con restricciones operativas; no procesarlo como 1.2. Estudiar 2.0/2.1/2.2 por separado al implementar.
* [TBX / ISO 30042:2019](https://www.iso.org/standard/62510.html): terminología multilingüe, estilos y dialectos; no confundir con TM. Elegir y validar dialecto explícito en M2; norma ISO completa no consultada.
* [SDLXLIFF](https://developers.rws.com/studio-api-docs/17.2/apiconcepts/filetypesupport/file_type_support_overview.html): XLIFF 1.2 con extensiones RWS. No prometer compatibilidad al aceptar XML genérico.
* [SDLPPX](https://developers.rws.com/studio-api-docs/apiconcepts/projectautomation/creating_a_project_package.html)/[SDLRPX](https://developers.rws.com/studio-api-docs/15.2/apiconcepts/projectautomation/about_packages.html): paquetes propietarios con recursos/manifiestos. Ser ZIP no basta para reconstruir un retorno correcto. M4 exige fixtures autorizados, límites de extracción y revisión de licencia; no ingeniería de evasión.
* [MQXLIFF/MQXLZ](https://docs.memoq.com/current/en/Workspace/mqxliff-export-settings.html), [MXLIFF](https://support.phrase.com/hc/en-us/articles/5709739992860--MXLIFF-Files-TMS), TXML: sabores de proveedor, no intercambiables por extensión. Phrase usa placeholders y metadata propios.

TXT será UTF-8 con límites de línea y terminadores preservados, sin inventar segmentación lingüística. SRX/UAX29 será una ampliación explícita, manteniendo IDs estables.

## Recuperación TM

[SQLite FTS5](https://www.sqlite.org/fts5.html) sirve para recuperar candidatos, no calcular fuzzy. `unicode61` no segmenta CJK lingüísticamente; trigram omite consultas menores de tres caracteres. Diseñar exacto con idioma y texto original, NFC auxiliar, candidatos por tokens/n-grams y ranking Levenshtein Unicode limitado. Retrieval aproximado debe declarar límites y medir recall; jamás llamar 100% a textos distintos normalizados ni afirmar context match sin evidencia contextual.

## Problemas por investigar con usuarios

Coste de preparación de proyectos, exceso de controles, formatos que pierden metadata, consultas TM irrelevantes y confianza en autosave son hipótesis de diseño. Falta entrevistar traductores y ejecutar comparativas reales; no convertir opiniones generales sobre Trados en hechos comprobados.

Ampliación de fuentes: [XLIFF 2.0 normativo](https://docs.oasis-open.org/xliff/xliff-core/v2.0/os/xliff-core-v2.0-os.html), [estado XLIFF 2.2 Committee Specification](https://www.oasis-open.org/standard/xliff-v2-2-cs01/) y [recursos TBX-Basic](https://www.tbxinfo.net/tbx-downloads/). No equiparar automáticamente el estado normativo de 2.2 con 2.1 ni aceptar versiones 2.x como 1.2.

Caso documentado por [soporte RWS](https://sdl.my.site.com/articles/en_US/SolutionArticles/000022545?articleName=000020533): un archivo bilingüe puede fallar al exportar si no se embebe el DependencyFile/skeleton necesario. Eso justifica conservar recursos y probar export al principio del flujo; no establece frecuencia de fallos ni cifras de rendimiento Trados.
