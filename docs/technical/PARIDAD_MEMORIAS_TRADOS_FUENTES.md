# Paridad de memorias con Trados: formatos, actualización fiel y fuentes primarias

Fecha: 5 de octubre de 2026. Investigación documental y de instalación solicitada por el coordinador. No se ejecutó Trados, no se abrió la GUI ni se modificaron memorias del usuario; la propiedad del código y la integración pertenece al coordinador (ver [BRECHAS_TRADOS.md](BRECHAS_TRADOS.md) para el plan general y [MEMORIAS.md](../guides/MEMORIAS.md) para el comportamiento actual).

Esta investigación no reduce el encargo de paridad: legados y motores propietarios permanecen pendientes o dependientes de componentes externos, no descartados por decisión del agente. El estado de implementación evoluciona después de la lectura inicial; consultar [TMX con códigos](TMX_INLINE_20261005.md).

Entorno verificado localmente (solo lectura):

- Trados Studio instalado en `C:\Program Files\Trados\Trados Studio\Studio19`; `Sdl.LanguagePlatform.TranslationMemoryApi.dll` con `ProductVersion/FileVersion 19.0.0.3043`, copyright «2000 - 2026 RWS Holdings plc» (Studio 19 = Trados Studio 2026; la documentación oficial del SDK referencia «Trados Studio 2026 Release» y la carpeta `...\Trados Studio\Studio19` como ruta vigente [1]).
- Memorias de ejemplo distribuidas con la instalación (`Samples\Projects\SampleProject\TMs\English-German.sdltm`, `English-French.sdltm`, `English-Japanese.sdltm`). Se copiaron a `output/verification/sdltm-inspect/` (fixtures desechables) y se inspeccionaron con SQLite en modo `mode=ro`; los originales no se tocaron.

## Resumen de decisiones recomendadas

1. **Lectura SDLTM: sí, nativa/portable, solo lectura.** Implementar un lector SQLite en Rust que extraiga TUs (texto, tags serializados y campos del sistema) desde una copia o en `mode=ro`, con la correspondencia Segment↔TMX documentada abajo. No existe escritura directa segura.
2. **Escritura/actualización SDLTM: no hacerla en SQL propio.** La semántica oficial de actualización exige mantener hashes, blobs de tokenización versionados, índices fuzzy de n-gramas, contextos, fragmentos, vocabulario y contadores; escribir solo `translation_units` degrada o corrompe el TM [2][3][4]. No se encontró ninguna biblioteca abierta madura que escriba SDLTM directamente; todo el ecosistema oficial y comunitario escribe a través de la API de Studio [5][6].
3. **Intercambio y actualización fiel: TMX 1.4b como formato puente.** Para «actualizar» una memoria Trados desde LumenCAT, exportar TMX (dialecto SDL documentado: props `x-*`, atributos de sistema) y dejar que Trados lo importe; o usar un helper externo basado en la API pública de Studio (dependiente de la instalación/licencia de Studio) solo como acción explícita y aislada. Nunca sobrescribir el `.sdltm` original: operar sobre copia y reemplazo atómico, o escribir a un archivo nuevo.
4. **Legados `.tmw`/`.mdb` y memorias de servidor: categorías distintas, sin soporte nativo.** El upgrade oficial de legados vive hoy en el app *Trados Compatibility and Migration Power Pack* de la RWS AppStore [7]; las memorias de servidor (GroupShare/TM Server) requieren conexión y credenciales [1][8]. LumenCAT no debe anunciar compatibilidad con ninguno de los dos; a lo sumo leer TMX exportado por esas vías.
5. **Sin compatibilidad falsa:** declarar por operación lo que se admite (leer/importar/exportar), rechazar lo que no se puede preservar (p. ej. TU con tags cuando LumenCAT aún no modela inline codes — hoy `import_tmx` ya los rechaza) y conservar las fuentes inmutables.

## 1. Formatos que Trados acepta, exporta y modifica (con fuentes primarias)

