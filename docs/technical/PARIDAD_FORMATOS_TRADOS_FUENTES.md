# Paridad de formatos con Trados: fuentes y contrato de aceptación

**Corte documental: 5 de octubre de 2026. Estado: investigación, no compatibilidad ejecutada.**

LumenCAT puede ampliar su interoperabilidad sin incorporar el runtime de Trados, pero la paridad debe medirse por **formato, variante, contenido extraído y documento entregado**, no por número de extensiones. La recomendación es completar primero los códigos protegidos y la conservación documental; después añadir filtros Rust para formatos abiertos y evaluar Okapi como motor externo opcional. No hay evidencia aquí de round-trip universal ni de apertura correcta en Word, Trados o aplicaciones de maquetación.

## Conclusiones que condicionan la implementación

1. **Studio instalado es `19.0.0.3043`, producto `Studio19`.** Se encontraron 42 manifiestos de FileTypeSupport en su instalación. Un manifiesto puede registrar varios filtros, plantillas o procesadores de contenido; no significa «42 formatos» ni licencia activa. Véase [evidencia local](#evidencia-local-reproducible).
2. **La referencia actual es la tabla oficial de Studio 2026**, recuperada desde el enlace real del centro documental [T26]. El marketing todavía enlaza la tabla de Studio 2024 y anuncia «over 50 different file types» [MARKETING]. No deben sumarse versiones, perfiles XML y extensiones como si fueran familias independientes.
3. **PDF no implica entrega PDF.** El filtro documentado convierte PDF a Word y guarda el destino como Word; el OCR de imágenes escaneadas está limitado. PDF Assistant es gratuito, pero utiliza Word en el flujo descrito [PDF] [APP]. Extraer texto con una biblioteca PDF no reproduce ese servicio.
4. **SDLXLIFF editable no implica reconstrucción del documento original.** La documentación distingue edición bilingüe de generación nativa, que puede requerir el filtro original. Un SDLXLIFF de un filtro retirado puede seguir editándose sin poder generar el destino [SDK-SETTINGS] [EOL].
5. **OpenXLIFF no es hoy una alternativa íntegramente abierta por defecto.** Su licencia principal es EPL-1.0, pero declara BCP47J y XMLJava, cuyas licencias consultadas prohíben producción/comercialización y redistribución sin permiso escrito. No recomendar su distribución completa hasta resolver esas dependencias [OPENXLIFF] [OPENXLIFF-LICENSE] [OPENXLIFF-DEPS] [BCP47J-LICENSE] [XMLJAVA-LICENSE].

## Cómo leer la evidencia

| Marca | Qué acredita | Qué no acredita |
| --- | --- | --- |
| **D26** | Fila o configuración publicada por RWS para Studio 2026. | Uso real, licencia activa, fidelidad de todas las variantes. |
| **L** | Manifiesto, builder o recurso presente en `Studio19\PlugIns`. | Que esté habilitado en un proyecto, ni que el usuario tenga derecho a usarlo. |
| **A** | Oferta y metadatos declarados por AppStore el día del corte. | Descarga efectiva, instalación, precio futuro, licencia libre o funcionamiento. |
| **S** | Especificación del estándar o documentación del propietario. | Cobertura del filtro de Trados o de una biblioteca. |
| **C** | Criterio propuesto para comprobar LumenCAT. | Prueba ejecutada o comportamiento observado de Trados. |

**Base** significa documentado como filtro entregado con Studio, dentro de un producto sujeto a su licencia; no «software gratuito». **Plugin gratuito/de pago** se basa en `paidFor` del AppStore, no en una auditoría de sus términos. **No localizado** significa ausente en el alcance inspeccionado, no inexistente.

Se consultaron documentación pública, metadatos del EXE, manifiestos XML y recursos de la instalación en lectura. No se cargaron DLL de Trados, no se abrieron proyectos personales, no se consultaron credenciales/licencias del usuario y no se recorrieron sus carpetas de plugins. No se lanzaron aplicaciones, GUI, conversiones, instalaciones ni pruebas funcionales.

### Punto de partida de LumenCAT

Lectura final del corte de [`src/formats.rs`](../../src/formats.rs), [`src/formats/inline.rs`](../../src/formats/inline.rs), [`src/formats/docx.rs`](../../src/formats/docx.rs) y [`Cargo.toml`](../../Cargo.toml), sin ejecución. El alcance refleja también los cambios concurrentes observados al cerrar el informe; no acredita el ejecutable entregado ni sus pruebas:

| Ruta actual | Alcance que declara el código leído | Brecha frente al objetivo |
| --- | --- | --- |
| TXT | UTF-8, BOM opcional, una unidad por línea; conserva finales de línea y no permite introducir líneas estructurales en el target. | Otras codificaciones y extracción mediante reglas no acreditadas. |
| XLIFF | `.xlf`/`.xliff`, versión 1.2; códigos `g`, `mrk`, `x`, `ph`, `bpt`, `ept`, `it`, `bx` y `ex` representados como códigos protegidos y recuperados en la exportación. | `seg-source`, `sub`, extensiones inline, pares superpuestos, 2.x y dialectos CAT no admitidos. No basta renombrar `.sdlxliff`; GUI y apertura externa no comprobadas aquí. |
| DOCX | Subset Transitional con rutas y prefijo `w:` convencionales; párrafos/tablas, regiones de formato representadas con `<g>` e imágenes admitidas con `<x>`. Conserva el paquete como esqueleto. | Strict, historias externas con texto, enlaces, campos, revisiones, controles, cuadros de texto y otros constructos rechazados. Apertura/render en Word no comprobados aquí. |
| TMX | Ruta separada de memoria, versión 1.4/1.4b; reconoce `hi`, `ph`, `bpt`, `ept`, `it` y `ut`, conservando XML de las unidades importadas para exportación. | Importar memoria no equivale a abrir TMX como documento bilingüe traducible ni acredita cobertura de toda extensión inline. |

Ya existen `quick-xml = 0.38.3`, `zip = 4.6.1` y `serde_json = 1.0.151`; las ampliaciones XML/ZIP/JSON deben empezar por evaluar esas dependencias. [`DOCX_WORD_RESEARCH.md`](DOCX_WORD_RESEARCH.md) sigue siendo útil para OPC, historias y fidelidad, pero su descripción de rechazo del formato mixto/dibujos es anterior al código leído en este corte. No se actualizó ese archivo: el alcance autorizado es este informe.

## Matriz del catálogo base

Las versiones/años son **los nombres publicados por RWS**, no una promesa adicional sobre Office/Adobe posteriores ni la versión de una biblioteca. Las extensiones provienen de [T26]; se recuperaron también los fragmentos HTML diferidos para evitar celdas aparentemente vacías. Todos los criterios C-* remiten al [contrato de aceptación](#contrato-de-aceptación-observable).

En la columna local se abrevia el prefijo `Sdl.FileTypeSupport.` y el sufijo `.plugin.xml`. Por ejemplo, `Filters.Doc` identifica el manifiesto completo `Sdl.FileTypeSupport.Filters.Doc.plugin.xml`.

### Office y documentos de oficina

| Formato y versión anunciada | Extensiones anunciadas | Disponibilidad y corroboración | Salida/alcance que debe comprobarse |
| --- | --- | --- | --- |
| Microsoft Word 2007–2019 | `.docx`, `.dotx`, `.docm`, `.dotm` | Base D26; L `Filters.MicrosoftOffice_2`, builder DOCX. | Misma clase de documento/plantilla, estilos, tablas e historias; conservar macros sin ejecutarlas cuando se admitan. C-OFFICE. |
| Microsoft Word 97–2003 | `.doc`, `.dot`, `.wps` | Base D26; L `Filters.Doc`. | DOC binario no es OOXML; WPS requiere identificación propia. Una conversión a DOCX debe anunciarse. C-LEGACY. |
| Microsoft PowerPoint 2007–2019 | `.pptx`, `.ppsx`, `.potx`, `.pptm`, `.potm`, `.ppsm` | Base D26; L `Filters.MicrosoftOffice_2`, builder PPTX. | Texto de diapositivas/notas según ajustes; conservar objetos, masters, vínculos, animaciones y tipo de paquete. C-PPT. |
| Microsoft PowerPoint 97–2003 | `.ppt`, `.pps`, `.pot` | Base D26; L `Filters.Ppt`. | Presentación binaria y sus recursos; no inferir soporte desde el lector PPTX. C-LEGACY. |
| Microsoft Excel 2007–2019 | `.xlsx`, `.xltx`, `.xlsm` | Base D26; L `Filters.MicrosoftOffice_2`, builder XLSX y subcontenido. | Celdas de texto seleccionadas, fórmulas/números/formatos intactos, rich text y contenido elegido. `.xlsb`/`.xltm` no acreditados por esta fila. C-XLSX. |
| Microsoft Excel 97–2003 | `.xls`, `.xlt` | Base D26; L `Filters.Xls`. | BIFF/CFB separado de XLSX; preservar fórmulas, hojas, nombres y referencias. C-LEGACY. |
| Bilingual Excel | `.xlsx` | Base D26; L `Filters.BilingualExcel`. No confundir con Multilingual Excel del AppStore. | Columnas origen/destino configurables, contexto/comentarios y traducciones existentes; no sobrescribir origen. C-BILINGUAL-TABLE. |
| Microsoft Visio | `.vsdx`, `.vsx`, `.vtx`, `.vdx`, `.vssx`, `.vstx`, `.vsdm`, `.vssm`, `.vstm` | Base D26; L `Filters.Visio`. | Dibujos, stencils y plantillas tienen contratos distintos. Conservar geometría, IDs y conexiones. `.vsd` no figura en esta tabla. C-VISIO. |
| Rich Text Format | `.rtf` | Base D26; L `Native.RTF_2`. | RTF monolingüe conservando controles/grupos, fuentes, imágenes y escapes; no mezclar con Workbench bilingüe. C-RTF. |
| OpenDocument Text | `.odt`, `.ott`, `.odm` | Base D26; L `Filters.Odf`, builder ODT. | Texto/plantillas/master; conservar estilos, manifiesto y recursos. RWS no fija versión ODF en esta fila. C-ODF. |
| OpenDocument Presentation | `.odp`, `.otp` | Base D26; L `Filters.Odf`, builder ODP. | Diapositivas/notas y estilos según perfil; misma clase de paquete. C-ODF. |
| OpenDocument Spreadsheet | `.ods`, `.ots` | Base D26; L `Filters.Odf`, builder ODS. | Celdas elegidas; fórmulas, tipos y repeticiones sin cambios accidentales. C-ODF. |

La documentación de ajustes da ejemplos de inclusión/exclusión de speaker notes, estilos no traducibles, propiedades y texto condicional [SDK-SETTINGS]. Por tanto, comparar extracción exige guardar el **perfil de ajustes**, no solo comparar extensiones. La documentación de incorporación de Excel bilingüe describe columnas y varios idiomas, pero no convierte el filtro base en el plugin multilingüe [BILINGUAL-EXCEL].

### Adobe, maquetación y PDF

| Formato y versión anunciada | Extensiones | Disponibilidad y corroboración | Salida/alcance que debe comprobarse |
| --- | --- | --- | --- |
| Adobe FrameMaker 8–2022 MIF V2 | `.mif` | Base D26; L `Filters.Bilingual.FrameMaker` y `Native.FrameMaker_2` como familia, sin equiparar sus versiones internas a los años anunciados. | MIF, no `.fm`; estilos, variables, referencias y texto condicional. C-DTP. |
| Adobe FrameMaker 8–2020 MIF | `.mif` | También figura en D26. Recurso local bilingüe: `Adobe FrameMaker 8-2020 MIF version 3`, ID `FrameMaker v 10.0.0`. | Contrastar filtro exacto; las dos filas no justifican duplicar la familia MIF. C-DTP. |
| Adobe InDesign CS2–CS4 INX | `.inx` | Base D26; L `Filters.Inx`. | Intercambio INX, no `.indd`; historias, etiquetas, condiciones y estilos. C-DTP. |
| Adobe InDesign CS4–CC IDML | `.idml` | Base D26; L `Filters.Idml`. | Paquete de intercambio, no documento INDD nativo; conservar estructura, historias, enlaces y estilos; detectar desbordamiento tras traducir. C-DTP. |
| Adobe InCopy CS4–CC ICML | `.icml` | Base D26; L `Filters.Icml`. | ICML, no asumir `.icma`/`.icml` intercambiables ni soporte de todo documento InCopy. C-DTP. |
| Adobe Photoshop | `.psd`, `.pdd`, `.psdt` | Base D26; L `Filters.Photoshop`. | Texto editable/capas según cobertura; conservar píxeles, máscaras, recursos y métricas. La especificación PSD/PSB existe [PSD], pero `.psb` no está anunciado en esta fila. C-PSD. |
| QuarkXPress Export | `.xtg`, `.tag` | Base D26; L `Filters.QuarkXPress`. | Texto etiquetado exportado, no `.qxp`; entrega reimportable en QuarkXPress. C-DTP. |
| PDF | `.pdf` | Base D26; L `Native.PDF`. Conversión de terceros; OCR limitado [PDF]. | **Destino Word**, no round-trip PDF nativo. Separar PDF con texto, OCR previo e imagen sin texto. C-PDF. |

Los intercambios Adobe/Quark siguen siendo vocabularios controlados por su propietario aunque estén representados con XML o texto. Su lectura es viable sin Trados; la cobertura y fidelidad visual siguen requiriendo corpus y aplicaciones destino. No se recuperó una especificación IDML actual completa del propietario en esta investigación: no atribuir conformidad universal a una descripción histórica ni a un parser XML genérico.

### Web, texto y localización de software

| Formato/perfil anunciado | Extensiones | Disponibilidad y corroboración | Alcance que debe comprobarse |
| --- | --- | --- | --- |
| HTML 5 | `.htm`, `.html`, `.xhtml`, `.jsp`, `.asp`, `.aspx`, `.ascx`, `.inc`, `.php`, `.hhk`, `.hhc` | Base D26; L `Filters.Html` con builders HTML4/HTML5, subcontenido y plantilla. | Texto/atributos elegidos; scripts, CSS, URLs y código servidor protegidos. Estas extensiones no acreditan traducción del lenguaje de programación completo. C-HTML. |
| XHTML 1.1 (2) | `.html`, `.htm`, `.xhtm` | Base D26; L `Filters.Xml_2.Xhtml`. | XML bien formado con reglas XHTML; distinto de HTML tolerante. C-XML. |
| XML 2: Any XML | `.xml` | Base D26; L `Filters.Xml_2` y `Filters.Xml_2.Template`. | Reglas por elementos/atributos/XPath, namespaces, contenido mixto y subcontenido. C-XML. |
| XML 2: OASIS DITA 1.3 | `.xml`, `.dita` | Base D26; L `Filters.Xml_2.Dita`. | Perfil DITA 1.3 anunciado. `.ditamap` no aparece en esta fila; no inferir recorrido completo de mapas/conrefs. C-XML-PROFILE. |
| XML 2: OASIS DocBook 4.5 | `.xml` | Base D26; L `Filters.Xml_2.DocBook`. | Perfil DocBook 4.5 anunciado; DocBook 5 requiere otro contrato/corpus. C-XML-PROFILE. |
| XML 2: Author-it | `.xml` | Base D26; L `Filters.Xml_2.AuthorIT`. | Vocabulario del proveedor, IDs, referencias y exclusiones; versión no especificada. C-XML-PROFILE. |
| XML 2: MadCap | `.html`, `.htm` | Base D26; L `Filters.Xml_2.MadCap`. | Condiciones, variables, referencias y marcado propio; no asumir paquete Flare completo. C-XML-PROFILE. |
| XML 2: AEM Sites XML | `.xml` | Base D26; L `Filters.Xml_2.AemSites`. | Export/import de contenido AEM y referencias; no es conexión al CMS. C-XML-PROFILE. |
| XML 2: W3C ITS | `.xml`, `.its` | Base D26; L `Filters.Xml_2.Its`. | Reglas de traducibilidad/herencia y selección; versión ITS del filtro no precisada en la tabla. ITS 2.0 es referencia independiente [ITS]. C-XML-PROFILE. |
| XML 2: Microsoft .NET Resources | `.resx` | Base D26; L `Filters.Xml_2.Resx`. | Recursos string; nombres, tipos, binarios y placeholders protegidos. C-RESOURCE. |
| JSON | `.json` | Base D26; L `Filters.Json`. | Strings seleccionadas por ruta/reglas y subcontenido; no claves, números ni booleanos por defecto. Versión/perfil del filtro no fijado en la tabla. C-JSON. |
| YAML | `.yaml`, `.yml` | Base D26; L `Filters.Yaml`, ID `YAML v 1.0.0.0` **del filtro**, no YAML 1.0. | Tipos, comillas, comentarios, anchors/aliases y escalares multilineales; separar mono/bilingüe. C-YAML. |
| Markdown | `.md`, `.markdown` | Base D26; L `Filters.Markdown`. | Prosa, títulos, enlaces y alt text; código, URLs y sintaxis protegidos. No se anunció dialecto preciso. C-MARKDOWN. |
| Portable Object | `.po` | Base D26; L `Filters.PO`. | `msgid`/`msgstr`, plurales, contexto, flags y comentarios; PO no equivale al binario `.mo`. C-PO. |
| Java Resources | `.properties` | Base D26; L `Filters.JavaResource`. | Claves, escapes Unicode, continuaciones, comentarios y placeholders. Codificación/perfil explícitos. C-RESOURCE. |
| .NET Libraries | `.dll`, `.exe` | Base D26; L `Filters.DotNetLibraries`. | Solo recursos localizables; conservar metadatos/código y declarar política de firmas. **Nunca ejecutar el archivo para extraer texto.** C-DOTNET. |
| Comma Delimited Text | `.csv` | Base D26; L `Filters.GenericDelimited`, builder CSV. | Columnas, delimitador, comillas, filas y escapes configurados; CSV multicolumna no es TXT por líneas. C-DELIMITED. |
| Tab Delimited Text | `.txt` | Base D26; L `Filters.GenericDelimited`, builder TAB. | Tabla tabulada; `.tsv` no aparece en esta fila aunque otros motores la admitan. C-DELIMITED. |
| Text / RegEx | `.txt` | Base D26; L `Native.RegEx_1_1` con filtro, subcontenido y plantilla. | Texto extraído/protegido por reglas y codificación; no confundir con todos los TXT del sistema. C-TEXT. |
| Email | `.msg`, `.eml`, `.emlx`, `.oft` | Base D26; L `Filters.Email`. | MIME y Outlook MSG/OFT requieren rutas distintas; asunto/cuerpo/HTML según ajustes, adjuntos sin pérdida. C-EMAIL. |
| Subtitle formats | `.srt`, `.vtt`, `.sbv`, `.sub`, `.ttml`, `.xml`, `.dfxp` | Base D26; L `Filters.Subtitles`, ID `Subtitles v 1.0.0.0`. | Tiempos, IDs, estilos y metadatos protegidos; dialecto identificado, no toda variante llamada `.sub`. C-SUBTITLE. |

[SUBTITLE] describe expresamente el reemplazo de SubRip y menciona SRT/VTT/SBV/SUB; la tabla [T26] incluye además TTML/XML/DFXP. Esto justifica el inventario documental, no una ejecución de cada variante. Las notas de Studio 2024 retiraron **los filtros antiguos** `FrameMaker 8.0 v 2.0.0.0`, `XML1` y `SubRip`, no los formatos MIF/XML/SRT completos [EOL].

### Bilingües CAT

| Formato/perfil | Extensiones | Disponibilidad y corroboración | Límite de interoperabilidad |
| --- | --- | --- | --- |
| SDL XLIFF | `.sdlxliff` | Base D26; L `Bilingual.SdlXliff`. XLIFF 1.2 con extensiones SDL según SDK [SDK]. | Conservar IDs, `seg-source`, códigos, estados, comentarios, contexto y skeleton; edición bilingüe ≠ generación nativa. C-CAT. |
| XLIFF | `.xlf`, `.xliff` | Base D26; L `Filters.Xliff`. Referencia de estándar 1.2 [XLIFF12]; no equiparar su versión de manifiesto `9.1.0.0` a XLIFF 9.1. | Conservar estructura y contenido desconocido; comprobar inline, segmentos y estados. C-XLIFF. |
| XLIFF 2.0 | `.xlf`, `.xliff` | Base D26; L `Filters.Xliff2`. Estándar separado [XLIFF20]. | No basta ampliar el número de versión del parser 1.2. 2.1/2.2 no acreditados por esta fila. C-XLIFF. |
| Kilgray memoQ XLIFF | `.mqxlf`, `.mqxliff`, `.mqxlz` | Base D26; L `Filters.Xliff.MemoQ`. | Dialecto propio y, para `.mqxlz`, contenedor; preservar extensiones/estados y reabrir en la herramienta que lo generó. C-CAT. |
| Memsource XLIFF | `.mxliff` | Base D26; L `Filters.Xliff.MXliff`; también existe plugin AppStore. | No confundir presencia base con necesidad obligatoria del plugin. Preservar metadatos específicos y confirmar versión del generador. C-CAT. |

TMX, SDLTM, TMW/MDB, MultiTerm/SDLTerm y TBX son recursos de memoria/terminología; no deben inflar el catálogo de documentos nativos. TMX como documento requiere un filtro distinto en Studio, recogido a continuación. Los paquetes `.sdlppx`/`.sdlrpx` tampoco son formatos nativos de contenido: necesitan validar manifiesto, idiomas, rutas, archivos bilingües y recursos por separado.

## Plugins gratuitos, de pago y compatibilidad declarada

Fuente primaria: API pública [APP], GET con cabecera `apiversion: 2.0.0`. Se inspeccionaron `id`, `name`, `paidFor`, descripción y versiones con `minimumRequiredVersionOfStudio`, `maximumRequiredVersionOfStudio`, `versionNumber`, `isPrivatePlugin`. Los rangos son **la declaración de la API para la familia de Studio**, no una validación contra el EXE de cuatro componentes.

Todos los registros de versión 19 listados debajo devolvieron `isPrivatePlugin=true`. No se deduce el significado contractual de ese campo ni disponibilidad de descarga anónima. «Sin rango 19» significa que la respuesta consultada no lo declaró; no equivale a incompatibilidad probada. Los plugins no están acreditados como instalados: solo se inspeccionó la carpeta del producto.

| Plugin, ID y ficha | Precio declarado | Versión/rango declarado para 19 | Cobertura y condición relevante |
| --- | --- | --- | --- |
| [Multilingual XML, 13](https://appstore.rws.com/plugin/13) | Gratuito | `4.0.1.0`; `19.0.0–19.0.9` | XML bilingüe/multilingüe y subcontenido/CDATA; diferencia respecto al XML base. C-BILINGUAL-TABLE/C-XML. |
| [Multilingual Excel, 17](https://appstore.rws.com/plugin/17) | Gratuito | `3.0.2.3`; `19.0.0–19.0.9` | XLSX con varios idiomas, subcontenido y control de longitud; no es el filtro Bilingual Excel base. C-BILINGUAL-TABLE. |
| [TMX file type, 61](https://appstore.rws.com/plugin/61) | Gratuito | `5.0.1.0`; `19.0.0–19.0.9` | TMX como documento traducible; la descripción limita la expectativa a tamaños razonables. C-CAT. |
| [MXLIFF File Type, 29](https://appstore.rws.com/plugin/29) | Gratuito | `6.0.1.0`; `19.0.0–19.0.9` | Oferta adicional al MXLIFF presente en instalación base; no sumarlo como otra familia. C-CAT. |
| [WorldServer Compatibility Pack, 103](https://appstore.rws.com/plugin/103) | Gratuito | `19.0.2158.0`; `19.0.0–19.9.0` | Bilingües/paquetes de WorldServer 11.x y 10.4.x; recursos y conexiones requieren otro alcance. C-CAT. |
| [Compatibility and Migration Power Pack, 102](https://appstore.rws.com/plugin/102) | Gratuito | **Sin rango 19**; primer registro `18.1.3.187`, `18.1.3–18.1.9` | ITD, TTX, Workbench bilingüe y paquetes heredados; migración de memorias separada. La tabla D26 lo menciona, pero la API no resuelve su disponibilidad para esta instalación. C-CAT. |
| [PDF Assistant, 197](https://appstore.rws.com/plugin/197) | Gratuito | `3.0.2.1`; `19.0.0–19.0.9` | Preconversión PDF→DOCX; usa API de Word en la solución descrita. La gratuidad del add-in no licencia Word ni garantiza OCR/fidelidad. C-PDF. |
| [Studio Subtitling, 5](https://appstore.rws.com/plugin/5) | Gratuito | `5.0.2.1`; `19.0.0–19.0.9` | Contexto audiovisual/edición/QA; la descripción advierte que no funciona con proyectos de documento único. No sustituye todos los filtros de subtítulos. C-SUBTITLE. |
| [ASS File Type, 6](https://appstore.rws.com/plugin/6) | Gratuito | **Sin rango 19**; primer registro `5.1.0.0`, `18.1.0–18.1.9` | `.ass`/`.ssa` Advanced SubStation Alpha. Soporte documentado vía plugin, disponibilidad 19 pendiente. C-SUBTITLE. |
| [STL File Type, 7](https://appstore.rws.com/plugin/7) | Gratuito | `6.0.1.0`; `19.0.0–19.0.9` | STL **Spruce** de subtítulos, no STL de mallas 3D ni inferencia automática sobre EBU STL binario. C-SUBTITLE. |
| [JSON de Supertext, 252](https://appstore.rws.com/plugin/252) | Gratuito | **Sin rango 19**; `3.1.0.0`, `18.0.0–18.9.0` | Variante mono/bilingüe, rutas y subcontenido; no hace que JSON base sea plugin-only. C-JSON. |
| [YAML de Supertext, 258](https://appstore.rws.com/plugin/258) | Gratuito | **Sin rango 19**; `3.1.0.0`, `18.0.0–18.9.0` | Variante con reglas y bilingües; YAML base está presente. C-YAML. |
| [PO de Supertext, 259](https://appstore.rws.com/plugin/259) | Gratuito | **Sin rango 19**; `3.1.0.0`, `18.0.0–18.9.0` | Puede usar target como source para revisión; distinto del contrato PO normal. C-PO. |
| [multifariousCAD, 483](https://appstore.rws.com/plugin/483) | Gratuito | `1.0.2.0`; `19.0.0–19.9.0` | DXF ASCII directo; DWG/DXF binario convertidos. **Siempre entrega DXF ASCII**, no DWG. Corpus limitado declarado por el autor. C-DTP. |
| [multifariousLaTeX, 484](https://appstore.rws.com/plugin/484) | Gratuito | `1.0.0.0`; `19.0.0–19.9.0` | `.tex`, `.ltx`, `.latex`; matemáticas/código protegidos. Preview compilado requiere TeX Live/MiKTeX; extracción no, según descripción. C-MARKDOWN como patrón de marcado, con corpus TeX específico. |
| [multifariousYAML, 488](https://appstore.rws.com/plugin/488) | Gratuito | `1.0.0.0`; `19.0.0–19.9.0` | Mono/bilingüe/multilingüe, anchors, comentarios y salida conjunta de idiomas; no atribuir esta cobertura al filtro base. C-YAML. |
| [multifariousAsciiDoc, 496](https://appstore.rws.com/plugin/496) | Gratuito | `1.0.0.0`; `19.0.0–19.9.0` | `.adoc`, `.asciidoc`, `.asc` y TXT configurado; protege marcado/código y declara verificación con Asciidoctor. No ejecutada aquí. |
| [Sysfilter Illustrator, 114](https://appstore.rws.com/plugin/114) | **De pago** | **Sin rango 19**; `2022.0.0.0`, `17.0.0–17.9.0` | AI/EPS↔XML/DOCX y reinserción mediante herramienta externa; compra en Polmann. No acredita filtro AI/EPS base ni motor abierto. C-DTP. |
| [Sysfilter Photoshop, 115](https://appstore.rws.com/plugin/115) | **De pago** | **Sin rango 19**; `2022.0.0.0`, `17.0.0–17.9.0` | PSD↔XML/DOCX y reinserción externa; distinto del PSD base. Compra y requisitos externos por confirmar antes de adopción. C-PSD. |

Hay anomalías en [T26]: la fila de Multilingual Excel mezcla extensiones ASS/SSA y algunas filas de plugins no identifican claramente la app. Para estas filas se utilizó la ficha/API específica; **no se atribuye ASS/SSA a Excel**. El AppStore es extensible y cambia: esta selección de plugins relevantes no es un censo exhaustivo de filtros comerciales ni de servicios cloud. No se investigaron precios monetarios ni se inició una compra.

## Interoperabilidad abierta desde Rust

### Estándar, vocabulario propietario y contenedor no son lo mismo

| Grupo | Base documental | Consecuencia de diseño/licencia |
| --- | --- | --- |
| OOXML / OPC | ECMA-376 / ISO/IEC 29500; ediciones distintas por parte [OOXML]. | ZIP/XML facilita una implementación propia. Strict, Transitional, macros y extensiones deben tener contratos separados; el estándar no es un motor de layout. |
| OpenDocument | OASIS ODF 1.3, paquetes y XML [ODF]. | Camino Rust viable con ZIP/XML; la versión del estándar elegida no prueba la versión soportada por Trados. Preservar manifiesto, estilos, firmas y recursos. |
| XLIFF 1.2 y 2.0 | Especificaciones OASIS separadas [XLIFF12] [XLIFF20]. | Códigos, segmentación, módulos y extensiones requieren preservación. XLIFF 2.0 advierte que un merger independiente necesita conocer el extractor; un XLIFF no garantiza reconstrucción del nativo. |
| JSON, YAML, Markdown, PO, ITS | RFC 8259; YAML 1.2.2; CommonMark 0.31.2; manual GNU gettext; ITS 2.0 [JSON] [YAML] [COMMONMARK] [PO] [ITS]. | Son referencias para perfiles concretos, no versiones inferidas del filtro de Studio. Reserializar un árbol puede perder la presentación aunque conserve valores. |
| DOC/XLS/PPT binarios y Outlook | Especificaciones del propietario publicadas por Microsoft [MS-FORMATS]. | Documentados, pero no OOXML ni automáticamente formatos de estándar abierto. CFB es el contenedor, no el parser de cada documento. Microsoft advierte sobre patentes/promesas/licencias aplicables. |
| IDML/INX/ICML/MIF/Quark/PSD y dialectos CAT | Catálogo RWS, SDK, especificación PSD y motores con filtros específicos [T26] [SDK] [PSD] [OKAPI-FILTERS]. | XML, ZIP o texto no elimina semántica propietaria. Evaluar corpus, cobertura, redistribución y licencias de cada componente; ausencia de fuente consultada no prueba que no exista. |

La licencia de una **especificación** y la licencia de su **implementación** son distintas. Leer documentación pública no concede automáticamente derechos para copiar código, redistribuir runtimes comerciales ni reutilizar DLL instaladas. Este informe no sustituye revisión jurídica del artefacto elegido.

### Bibliotecas Rust candidatas, no aprobadas ni instaladas

Licencias y versiones publicadas consultadas en la API de crates.io el día del corte; documentación de los autores contrastada en docs.rs. La versión anotada identifica el candidato observado, **no una recomendación de instalar `latest`**. Revisar licencia real de la versión, dependencias transitivas, mantenimiento, cooldown y protección de suministro antes de incorporarlo.

| Candidato observado / fuente del autor | Licencia declarada | Capacidad útil y límite |
| --- | --- | --- |
| [calamine 0.36.1](https://docs.rs/calamine/0.36.1/calamine/) | MIT | Lector Excel/ODS. No ofrece por esa descripción escritura conservadora ni round-trip de fórmulas/estilos. |
| [rust_xlsxwriter 0.99.1](https://docs.rs/rust_xlsxwriter/0.99.1/rust_xlsxwriter/) | MIT OR Apache-2.0 | Generador XLSX. Crear un libro nuevo desde valores extraídos no preserva automáticamente el libro original. |
| [umya-spreadsheet 3.1.0](https://docs.rs/umya-spreadsheet/3.1.0/umya_spreadsheet/) | MIT | Lectura/escritura XLSX; candidato comparativo. No hay evidencia aquí de preservación de todas las partes desconocidas/macros. |
| [spreadsheet-ods 1.0.4](https://docs.rs/spreadsheet-ods/1.0.4/spreadsheet_ods/) | MIT/Apache-2.0 | Lectura/escritura ODS; no atribuirle soporte ODT/ODP ni fidelidad general ODF. |
| [html5gum 0.8.4](https://docs.rs/html5gum/0.8.4/html5gum/) | MIT | Tokenizador HTML5; evaluar posiciones y conservación del marcado original. No es política de extracción ni serializer CAT. |
| [pulldown-cmark 0.13.4](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/) | MIT | Parser CommonMark; rangos de origen pueden servir para editar prosa sobre el esqueleto. No prometer todos los dialectos ni serializer lossless. |
| [yaml-rust2 0.13.0](https://docs.rs/yaml-rust2/0.13.0/yaml_rust2/) | MIT OR Apache-2.0 | Parser YAML 1.2; conservación de comentarios, estilo y anchors necesita evaluación aparte. |
| [polib 0.3.0](https://docs.rs/polib/0.3.0/polib/) | MIT | Leer/manipular/guardar PO. Validar plurales, flags, comentarios, escapes y codificación con corpus. |
| [mailparse 0.18.0](https://docs.rs/mailparse/0.18.0/mailparse/) | 0BSD | Parser MIME; no cubre MSG/OFT ni acredita escritura conservadora. **Esta versión se publicó el mismo día del corte:** no eludir cooldown. |
| [cfb 0.15.0](https://docs.rs/cfb/0.15.0/cfb/) | MIT | Leer/escribir Compound File Binary; falta el significado de streams DOC/XLS/PPT/MSG. |
| [lopdf 0.45.0](https://docs.rs/lopdf/0.45.0/lopdf/) | MIT | Manipulación PDF; no es OCR ni PDF→DOCX con layout fiel. |
| [pdfium-render 0.9.4](https://docs.rs/pdfium-render/0.9.4/pdfium_render/) | MIT OR Apache-2.0 | Wrapper Rust de Pdfium C++; despliegue nativo y licencias de Pdfium/terceros aparte. Render no equivale a reinserción de traducción. |
| [subparse 0.7.0](https://docs.rs/subparse/0.7.0/subparse/) | MPL-2.0 | Lectura/cambio/escritura de SRT/ASS/IDX/SUB; no inferir VTT/TTML ni todas las variantes de SUB. Evaluar mantenimiento y obligaciones MPL. |
| [dotnetdll 0.3.0](https://docs.rs/dotnetdll/0.3.0/dotnetdll/) | GPL-3.0+ | Leer/escribir metadatos .NET; no es un filtro CAT de recursos listo. Dependencia con copyleft fuerte: decisión de licencia previa a integración. |

Para una auditoría reproducible de metadatos, el endpoint es `https://crates.io/api/v1/crates/{nombre}/{versión}` y el campo relevante es `version.license`. La declaración del publicador no sustituye revisar los archivos LICENSE y el árbol bloqueado. Ninguna de estas descripciones demuestra preservación sin pérdidas.

### Motores externos y tradeoffs

| Motor | Cobertura documentada relevante | Licencia/dependencia | Evaluación para LumenCAT |
| --- | --- | --- | --- |
| **Okapi Framework / Tikal** | OOXML, ODF, IDML/ICML, MIF, HTML/XML/ITS, JSON/YAML/Markdown, PO/properties, XLIFF, TMX, TTX, RTF Workbench y paquetes SDL/WS [OKAPI-FILTERS] [TIKAL]. | Código Apache-2.0 [OKAPI-LICENSE]; motor Java, no Rust. Auditar distribución/dependencias elegidas. | Mejor candidato externo abierto por amplitud documental. Rust puede invocar un proceso local; añade runtime, despliegue y contrato de skeleton/versiones. El filtro PDF entrega **TXT**, no PDF ni Word [OKAPI-PDF]. |
| **OpenXLIFF** | Extracción/merge de Office moderno, ODF, IDML/INX/ICML, MIF, web/software, SRT/VTT, SDLXLIFF y paquetes SDLPPX; XLIFF 1.2/2.0/2.1/2.2 [OPENXLIFF]. | Principal EPL-1.0; README exige JDK 25/Gradle 9.5 para construir el código consultado. BCP47J/XMLJava con licencia restrictiva [OPENXLIFF-DEPS]. | Referencia técnica útil, **no autorizar distribución/producción del conjunto por llamarse “Open”**. Resolver licencias o evaluar un corte histórico íntegramente abierto y mantenible; no se investigó ni aprobó tal corte aquí. |
| **LibreOffice** | Filtros de importación/exportación de Office/ODF y conversiones documentadas [LO-FILTERS]. | MPL-2.0, con componentes bajo otras licencias [LO-LICENSE]; proceso externo, no runtime Trados. | Posible puente explícito para DOC/XLS/PPT heredados. Conversión no es preservación binaria: se deben declarar salida nueva y pérdidas detectadas. LibreOffice no prueba render idéntico en Microsoft Office. |
| **Filtro propio Rust sobre original** | XML/ZIP y formatos textuales cuyos perfiles se implementen; dependencias ya presentes. | Sin reutilizar DLL de Trados; revisar licencias de crates y especificaciones. | Menor coste de despliegue y mayor control de partes desconocidas. Mayor trabajo de extracción/merge y cobertura: no crear parsers binarios o motores de layout generales para ganar una casilla. |

Un puente opcional no debe reemplazar silenciosamente el filtro nativo. Guardar nombre/versión del motor, configuración, original, skeleton y hashes de recursos. El merge debe utilizar el motor compatible con la extracción. Nunca enviar documentos a validadores/servicios online sin autorización ni convertir «el motor existe» en «LumenCAT ya lo soporta».

## Contrato de aceptación observable

**Son requisitos propuestos para una implementación futura. Ningún recorrido de esta sección se ejecutó en esta investigación.** La aceptación tiene tres resultados separados: importar contenido correcto, editarlo sin romper códigos y generar un artefacto consumible. Compilación o reimportación propia son evidencias auxiliares, no sustitutos de la aplicación destino.

### Puertas comunes para cada formato/variante

1. **Identificación y extracción:** usar firma, versión, namespaces/content types y configuración cuando aplique. Comparar contra un inventario de textos y ubicaciones esperado, incluyendo negativos. Rechazar explícitamente variantes no admitidas; no producir importaciones vacías «correctas».
2. **Traducción y persistencia:** aplicar targets reconocibles con Unicode, texto más largo, caracteres reservados y códigos inline. Guardar/reabrir el proyecto; verificar IDs, orden permitido, bloqueos, estados y los textos que no se deben traducir. Romper un código obligatorio debe impedir una entrega inválida.
3. **Generación y conservación:** entregar el tipo prometido, sin sobrescribir el original. En rutas conservadoras comparar bytes de partes/intervalos no editados; en paquetes comparar nombres, relaciones y hashes de recursos. Si se utiliza conversión, registrar las diferencias esperadas y no llamarla round-trip lossless.
4. **Consumidor independiente:** abrir/validar con la herramienta adecuada, identificando versión y avisos de reparación. Comprobar texto traducido, estilos y relaciones; revisar overflow/layout cuando corresponda. Una traducción más larga puede cambiar paginación sin implicar corrupción, pero debe detectarse el desbordamiento.
5. **Rechazo, límites y evidencia:** archivos truncados/cifrados/no soportados, tamaños/expansión, cancelación y destino existente deben dejar el original intacto. Registrar fixture/hash, variante, ajustes, extractor, resultado, salida/hash y consumidor. Credenciales, macros y documentos personales no forman parte del corpus.

Para comparar con Trados deben utilizarse el mismo par de idiomas y ajustes equivalentes. La segmentación puede diferir legítimamente; comparar cobertura de contenido y su reinserción, no exigir IDs o número de segmentos idénticos si cada motor define otras fronteras.

### Corpus y resultados por familia

| Criterio | Importar: fixture mínimo significativo | Traducir: resultado observable | Exportar: consumidor y conservación |
| --- | --- | --- | --- |
| **C-TEXT** | BOM/no BOM, LF/CRLF/CR, líneas vacías, Unicode y codificación admitida; reglas de extracción si se prometen. | Target correcto por ubicación; exclusiones y finales de línea conservados. | Lector independiente; bytes fuera de texto intactos. Rechazo explícito de codificación no admitida. |
| **C-DELIMITED** | Varias columnas, comillas/delimitadores en valores, saltos dentro de celda y filas vacías; perfil CSV/TAB guardado. | Solo columnas traducibles; claves/celdas excluidas intactas. | Parser tabular independiente obtiene mismas filas/columnas y targets; no cambiar fórmulas ni añadir celdas. |
| **C-XLIFF** | 1.2 con/sin `seg-source`, inline anidados, multi-file, IDs repetidos válidos entre files; fixture 2.0 con `unit/segment`, `originalData`, módulos. | Cambiar target sin aplanar códigos, source ni skeleton; conservar estados/extensiones según contrato. | Validación de esquema más restricciones semánticas; apertura en otra herramienta CAT. Implementar/testear 1.2 y 2.0 por separado. |
| **C-CAT** | SDLXLIFF y cada dialecto desde el generador identificado, pretraducciones, locks, estados, comentarios y referencias a skeleton; contenedor cuando corresponda. | Edición y reexportación bilingüe mantienen metadatos desconocidos y asociación segmento/target. | Reabrir en herramienta origen. **Prueba distinta** para generar documento nativo o paquete de retorno; si no existe generador compatible, anunciar solo entrega bilingüe. |
| **C-OFFICE** | DOCX con formato mixto, listas/tablas, hyperlinks, campos, headers/footers, notas/comentarios, imágenes y variantes Strict/Transitional separadas. | Prosa continua con códigos protegidos; estilos/historias cubiertos o rechazo localizado, no pérdidas silenciosas. | Word de versión registrada abre sin reparar; relaciones/media intactos; inspección de estilos/layout. DOCM/DOT* en contratos adicionales. |
| **C-PPT** | PPTX con notas, masters/layouts, formas, tablas, gráficos, SmartArt y texto oculto; ajustes de inclusión. | Solo ubicaciones seleccionadas; no alterar animaciones, fórmulas/datos ni recursos. | PowerPoint abre sin reparar; orden, geometría, notas y recursos se conservan; detectar overflow. |
| **C-XLSX** | Shared/inline strings, rich text, fórmulas, fechas/números, hojas ocultas, celdas combinadas, comentarios, gráficos y XLSM separado. | Solo strings elegidas; mismos tipos y fórmulas; no traducir números/formulas por parecer texto. | Excel abre sin reparar; partes/recursos no editados conservados. Comprobar cambios en shared strings no contaminan celdas excluidas que compartían índice. |
| **C-BILINGUAL-TABLE** | XLSX/XML con source, target parcial, contexto/comentarios y varios idiomas; mapeo explícito. | Source no cambia; target existente se revisa, no se descarta; cada idioma se guarda en su ubicación. | Misma tabla/estructura, columnas excluidas intactas; salida conjunta multilingüe solo si se anuncia e implementa. |
| **C-VISIO** | Dibujos y stencils/plantillas por variante, formas conectadas, masters, texto formateado. | Traducir texto sin modificar IDs, rutas ni conexiones. | Visio abre sin reparar y mantiene geometría/conexiones; verificar overflow y tipos de paquete. |
| **C-RTF** | Grupos anidados, Unicode/escapes, tablas, fuentes, imágenes y texto no traducible. | Cambiar texto sin editar controles/grupos; Workbench bilingüe fuera de este perfil. | Procesador RTF independiente mantiene estructura y recursos; apertura visual si se promete fidelidad. |
| **C-ODF** | ODT/ODP/ODS y plantillas/master por separado; estilos, notas, fórmulas/celdas repetidas y recursos. | Solo zonas del perfil; códigos y repeticiones mantienen significado. | LibreOffice abre sin reparación; manifiesto, MIME, recursos y fórmulas íntegros; comparación de contenido y layout. |
| **C-LEGACY** | DOC/XLS/PPT binarios reales por versión; objetos/streams desconocidos, plantillas; WPS aparte. | Extraer contenido del parser correspondiente, no de un lector CFB genérico. | Consumidor nativo confirma formato y recursos. Si se convierte a OOXML, mostrar cambio de formato y cotejar contenido/estructura en ambos extremos. |
| **C-DTP** | MIF/INX/IDML/ICML/XTG/TAG por separado; estilos, condiciones, referencias, tablas y recursos. CAD requiere entidades/encoding/corpus propio. | Códigos y referencias protegidos; solo texto del perfil. | Aplicación de maquetación/CAD importa la salida y muestra targets, sin roturas de estructura; revisar overflow. Si entrada DWG entrega DXF, identificar conversión. |
| **C-PSD** | Capas de texto editable, máscaras, estilos de texto, grupos y recursos; negativos rasterizados. | Traducir texto editable sin OCR implícito ni modificar imágenes/capas excluidas. | Photoshop reabre capas editables; píxeles/máscaras/recursos íntegros y métricas revisadas. Una preview raster correcta no prueba edición del texto. |
| **C-XML** | Namespaces/prefijos alternativos, atributos, contenido mixto, CDATA, comentarios/PI y reglas locales/globales; DTD con política explícita. | Solo nodos/atributos seleccionados, inline protegidos y escaping correcto; conservar el vocabulario desconocido. | Parser/validador independiente y consumidor del vocabulario; bytes ajenos a cambios conservados si se promete. No descargar entidades externas. |
| **C-XML-PROFILE** | Corpus DITA/DocBook/Author-it/MadCap/AEM/ITS **por perfil**; variables, condiciones, referencias y exclusiones. | Mismas zonas seleccionadas, IDs y referencias; reglas ITS heredan/preceden según versión declarada. | Validación del vocabulario y consumo por publicador/importador; mapas/conrefs/paquetes solo si pertenecen al alcance anunciado. |
| **C-HTML** | HTML tolerante y XHTML separados; entidades, texto mixto, alt/title, scripts/CSS y plantillas de servidor. | Traduce prosa/atributos elegidos; URLs y código quedan protegidos. | Parser/browser con scripts/red deshabilitados para fixture; estructura y enlaces intactos. No reserializar toda la página si se promete conservación lexical. |
| **C-JSON** | Rutas incluidas/excluidas, arrays, escapes Unicode, números precisos, duplicados con política y HTML/ICU embebido. | Solo valores string elegidos; tipos, claves y placeholders no cambian. | Parser JSON independiente y aplicación/corpus de recursos; modificar escapes sin alterar significado ni números. Perfil bilingüe separado. |
| **C-YAML** | Escalares quoted/plain/block, folding/chomping, comentarios, documentos múltiples, anchors/aliases y tipos implícitos. | Solo strings del perfil; traducción de anchor no duplica/rompe aliases; otros idiomas intactos. | Parser YAML del consumidor conserva tipos/estructura; comentarios y presentación según promesa. No basta comparar el árbol deserializado. |
| **C-MARKDOWN** | Dialecto declarado, títulos/listas, tablas si aplican, énfasis, enlaces/alt, fences, código inline y HTML embebido. | Prosa continua, URLs/código protegidos, caracteres de marcado escapados correctamente. | Renderer del dialecto confirma misma estructura, referencias y código; salida sigue siendo Markdown, no HTML generado. TeX requiere compilador/corpus adicional. |
| **C-PO** | Contextos, plurales, fuzzy/obsolete, comentarios, placeholders y charset/header. | `msgstr` cambia; `msgid`, contexto, placeholders y número de plurales se mantienen. | GNU gettext valida/compila y consumidor encuentra targets; no perder flags ni entradas al reserializar. |
| **C-RESOURCE** | Properties con escapes/continuaciones y RESX con strings/binarios; placeholders por sintaxis de aplicación. | Claves, tipos, recursos binarios y variables intactos; codificación admitida explícita. | API/compilador de recursos Java/.NET carga mismas claves y targets. |
| **C-DOTNET** | Ensamblado desechable con recursos/culturas, metadatos y firma; sin ejecutar contenido de entrada. | Solo recursos string; mismos IDs/código y política de firma explícita. | Inspector/cargador de recursos controlado; comparar código/metadatos; no entregar una firma inválida como válida. |
| **C-EMAIL** | EML MIME multipart/text+HTML y adjuntos; MSG/OFT y EMLX en corpus independientes. | Asunto/cuerpo seleccionados; direcciones, boundaries, adjuntos y encabezados excluidos conservados. | Parser/cliente en entorno desechable **sin envío**; mensajes y adjuntos legibles; firmas criptográficas alteradas deben declararse. |
| **C-SUBTITLE** | SRT/VTT/SBV/SUB/TTML/ASS/STL por dialecto; cues multilineales, estilos, IDs/times y Unicode. | Solo cue text, tiempos/estilos protegidos; QA de duración/longitud cuando se anuncie. | Parser/reproductor del dialecto muestra targets en mismos tiempos y mantiene metadatos; no confundir subtítulos textuales con bitmap IDX/SUB. |
| **C-PDF** | PDF con texto, PDF OCR previo e imagen sin texto; columnas/tablas/fuentes complejas. | Cobertura y orden de lectura comprobados sobre el documento convertido; original PDF permanece intacto. | Entrega **DOCX/Word si ese es el servicio prometido**, abierto independientemente; informar fallos/pérdidas de conversión/OCR. Exportar PDF es otra función, no acreditada por la paridad con el filtro base. |

## Prioridades y dependencias propuestas

Orden de implementación recomendado, no trabajo autorizado en esta entrega:

| Prioridad | Resultado acotado | Dependencias y puerta de salida |
| --- | --- | --- |
| **P0: conservación y bilingües útiles** | Consolidar códigos protegidos, persistencia/QA, completar XLIFF 1.2 segmentado y ampliar inline/DOCX según el corpus objetivo. | Reutilizar XML/ZIP y adaptador inline actuales; definir política de estados y preservación de contenido desconocido. C-XLIFF/C-OFFICE y puertas comunes antes de ampliar promesas. No asumir que representación de tags ya resuelve todos los dialectos. |
| **P1: formatos textuales** | CSV/TAB, JSON, XML/RESX, PO/properties y SRT/VTT; cada filtro con selección/encoding explícitos. | P0 para marcado/variables; reglas y skeleton por formato. Empezar con dependencias instaladas; añadir librería solo si reduce trabajo real. Corpus positivo/negativo y consumidor independiente. |
| **P2: paquetes de oficina** | XLSX monolingüe/bilingüe, PPTX, ODT/ODS/ODP; Visio después según demanda. | Conservación por partes, mapeo de ubicaciones/idiomas, ajustes de extracción y QA de recursos; evaluar parser nativo frente a Okapi con el mismo corpus. No combinar lector de valores y escritor de libros nuevos como supuesto round-trip. |
| **P3: CAT propietario y maquetación** | SDLXLIFF/dialectos, paquetes de retorno, IDML/ICML/MIF según documentos reales. | Preservar extensiones/skeleton, consumidor origen identificado y separar edición de generación nativa. Okapi opcional puede reducir coste, a cambio de runtime Java y versionado del merge. |
| **P4: conversión y binarios difíciles** | DOC/XLS/PPT heredados, PDF/OCR, PSD, .NET y CAD solo con demanda/corpus. | Evaluación de licencia y dependencias externas; declarar transformaciones/pérdidas. LibreOffice como puente posible; no reutilizar runtime Trados ni introducir un motor de layout propio por defecto. |

YAML/Markdown avanzados y plugins multilingües necesitan conservación lexical y un perfil/dialecto explícito; su apariencia textual no los convierte en filtros triviales. La amplitud de plugins no justifica implementarlos todos antes de demostrar P0/P1.

### Pendientes que impiden afirmar paridad

- Licencia/edición activa de Studio, filtros habilitados y plugins de usuario: **no inspeccionados** deliberadamente.
- Disponibilidad real para Studio 19 de ASS y Compatibility and Migration Power Pack: divergencia documental/API sin resolver mediante instalación.
- Versiones de formatos no precisadas por RWS, dialectos y constructos admitidos por cada filtro: requieren documentación específica y corpus; años del nombre no bastan.
- Cobertura, precisión OCR, rendimiento, tamaños máximos y fidelidad de conversión: **no medidos**.
- Importar/traducir/exportar en GUI de LumenCAT y abrir la entrega en Word/Trados/Adobe/otros consumidores: **no ejecutado**. Si se autoriza después, seguir `.cursor/skills/verify-lumencat/SKILL.md`, su mapa y el controlador en cola con proyectos desechables bajo `output/verification/`; no lanzar el EXE por cuenta propia ni tomar foreground sin autorización.

## Evidencia local reproducible

| Objeto inspeccionado | Resultado |
| --- | --- |
| `C:\Program Files\Trados\Trados Studio\Studio19\SDLTradosStudio.exe` | `FileVersion=19.0.0.3043`, `ProductVersion=Studio19`, `ProductName=Trados Studio`. |
| `Studio19\pluginconfig.xml` | Producto `TradosStudio`, versión `19.0`; plugins de terceros habilitados en configuración y rutas de Packages/Unpacked declaradas. No prueba que un plugin concreto esté instalado. |
| `Studio19\PlugIns\Sdl.FileTypeSupport.*.plugin.xml` | **42 manifiestos** UTF-16. GenericDelimited registra CSV/TAB/plantilla; Office registra DOCX/PPTX/XLSX/subcontenido; ODF registra tres builders. |
| Recursos neutros `*.plugin.resources` | Strings leídas con `ResourceReader.GetResourceData`, decodificando solo `ResourceTypeCode.String`, sin cargar DLL ni deserializar tipos personalizados. |

Inventario exacto de sufijos de los 42 manifiestos, conservando la capitalización observada:

```text
Bilingual.SdlXliff
Filters.Bilingual.FrameMaker
Filters.BilingualExcel
Filters.Doc
Filters.DotNetLibraries
Filters.Email
Filters.GenericDelimited
Filters.Html
Filters.Icml
Filters.Idml
Filters.Inx
Filters.JavaResource
Filters.Json
Filters.Markdown
Filters.MicrosoftOffice_2
Filters.Odf
Filters.Photoshop
Filters.PO
Filters.Ppt
Filters.QuarkXPress
Filters.Subtitles
Filters.Visio
Filters.Xliff.MemoQ
Filters.Xliff.MXliff
Filters.Xliff
Filters.Xliff2
Filters.Xls
Filters.Xml_2.AemSites
Filters.Xml_2.AuthorIT
Filters.Xml_2.Dita
Filters.Xml_2.DocBook
Filters.Xml_2.Its
Filters.Xml_2.MadCap
Filters.Xml_2
Filters.Xml_2.Resx
Filters.Xml_2.Template
Filters.Xml_2.Xhtml
Filters.Yaml
Native.FrameMaker_2
Native.PDF
Native.RegEx_1_1
Native.RTF_2
```

Ejemplo mínimo para repetir el inventario **sin iniciar Trados**:

```powershell
$studio = 'C:\Program Files\Trados\Trados Studio\Studio19'
$v = (Get-Item -LiteralPath "$studio\SDLTradosStudio.exe").VersionInfo
$v | Select-Object FileVersion, ProductVersion, ProductName
$files = Get-ChildItem -LiteralPath "$studio\PlugIns" -Filter 'Sdl.FileTypeSupport.*.plugin.xml'
$files.Count
$files | ForEach-Object {
    [xml]$manifest = Get-Content -LiteralPath $_.FullName -Raw
    [pscustomobject]@{
        Manifest = $_.Name
        PluginVersion = $manifest.plugin.version
        Builders = (@($manifest.plugin.extension) | ForEach-Object { $_.type.Split(',')[0] }) -join ', '
    }
}
```

Versiones de plugin/manifiesto, IDs de filtro y versiones del **formato** son ejes diferentes. El inventario está limitado a esta carpeta y patrón; no pretende censar todos los componentes, verificadores, integraciones ni el ecosistema AppStore.

## Fuentes primarias y trazabilidad

Todas las fuentes se consultaron para el corte indicado; enlaces vivos pueden cambiar. Los puntos decisivos de licencias de OpenXLIFF se fijan a commits. Las afirmaciones de disponibilidad proceden de RWS/AppStore; las de capacidades de bibliotecas proceden de sus autores, no de pruebas locales.

| Fuente | Uso y advertencia |
| --- | --- |
| [Tabla Studio 2026][T26] | Inventario principal. URL recuperada del enlace oficial: `/en-US/1279521/334056/trados-studio-2026-release/supported-file-types`. |
| [Specific file types 2026][SPECIFIC] | Navegación hacia familias y tabla real; se inspeccionó HTML porque la extracción Markdown inicialmente devolvió solo título. |
| [Marketing desktop/cloud][MARKETING] y [tabla 2024][T24] | Contexto/histórico. No atribuir soporte cloud ni complementos AI Multimedia a la instalación desktop. |
| [Cambios de filtros 2024][EOL] | Retirada de filtros heredados y diferencia entre editar SDLXLIFF y generar destino. No confundir con retiro del formato completo. |
| [SDK de FileTypeSupport][SDK] y [ajustes 17.1][SDK-SETTINGS] | Modelo nativo/bilingüe, SDLXLIFF y dependencia del filtro para generación. El SDK advierte que puede contener información desactualizada; no prevalece sobre catálogo/API actuales para disponibilidad. |
| [PDF 2026][PDF] y [subtítulos 2026][SUBTITLE] | Límites de salida/OCR y sustitución de SubRip. |
| [AppStore API][APP] | Consulta pública sin autenticación, cabecera `apiversion: 2.0.0`, selección explícita de IDs en matriz. `paidFor=false` no significa licencia open source. |
| [Okapi filtros][OKAPI-FILTERS], [licencia][OKAPI-LICENSE], [Tikal][TIKAL] y [PDF][OKAPI-PDF] | Motor alternativo/documentación del autor. Las páginas tienen fechas distintas; verificar distribución concreta antes de adopción. |
| [OpenXLIFF][OPENXLIFF], [licencia principal][OPENXLIFF-LICENSE] y [dependencias][OPENXLIFF-DEPS] | Cobertura anunciada frente a licencias transitivas. No se descargó/ejecutó distribución. |
| [BCP47J][BCP47J-LICENSE] y [XMLJava][XMLJAVA-LICENSE] | Restricciones de producción, comercialización y redistribución; ambas requieren permiso para esos usos. |
| [LibreOffice filtros][LO-FILTERS] y [licencias][LO-LICENSE] | Conversión opcional abierta, no prueba de conservación ni fidelidad con Microsoft Office. |

Incidencias de recuperación: la URL inferida `https://docs.rws.com/en-US/trados-studio-2026-release-1279521/supported-file-types-334056` inicialmente devolvió contenido ajeno al tema, por lo que **no se usó como tabla**. La ruta correcta es [T26]. Varias rutas inferidas de IDML/ODF y un enlace de Microsoft devolvieron error; se sustituyeron los enlaces de ODF/Microsoft por fuentes verificadas, y se mantuvo explícita la ausencia de especificación IDML actual completa. Las celdas diferidas y la mezcla ASS/SSA en Excel se contrastaron, no se completaron por adivinación.

[T26]: https://docs.rws.com/en-US/1279521/334056/trados-studio-2026-release/supported-file-types
[SPECIFIC]: https://docs.rws.com/en-US/trados-studio-2026-release-1279521/specific-file-types-348048
[MARKETING]: https://www.trados.com/ecosystem/languages-and-file-types/
[T24]: https://docs.rws.com/en-US/trados-studio-2024-1145319/supported-file-types-334056
[EOL]: https://docs.rws.com/en-US/trados-studio-2024-1145319/file-types-and-core-components-changes-1148909
[BILINGUAL-EXCEL]: https://docs.rws.com/en-US/trados-studio-2024-1145319/added-support-for-bilingual-excel-files-278085
[PDF]: https://docs.rws.com/en-US/trados-studio-2026-release-1279521/pdf-551979
[SUBTITLE]: https://docs.rws.com/en-US/trados-studio-2026-release-1279521/subtitle-formats-887585
[SDK]: https://developers.rws.com/studio-api-docs/apiconcepts/filetypesupport/file_type_support_overview.html
[SDK-SETTINGS]: https://developers.rws.com/studio-api-docs/17.1/apiconcepts/filetypesupport/file_type_settings.html
[APP]: https://api-appstore.rws.com/app-store-api/v1/plugins?excludeHtmlContent=true
[OOXML]: https://ecma-international.org/publications-and-standards/standards/ecma-376/
[ODF]: https://docs.oasis-open.org/office/OpenDocument/v1.3/OpenDocument-v1.3-part2-packages.html
[XLIFF12]: https://docs.oasis-open.org/xliff/v1.2/os/xliff-core.html
[XLIFF20]: https://docs.oasis-open.org/xliff/xliff-core/v2.0/os/xliff-core-v2.0-os.html
[JSON]: https://www.rfc-editor.org/rfc/rfc8259.html
[YAML]: https://yaml.org/spec/1.2.2/
[COMMONMARK]: https://spec.commonmark.org/0.31.2/
[PO]: https://www.gnu.org/software/gettext/manual/html_node/PO-Files.html
[ITS]: https://www.w3.org/TR/its20/
[MS-FORMATS]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-offfflp/8aea05e3-8c1e-4a9a-9614-31f71e679456
[PSD]: https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/
[OKAPI-FILTERS]: https://okapiframework.org/wiki/index.php/Filters
[OKAPI-LICENSE]: https://gitlab.com/okapiframework/Okapi/-/raw/main/LICENSE
[OKAPI-PDF]: https://okapiframework.org/wiki/index.php/PDF_Filter
[TIKAL]: https://okapiframework.org/wiki/index.php/Tikal
[OPENXLIFF]: https://github.com/maxprograms-com/OpenXLIFF
[OPENXLIFF-LICENSE]: https://raw.githubusercontent.com/maxprograms-com/OpenXLIFF/master/LICENSE
[OPENXLIFF-DEPS]: https://github.com/maxprograms-com/OpenXLIFF/blob/dae19c8e326399c912f1fd1c4b1ead326c4fcf16/licenses/README.md
[BCP47J-LICENSE]: https://github.com/maxprograms-com/BCP47J/blob/16d7770d91de296992009ceb7e1c21524097ccc1/LICENSE.md
[XMLJAVA-LICENSE]: https://github.com/maxprograms-com/XMLJava/blob/33eb8c3fb5b5f2848da586b69b3809e2ccc3f21c/LICENSE.md
[LO-FILTERS]: https://help.libreoffice.org/latest/en-US/text/shared/guide/convertfilters.html
[LO-LICENSE]: https://www.libreoffice.org/about-us/licenses/