| Formato | ¿Studio lo acepta? | ¿Lo exporta? | ¿Lo modifica (update)? | Categoría |
|---|---|---|---|---|
| **SDLTM** (TM de archivo) | Sí; es el formato nativo de TM de archivo (`FileBasedTranslationMemory`) [2][9] | Sí (a TMX; también gzip `.tmx.gz` según documentación del ensamblado) | Sí, vía API `AddTranslationUnit`/`EditTu`/`DeleteTranslationUnit` y wizard de import [2][3] | Archivo nativo |
| **TMX 1.4** | Sí, importación a SDLTM con `TranslationMemoryImporter` (chunking, filtros de confirmación, exclusión de inválidas a archivo aparte) [10] | Sí, `TranslationMemoryExporter`, con filtros por campos [11] | Indirecto: importa/actualiza TUs existentes según `ExistingTUsUpdateMode` [10] | Intercambio estándar |
| **TXT** (export Workbench/WinAlign) | Sí, vía wizard de upgrade [7] | — | No directamente; se convierte a SDLTM | Legado de exportación |
| **TMW** (Translator's Workbench / Trados 2007) | Solo mediante upgrade; desde Studio 2021 el wizard exige el *Power Pack* de la AppStore [7] | — | No; se convierte a SDLTM | Legado |
| **MDB** (SDLX) | Solo mediante upgrade con *Power Pack* [7] | — | No; se convierte a SDLTM | Legado |
| **TM de servidor** (GroupShare/TM Server, SQL Server) | Sí, con conexión y credenciales (`ServerBasedTranslationMemory`, `TranslationProviderServer`) [1][9] | Sí (import/export programados) | Sí, en servidor | Servidor (producto aparte) |

Notas sobre la tabla:

- La ayuda oficial documenta que el wizard de upgrade convierte «legacy TMs, `.TMX` files and `.TXT` files (SDL Workbench or WinAlign Export) to the Trados Studio format (`.SDLTM`)» y que desde 2021 ese wizard «is no longer available out-of-the-box», requiriendo el Power Pack, que habilita: combinar TMs de Studio, actualizar `.TMW` y `.MDB` a `.SDLTM`, migrar TMX→SDLTM por lotes y migrar entre TM de archivo y servidor GroupShare [7]. (Fuente renderizada vía Wayback de `docs.rws.com` porque el portal actual carga por JavaScript.)
- El propio ensamblado instalado documenta la importación programada de «a TMX, SDLIFF, ITD or TTX file into a **server-based** translation memory» (clase `ScheduledServerTranslationMemoryImport`) [4], confirmando que TMX/TTX/ITD son vías de entrada y que el servidor es una categoría separada.
- `TranslationUnitFormat` enumera orígenes de TU que Studio reconoce (Trados Studio, Translator's Workbench, TTX, etc.) y `TranslationUnitOrigin` el origen (TM, MT, alineación...) [3] — metadato a preservar en importaciones.

## 2. SDLTM: esquema real y semántica de actualización

### 2.1 Evidencia de esquema (instalación local, fixture pública)

Volcado del esquema de la memoria de ejemplo distribuida por la instalación (`English-German.sdltm`, 43 TUs; `PRAGMA integrity_check = ok`, page_size 1024, journal delete). Tablas principales:

- `translation_memories`: identidad, idiomas, `settings`, `fuzzy_indexes` (máscara 1|2|4), `tucount`, `data_version`, `text_context_match_type`, `id_context_match`, `fga_support`.
- `translation_units`: `source_segment`/`target_segment` (XML serializado), `source_hash`/`target_hash`, `source_token_data`/`target_token_data` (BLOB binario propietario), `alignment_data`, `tokenization_sig_hash`, campos de sistema (`creation/change/last_used` fecha+usuario, `usage_counter`), `flags`, `guid`.
- Índices fuzzy: `fuzzy_index1/2/4` (features n-grama como cadenas `hash|hash|...` con `length`) y `fuzzy_data` (columnas `fi1/fi2/fi4/fi8`).
- Contexto: `translation_unit_contexts` (`left_source_context`, `left_target_context`), `translation_unit_idcontexts` (contexto por ID, p. ej. SID).
- Otros: `translation_unit_fragments` (fragment_hash), `trans_model`/`trans_model_rev`, `vocab_src`/`vocab_trg`/`vocabfilter`, sistema de campos (`attributes`, `string/date/numeric/picklist_attributes`, `picklist_values`), `resources`/`tm_resources`, `parameters` (`VERSION`, `VERSION_CREATED`, `TokenDataVersion`, `AlignmentDataVersion`, `FREQUENCYTOP`, `LAST_ANALYZE`).

Ejemplo real de TU con tag (fila 5 de la fixture):

```xml
<Segment xmlns:xsi=... ><Elements>
  <Text><Value>This conference presents the new </Value></Text>
  <Tag><Type>Start</Type><Anchor>1</Anchor><AlignmentAnchor>1</AlignmentAnchor><TagID>1</TagID><CanHide>false</CanHide></Tag>
  <Text><Value>education programme</Value></Text>
  <Tag><Type>End</Type><Anchor>1</Anchor><AlignmentAnchor>0</AlignmentAnchor><TagID>1</TagID><CanHide>false</CanHide></Tag>
  <Text><Value> unveiled by the Minister for Education last year.</Value></Text>
</Elements><CultureName>en-US</CultureName></Segment>
```

El motor instalado actualmente (`Sdl.LanguagePlatform.TranslationMemoryImpl.dll` 1.7.4002.0) contiene embebidas sentencias DDL con el **mismo** vocabulario (`translation_units`, `fuzzy_index1/2/4`, `source_token_data`, `tokenization_sig_hash`, `translation_unit_fragments`, `vocab_src`...) y además un esquema evolucionado: columna `confirmationlevel SMALLINT`, `relaxed_hash BIGINT`, `tm_id` en todas las tablas, tablas `attributes_v2_%%`, `translation_unit_alignment_data` y `ff` (frecuencia de features), e índices con `INCLUDE`/parciales. Es decir, **el layout interno tiene versiones (`data_version`) y el motor moderno crea estructuras más ricas que la muestra antigua** (cuya `VERSION_CREATED` es 8.10).

### 2.2 Semántica oficial de actualización (API pública)

De la documentación oficial del SDK [2][3] (énfasis nuestros):

- Al confirmar un segmento nuevo se añade una TU nueva; la edición de un fuzzy añade TU; **la edición de un exact «will overwrite the original translation»**; el comando de traducción alternativa crea «two TUs for the same source segment... side-by-side», penalizadas por defecto al 99 % para obligar a elegir [2].
- Programa: `FileBasedTranslationMemory tm = new(tmPath)`; `tm.LanguageDirection.AddTranslationUnit(tu, importSettings)`; `tm.Save()`; duplicados exactos no se añaden; edición = búsqueda → modificar `TargetSegment` → `SystemFields.UseCount++` → `Save()`; borrado por `PersistentObjectToken` (`DeleteTranslationUnit(s)`, `DeleteAllTranslationUnits` «cannot be undone») [3].
- `ImportSettings` gobierna la fidelidad: `CheckMatchingSublanguages`, `ExistingFieldsUpdateMode` (`Merge`/`Overwrite`/`LeaveUnchanged`), `ExistingTUsUpdateMode.Overwrite`, `ConfirmationLevels` (filtro), `PlainText` (despoja tags), `TagCountLimit`, `IncrementUsageCount` [3][10].
- TU con metadatos: `FieldValues` (picklist/string/date/numeric), `ConfirmationLevel`, `TranslationUnitFormat`, `TranslationUnitOrigin`, `StructureContexts` («H» titular, «FN» nota al pie...) [3].
- Actualizar TUs existentes en lote: `UpdateTranslationUnits` / `AddTranslationUnitsMasked` (documentación XML del ensamblado instalado) [4].
- Mantenimiento: `TranslationMemoryUpgradeUtil` con `TranslationMemoryRequiresUpgrade/Reindex/Alignment/ModelRebuild` y `ScheduledReindexOperation` [4] — evidencia de que el formato interno necesita reindexaciones/reconstrucciones cuando cambia el motor, y de que RWS trata esas operaciones como parte del ciclo de vida del archivo.

### 2.3 Por qué «es SQLite» no basta para escribir

Un escritor fiel tendría que reproducir, por cada TU y por cada `data_version` soportado:

1. La serialización XML exacta de `Sdl.LanguagePlatform.Core.Segment` (incluyendo variantes de `Tag`: pares Start/End, standalone, `TextPlaceholder`, `LockedContent`, `AlignmentAnchor`).
2. `source_hash`/`target_hash` y `relaxed_hash` (algoritmos no públicos).
3. `source_token_data`/`target_token_data`: BLOB binario propietario, versionado por `TokenDataVersion` y ligado a `tokenization_sig_hash` (depende del idioma y de los recursos lingüísticos del TM).
4. Los índices fuzzy `fuzzy_index1/2/4`+`fuzzy_data`/`ff` (features n-grama derivadas de la tokenización).
5. Contextos (`left_source_context`/`left_target_context`), `translation_unit_fragments`, `vocab_*`, `tucount`, y las invariantes de `translation_memories`.

Modificar solo el texto de `translation_units` deja los índices inconsistentes: búsquedas fuzzy/exact y context match devolverían resultados viejos o erróneos, y Trados puede exigir un reindex/upgrade posterior. Esto coincide con la práctica observada: **ni siquiera el código comunitario oficial de RWS escribe TUs por SQL**; «SDLTM Repair» se limita a `sqlite3 .dump` → reimportar a un archivo nuevo (repara la estructura física del archivo, no regenera la semántica) [12], y todas las herramientas de import/export/anonimización usan la API [5][6].

## 3. TMX 1.4b y el dialecto SDL

### 3.1 Lo que exige la especificación (fuente primaria)

Especificación TMX 1.4b (LISA/OSCAR, relicenciada CC BY 3.0; texto archivado porque gala-global.org reestructuró su web) [13]:

- Contenedor: `tmx/header/body`; `header` exige `creationtool`, `creationtoolversion`, `segtype`, `o-tmf`, `adminlang`, `srclang`, `datatype`; `tu` admite `tuid`, `usagecount`, `lastusagedate`, `creationdate/creationid`, `changedate/changeid`, etc.; `tuv` exige `xml:lang`; codificación UTF-8/UTF-16/ASCII con solo las 5 entidades predefinidas.
- Niveles: **Level 1** (solo texto) y **Level 2** (content markup) — solo Level 2 permite reconstruir el documento traducido sin pérdida de códigos.
- Inline: `bpt`/`ept` (pares, atributo `i`), `it` (aislado, `pos` begin/end), `ph` (standalone, `assoc`), `hi`, `sub`; `ut` deprecado. El atributo `x` empareja códigos entre `tuv`s aunque el orden difiera.
- Metadatos arbitrarios por herramienta: `<prop type="...">`; la spec dice que tipos no publicados «should begin with the prefix `x-`» — el gancho normativo del dialecto SDL.

### 3.2 Dialecto SDL documentado (importador/exportador oficial)

Del ejemplo oficial del importador [10], una TU TMX producida/consumida por Studio:

```xml
<tu creationdate="20100507T161524Z" creationid="Ziad" changedate="..." changeid="..." lastusagedate="..." usagecount="2">
  <prop type="x-Origin">TM</prop>
  <prop type="x-ConfirmationLevel">ApprovedTranslation</prop>
  <prop type="x-Customer:MultiplePicklist">Microsoft</prop>
  <prop type="x-Context">0, 0</prop>
  <tuv xml:lang="en-US"><seg>A dialog box will open.</seg></tuv>
  <tuv xml:lang="de-DE"><seg>Es öffnet sich ein Dialogfenster.</seg></tuv>
</tu>
```

- Atributos de sistema en `tu` ↔ columnas de `translation_units` (fechas ISO 8601 `YYYYMMDDThhmmssZ`, `usagecount` ↔ `usage_counter`) [10][13].
- Props `x-*` ↔ TU: `x-Origin` (↔ `TranslationUnitOrigin`), `x-ConfirmationLevel` (↔ `ConfirmationLevel`), `x-<Campo>:<Tipo>` (↔ `FieldValues`; p. ej. `MultiplePicklist`), `x-Context` (vector de contexto previo) [3][10].
- Contexto estructural: `StructureContexts` («H», «FN»...) se guarda como atributo/campo del sistema (en el esquema SDLTM aparece el atributo `StructureContext`) [3]; el código comunitario oficial trata además la clave `sdl:sid` como contexto de emparejamiento por ID (SID) en ficheros bilingües [5].
- Filtros de import relevantes para «actualización fiel»: `ExistingTUsUpdateMode.Overwrite` vs dejar intactas, `ExistingFieldsUpdateMode.Merge`, `ConfirmationLevels` permitidos, `InvalidTranslationUnitsExportPath` para TUs inválidas [10].

### 3.3 Correspondencia inline Segment↔TMX (evidencia y estado)

- Mapeo observado (fixture local + convertidor abierto `maxprograms/sdltm`, EPL-1.0 [14]): `Tag Start` → `<bpt i="Anchor" x="AlignmentAnchor" type="TagID">`, `Tag End` → `<ept i="Anchor">`, standalone → `<ph x type>`. La salida exacta del *exporter oficial* de Studio no está documentada públicamente al nivel de atributos; debe fijarse con fixtures exportadas por Trados (ver §7).
- Estado de LumenCAT al iniciar la investigación: `import_tmx` rechazaba códigos inline y limitaba segmentos a 4 MiB. `TmUnit` ya conservaba atributos de sistema, propiedades `x-*` y otras variantes dentro de `raw_xml`, aunque no como campos tipados. La cabecera original no se almacenaba; la exportación generaba otra. El corte posterior [TMX con códigos](TMX_INLINE_20261005.md) amplía importación/exportación sin aplanar esos códigos. No debe afirmarse que se perdían los metadatos TU ya preservados.

## 4. Legados (TMW/MDB/TXT) y memorias de servidor

- El upgrade de legados está fuera del producto base desde Studio 2021: requiere el *Trados Compatibility and Migration Power Pack* (AppStore oficial) que añade el wizard y la apertura de formatos SDLX/ITD/TTX/Workbench [7]. Históricamente, abrir `.tmw` invoca componentes COM del Workbench clásico (errores documentados `COM class factory failed 80040154` cuando no están instalados) [15].
- **Decisión recomendada:** LumenCAT declara estos formatos fuera de alcance; quien necesite reutilizarlos debe exportar TMX desde el entorno legado o convertirlo con Trados+Power Pack y luego importar el TMX/SDLTM resultante en LumenCAT. No intentar parsear `.tmw`/`.mdb` (formatos cerrados, sin documentación pública vigente).
- Servidor: `ServerBasedTranslationMemory`/`TranslationProviderServer` con autenticación y `DatabaseServer` SQL Server [1][4][9]. La paridad no exige replicarlo: es infraestructura de empresa con licencias propias. Tratarlo como «conectividad futura opcional», nunca como dependencia silenciosa.

## 5. Bibliotecas y código abierto (con licencias verificadas)

| Recurso | Qué hace | Licencia | Valor para LumenCAT |
|---|---|---|---|
| [RWS/Sdl-Community](https://github.com/RWS/Sdl-Community) (monorepo de plugins oficiales-comunitarios) | `SDLTM Import Plus` (TMX→TM vía API), `TuToTm` (`AddTranslationUnit`+`Save`), `SDLTM Repair` (dump/rebuild SQL), `Trados Translation Memory Management Utility`, `Trados Batch Anonymizer`, filetype TMX | **Apache-2.0** [5][6][12] | Patrón de referencia exacto de cómo se escribe/lee TM «oficialmente» fuera de RWS; código reutilizable (attribución) para el helper |
| RWS/Sdl-studio-powershell-toolkit | CLI PowerShell sobre la API pública | Apache-2.0 [16] | Demuestra que la automatización por API es práctica y soportada |
| RWS/trados-studio-vs-extension (antiguo «SDK») | Plantillas VS para desarrollo de plugins | **MIT** [17] | El «SDK» como plantillas es abierto; los ensamblados `Sdl.*` en runtime NO lo son |
| maxprograms-com/sdltm | Lector TypeScript SDLTM→TMX 1.4b (usa `node:sqlite`) | **EPL-1.0** [14] | Referencia madura de lectura del esquema; EPL es copyleft débil (dinámico ok, estático obliga a abrir) |
| GregoryVigoTorres/sdltm2tmx; TomasoAlbinoni/Trados-Studio-Resource-Converter; briacp/SDLPPXPackager | Conversores SDLTM→TMX (Python/Java) | GPL-3.0 / LGPL-3.0 [18] | **No reutilizar código** (contamina); solo como contraste de ingeniería inversa |
| paulfilkin/SQLite-Shorts | SQL de solo lectura sobre SDLTM (extrae campos de sistema) | Unlicense [19] | Corrobora lectura por SQL y el formato del XML de segmento |
| RWS/sdltm_browser | Fork de DB Browser for SQLite | NOASSERTION (fork DB4S) | RWS mismo usa/proporciona un navegador SQLite para SDLTM |

**Hallazgo de la búsqueda:** no se identificó una biblioteca abierta madura que escriba SDLTM fielmente; los escritores consultados utilizan la API de Studio. Esto no demuestra inexistencia universal. Implementar un escritor directo requeriría verificar las invariantes y versiones del esquema; modificar únicamente el texto no las mantiene.

## 6. Escritura nativa portable vs. helper SDK dependiente de licencia

- **Nativa portable (Rust, offline):** lectura SDLTM (esquema verificado) + import/export TMX Level 1/2 con dialecto SDL documentado. Sin dependencia de Trados en runtime. Es la vía principal para la meta «nativa/offline portable, sin dependencia comercial silenciosa».
- **Helper oficial (API pública de Studio):** la documentación de RWS establece que para desarrollar contra la API basta referenciar los ensamblados que acompañan a la aplicación instalada (p. ej. `C:\Program Files\Trados\Trados Studio\Studio19`), con el SDK como extensión VS pública [1]. Pero el *runtime* es el Trados Studio instalado y licenciado del usuario; los binarios `Sdl.*` no son redistribuibles bajo licencia abierta (copyright RWS; el Apache-2.0/MIT de arriba cubre solo código comunitario/plantillas). Por tanto:
  - El helper solo puede ser **opcional, detectable y de acción explícita** (p. ej. «Actualizar esta memoria SDLTM con Trados instalado»), nunca un requisito silencioso ni parte del núcleo.
  - Si el coordinador decide construirlo, el camino de menor riesgo y coste es un ejecutable .NET aparte (x64, .NET Framework 4.8) siguiendo `TuToTm`/`SDLTM Import Plus` (Apache-2.0, con atribución), invocado bajo el control de la cola de la app (`output/verification/.app-lock.json` para pruebas E2E, según AGENTS.md).
  - El helper debe respetar la misma política de backups/sin-overwrite que el núcleo.

## 7. Camino mínimo recomendado (sin compatibilidad falsa)

**Fase A — Lectura SDLTM (nativa, portable):**

1. Abrir siempre en `mode=ro` o sobre copia; nunca escribir. Rechazar archivos con esquema desconocido (`SELECT sql FROM sqlite_master` fingerprint) antes de leer.
2. Extraer: TUs (texto visible + tags serializados + campos de sistema + atributos si existen), idiomas del TM, `tucount` para reconciliar conteos.
3. Convertir el XML `Segment` al modelo interno de LumenCAT con el mapeo Start/End→par, standalone→atómico; TU con variantes de `Tag` que no sepan modelarse → importar como texto plano **con diagnóstico explícito** (nunca silencio, nunca «compatibles»).
4. Backups atómicos: importar nunca modifica el original; la importación cae en la colección dentro del `.lcat` (comportamiento actual).

**Fase B — TMX fiel (Level 1 → Level 2):**

1. Import: parsear atributos de sistema de `tu` y props `x-*` del dialecto SDL (mapear a metadatos de procedencia en LumenCAT; conservar en `raw_xml` lo no mapeable). Rechazar TU inline hasta que exista el modelo de códigos de F04 [BRECHAS]; luego validar `bpt/ept` balanceados por `i` y emparejamiento `x`.
2. Export: emitir header conforme (`creationtool=LumenCAT`, `o-tmf`, `srclang` real del par, `segtype`, `datatype`), atributos de sistema desde metadatos propios y props `x-*` solo para lo que se pueda garantizar ida y vuelta.
3. Regla anti-overwrite: exportar siempre a archivo nuevo; «actualizar» una memoria Trados = entregar TMX y que la importe quien corresponda (con `ExistingTUsUpdateMode` explícito del lado Trados).

**Fase C — (Opcional, explícita) helper de actualización SDLTM vía API de Studio**, solo si el usuario final lo autoriza por operación: helper aislado, x64, contra la instalación detectada, operando sobre una **copia** del `.sdltm` con reemplazo atómico (escribir a `*.sdltm.new`, `fsync`, renombrar con backup previo del original junto al destino); nunca `DeleteAllTranslationUnits`; import settings conservadores (`ExistingFieldsUpdateMode.Merge`, sublanguages estricto) [3][10].

**Lo que NO hacer:** escritura SQL de TUs/índices en SDLTM; anunciar soporte TMW/MDB/servidor; exportar TU con tags como texto escapado fingiendo TMX inline; abrir en escritura memorias del usuario sin autorización explícita de la operación concreta.

## 8. Riesgos y mitigaciones

| Riesgo | Evidencia | Mitigación |
|---|---|---|
| Corrupción/invalidación de índices fuzzy al escribir SDLTM directo | §2.1-2.3: blobs token versionados, n-gramas, `tucount`, `data_version` | No escribir SDLTM nunca desde LumenCAT; solo lectura o helper API |
| Deriva de esquema entre versiones de Studio (muestra v1 vs DDL actual con `confirmationlevel`/`relaxed_hash`/tablas v2) | Comparación fixture vs cadenas embebidas en `TranslationMemoryImpl.dll` 1.7.4002.0 | Fingerprint de esquema + rechazo conservador; pruebas con TMs creadas por la versión instalada (19.0.0.3043) y una antigua |
| Pérdida de inline/metadata en TMX (compatibilidad falsa) | Estado actual de `formats.rs` (rechaza inline; export texto plano) [§3.3] | Matriz declarada por operación; inline detrás de F04; conservar `raw_xml`/props no mapeadas |
| Diferencias de hashing de contexto entre versiones («legacy segment hashing... consume legacy context information in TMX without conversion», doc. del ensamblado) | [4] | No prometer context match 101%/CM desde import; tratar contexto como metadato hasta validar con fixtures |
| Dependencia comercial silenciosa de Trados | §6 | Núcleo 100 % nativo; helper opcional/explícito y documentado como tal |
| Overwrite accidental de memorias del usuario | Requisito del coordinador | Todo flujo opera sobre copias; reemplazo atómico solo con autorización explícita y backup previo; E2E con instancias desechables bajo `output/verification/` |
| Licencias contaminantes (GPL) al reutilizar conversores | §5 | Reutilizar solo Apache-2.0/MIT con atribución; EPL solo enlazado dinámico o reescritura limpia |

## 9. Verificación contra Trados real (fixtures sintéticas)

Plan para el coordinador (usa el controlador de cola de la app según AGENTS.md; nunca el EXE directamente, sin tomar foreground):

1. **Fixture SDLTM moderna:** crear con el Studio 19 instalado una TM en-US→es-ES desechable bajo `output/verification/`, añadir TUs (texto, tags, campo picklist) vía la GUI encolada. Volcar esquema y comparar con `output/verification/sdltm-inspect/English-German.sdltm` (esperado: columnas nuevas del DDL embebido, `data_version` actual). Registra el fingerprint que el lector de Fase A debe aceptar.
2. **Round-trip TMX:** exportar esa TM a TMX desde Trados; importar en LumenCAT; re-exportar; reimportar en una TM limpia de Trados. Reconciliar conteos (TotalRead/TotalImported/Added/Discarded/Merged/Errors según `ImportStatistics` [10]) y comparar texto plano TU a TU. Con tags: validar que `bpt/ept` sobreviven con `i`/`x` coherentes (tras F04).
3. **Metadatos:** verificar que props `x-Origin`, `x-ConfirmationLevel`, `x-Context` y `usagecount`/fechas sobreviven Trados→LumenCAT→Trados sin degradarse (comparar `<prop>` sets antes/después).
4. **Actualización explícita (Fase C, si se aprueba):** copiar la TM, ejecutar el helper, verificar en Trados que la TM sigue abriendo, que el fuzzy/exact lookup devuelve las TUs nuevas (no solo la tabla), y que `integrity_check` pasa; conservar backup del original hasta validación.
5. **Límites declarados:** confirmar que TU con tags se rechaza con diagnóstico (hasta F04) y que ni TMW/MDB ni TM de servidor aparecen como formatos admitidos en la UI.

## 10. Límites de esta investigación

- No se ejecutó Trados ni su GUI (restricción del encargo); todo lo dicho sobre su comportamiento en runtime proviene de documentación oficial, ensamblados instalados y fixtures públicas distribuidas por la instalación.
- La muestra SDLTM inspectada fue creada por un motor antiguo (`VERSION_CREATED 8.10`, `insert_date` 2018) aunque se distribuye con Studio 19; el esquema vigente de creación nueva se infiere del DDL embebido en el DLL y debe confirmarse con la fixture del §9.1.
- `docs.rws.com` carga por JavaScript; su contenido se citó desde render Wayback (captura 2024-11-15) de la página oficial. Las afirmaciones sobre «Trados Studio 2026 Release» se limitan a lo que dicen las fuentes citadas [1]; no se extrapola nada más de 2026.
- La correspondencia exacta de atributos del *exporter* TMX oficial no está publicada; el mapeo de §3.3 combina la fixture local y un convertidor abierto, y queda pendiente de fijar con la verificación §9.2.
- La búsqueda integrada de web devolvió resultados vacíos; se usaron DuckDuckGo HTML, GitHub API y Wayback Machine como vías de acceso a las mismas fuentes primarias.

## 11. Registro de evidencia local

- `output/verification/sdltm-inspect/English-German.sdltm` (copia de `C:\Program Files\Trados\Trados Studio\Studio19\Samples\Projects\SampleProject\TMs\English-German.sdltm`): esquema y filas usadas en §2.1 y §3.3.
- `output/verification/sdltm-inspect/English-French.sdltm`, `English-Japanese.sdltm`: copias vacías (0 TUs) — útiles como fixtures de TM sin TUs.
- Instalación (solo lectura): `Sdl.LanguagePlatform.TranslationMemoryApi.xml` (inventario de clases y notas citadas), `Sdl.LanguagePlatform.TranslationMemoryImpl.dll` (DDL embebido), versión 19.0.0.3043.

## Referencias

1. RWS, *Setting up a Developer Machine* (Studio API docs): <https://developers.rws.com/studio-api-docs/articles/gettingstarted/setting_up_a_developer_machine.html> — ensamblados junto a la app instalada (ej. `C:\Program Files\Trados\Trados Studio\Studio19`), SDK como extensión VS pública, x64/.NET 4.8, GroupShare/TM Server como producto de servidor.
2. RWS, *Updating Translation Memories* (16.1): <https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/updating_translation_memories.html> — escenarios de update al traducir; overwrite del exact editado; TU alternativa con penalización 99 %.
3. RWS, *Updating a Translation Memory* (16.1): <https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/updating_a_translation_memory.html> — `AddTranslationUnit`+`ImportSettings`, `EditTu` (UseCount), `DeleteTranslationUnit(s)`, `StructureContexts`, `TranslationUnitFormat/Origin`, duplicados no añadidos.
4. `Sdl.LanguagePlatform.TranslationMemoryApi.xml` (doc XML del ensamblado instalado, 19.0.0.3043): `UpdateTranslationUnits`, `AddTranslationUnitsMasked`, `ScheduledServerTranslationMemoryImport` («TMX, SDLIFF, ITD or TTX ... server-based»), `TranslationMemoryUpgradeUtil.*`, `ScheduledReindexOperation`, «legacy segment hashing ... legacy context information in TMX», export GZip.
5. RWS/Sdl-Community, `SDLTM Import Plus/FileService/TuConverter.cs` (claves `sdl:sid`, conversión vía Sdl.LanguagePlatform): <https://github.com/RWS/Sdl-Community/tree/master/SDLTM%20Import%20Plus>
6. RWS/Sdl-Community, `TuToTm/TuToTm/Helpers/TmHelper.cs` (`AddTranslationUnit` + `Save`): <https://github.com/RWS/Sdl-Community/tree/master/TuToTm>
7. RWS, *Upgrading legacy file-based TMs from Upgrade wizard* (Trados Studio 2024, render Wayback 2024-11-15): <https://docs.rws.com/en-US/trados-studio-2024-1145319/upgrading-legacy-file-based-tms-from-upgrade-wizard-534268> — legados/TMX/TXT→SDLTM; desde 2021 requiere *Trados Compatibility and Migration Power Pack* (<https://appstore.rws.com/plugin/102>); TM Optimizer.
8. RWS, *Translation Memory API overview*: <https://developers.rws.com/studio-api-docs/apiconcepts/overview.html>
9. Inventario de clases del ensamblado instalado (§registro): `FileBasedTranslationMemory(LanguageDirection)`, `ServerBasedTranslationMemory...`, `TranslationMemoryImporter/Exporter`, `InMemoryTranslationMemory`, `TranslationMemoryUpgradeUtil`, `LicensingStatusInformation`.
10. RWS, *Importing a TMX File* (16.1): <https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/importing_a_tmx_file.html> — `TranslationMemoryImporter`, chunking, settings completos, ejemplo TU con props `x-*` y `x-Context`, `ImportStatistics`, archivo de inválidas.
11. RWS, *Exporting to a TMX File* (16.1): <https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/exporting_to_a_tmx_file.html> — `TranslationMemoryExporter`, filtros `FilterExpression`/`AtomicExpression`.
12. RWS/Sdl-Community, `SDLTM Repair/SDLTMRepair/RepairForm.cs` y `3rd party/sqlite3.exe` (dump/reimport SQL a `repaired_*.sdltm`): <https://github.com/RWS/Sdl-Community/tree/master/SDLTM%20Repair>
13. LISA/OSCAR→GALA, *TMX 1.4b Specification* (26-abr-2005, CC BY 3.0), texto archivado: <https://web.archive.org/web/20120511032753/http://www.gala-global.org/oscarStandards/tmx/tmx14b.html> — niveles 1/2, inline `bpt/ept/it/ph/hi/sub`, atributos (`i`, `x`, `pos`, `assoc`, `type`), props `x-`, ISO 8601.
14. maxprograms-com/sdltm (TypeScript, EPL-1.0): <https://github.com/maxprograms-com/sdltm> — lector SDLTM→TMX 1.4b; mapeo Start→`bpt i x type`, End→`ept i`, standalone→`ph x type` en `ts/TMReader.ts`.
15. RWS Knowledge Base (histórico), *Error ... legacy Trados or SDLX translation memory ... COM class factory failed (80040154)*: <https://gateway.sdl.com/articles/en_US/SolutionArticles/000002615?articleName=000005172>
16. RWS/Sdl-studio-powershell-toolkit (Apache-2.0): <https://github.com/RWS/Sdl-studio-powershell-toolkit>
17. RWS/trados-studio-vs-extension (MIT): <https://github.com/RWS/trados-studio-vs-extension>
18. Búsquedas GitHub API: `GregoryVigoTorres/sdltm2tmx` (GPL-3.0), `TomasoAlbinoni/Trados-Studio-Resource-Converter` (LGPL-3.0), `briacp/SDLPPXPackager` (GPL-3.0).
19. paulfilkin/SQLite-Shorts (Unlicense), `systemFields/systemFields.sql`: <https://github.com/paulfilkin/SQLite-Shorts>
