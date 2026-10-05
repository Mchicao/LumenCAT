# Motor documental vía Trados Studio instalado: API standalone probada

**Corte: 5 de octubre de 2026. Estado: ejecutado y verificado por API, sin GUI.**

Investigación con ejecución real sobre la instalación `C:\Program Files\Trados\Trados Studio\Studio19` (`19.0.0.3043`), sin redistribuir DLL, sin abrir la aplicación, sin tocar proyectos personales. Todo corrió bajo `powershell.exe` 5.1 (.NET Framework 4.8) con fixtures sintéticos propios. Evidencia y scripts: [`output/verification/document-sdk-probe/`](../../output/verification/document-sdk-probe/).

## Resumen ejecutivo

| Afirmación | Estado |
| --- | --- |
| Cargar el framework FileTypeSupport standalone con todos los filtros instalados | **Probado**: `DefaultFileTypeManager.CreateInstance(true)` → 51 definiciones de filtro. |
| Extraer nativo → SDLXLIFF con `GetConverterToDefaultBilingual` | **Probado** en TXT, DOCX y PDF (sintéticos). |
| Enumerar/editar segmentos Source/Target con `IBilingualContentProcessor` | **Probado**: promoción párrafo→segmento, escritura de target, `ConfirmationLevel=Translated`. |
| Regenerar nativo desde SDLXLIFF con `GetConverterToNative` | **Probado** en TXT y DOCX, **sin el archivo original en disco**. |
| SDLXLIFF autosuficiente (skeleton embebido) | **Probado** en DOCX (binario original embebido base64) y TXT (tag-defs inline). |
| PDF | **Extracción probada**: convierte PDF→DOCX **in-process** (sin lanzar Word) y embebe el DOCX como skeleton. |
| Catálogo de formatos por manifiesto | **No prometido ni probado**; 51 definiciones ≠ 51 formatos (ver límites). |

**Camino mínimo viable confirmado**: dependencia opcional explícita «Trados Studio instalado», puente `powershell.exe` 5.1 por proceso hijo (patrón del puente SDLTM existente), `DefaultFileTypeManager` + bundle JSON + conversores del `IFileTypeManager`.

## 1. Hechos de API confirmados contra los binarios instalados

Fuente primaria de firmas: doc XML de la instalación (`Sdl.FileTypeSupport.Framework.Core.XML`, 798 KB) más reflexión en ejecución. Versiones de archivo relevantes: Framework.Core/Implementation/Utilities `4.0.0.0`, Sdl.Core.Settings/Globalization `5.1.1.0`, filtros Office/PDF `6.0.1.0`.

### 1.1 Punto de entrada: `FileTypeManagerFactory` NO existe

- La clase `FileTypeManagerFactory` que aparece en material público del SDK **no está presente en ningún ensamblado de la instalación** (escaneo de tipos sobre todas las DLL del directorio, evidencia `probe1b_manager.txt`).
- El punto de entrada standalone real es **`Sdl.FileTypeSupport.Framework.Core.Utilities.IntegrationApi.DefaultFileTypeManager`** (en `Sdl.FileTypeSupport.Framework.Core.Utilities.dll`):

```text
static IFileTypeManager CreateInstance()                    // no probado
static IFileTypeManager CreateInstance(bool autoLoadFileTypes)  // probado: true
```

`CreateInstance(true)` devuelve `Sdl.FileTypeSupport.Framework.Integration.PocoFilterManager` (de `Framework.Implementation.dll`) con **51 `IFileTypeDefinition` cargadas automáticamente** y `DefaultBilingualFileTypeDefinition` = «XLIFF de SDL» (`SDL XLIFF 1.0 v 1.0.0.0`). El constructor por `New-Object` falla: es interno.

### 1.2 Miembros clave de `IFileTypeManager` (confirmados por reflexión + uso)

```text
IFileTypeDefinition[] FileTypeDefinitions          // 51 con autoload
IFileTypeDefinition  DefaultBilingualFileTypeDefinition
ISettingsBundle      SettingsBundle                // get+set: ASIGNAR SIEMPRE (ver 1.5)
FileTypeDefinitionId → FindFileTypeDefinition(id)

// Detección (devuelve Pair<IFileTypeDefinition, SniffInfo>, NO la definición sola):
Pair<IFileTypeDefinition,SniffInfo> GetBestMatchingFileTypeDefinition(string file, EventHandler<MessageEventArgs>)
Pair<IFileTypeDefinition,SniffInfo> GetBestMatchingFileTypeDefinition(string file, Language, Codepage, EventHandler<MessageEventArgs>)
IList<Pair<...>> GetAllMatchingFileTypeDefinitions(string file, EventHandler<MessageEventArgs>)

// Conversión (devuelven IMultiFileConverter):
IMultiFileConverter GetConverterToDefaultBilingual(string nativeInput, string sdlxliffOutput, EventHandler<MessageEventArgs>)
IMultiFileConverter GetConverterToNative(string sdlxliffInput, OutputPropertiesProvider, EventHandler<MessageEventArgs>)
// Sobrecargas avanzadas: GetConverterToBilingual(...IBilingualDocumentGenerator...), GetConverter(...), BuildExtractor/BuildNativeGenerator...
```

### 1.3 `IMultiFileConverter` (lo realmente usado)

```text
void Parse()                       // ejecuta el pipeline completo en ambas direcciones
bool ParseNext()
void AddBilingualProcessor(IBilingualContentProcessor)   // heredado de IBilingualProcessorContainer
IDocumentItemFactory ItemFactory   // fábrica de segmentos/props
IDocumentProperties DocumentInfo   // SourceLanguage / TargetLanguage (Sdl.Core.Globalization.Language)
void SetDocumentInfo(IDocumentProperties, bool applyToAllExtractors)
OutputPropertiesProvider OutputPropertiesProvider         // delegado para fijar salida nativa
DependencyFileLocator DependencyFileLocator               // resolver dependencias externas si hiciera falta
```

### 1.4 Idiomas: NO hay `LanguagePair` en este API

- No existe `LanguagePair` en los ensamblados FileTypeSupport ni en ProjectAutomation.Core/Implementation (solo un modelo servidor ajeno en `Sdl.ProjectApi.Server.Model.ProjectTemplates`). En este pipeline los idiomas son:
  - `Sdl.Core.Globalization.Language` con ctor desde `CultureInfo` (marcado obsoleto a favor de `GetLanguageAsync`; funciona con pragma), o `Language.ToLanguage(LanguageBase)`.
  - `CultureInfo` + `Sdl.Core.Globalization.Codepage` como parámetros de las sobrecargas multi-archivo.
- Probado: crear `IDocumentProperties` vía `converter.ItemFactory.CreateDocumentProperties()`, asignar `SourceLanguage`/`TargetLanguage` y `converter.SetDocumentInfo(docProps, $true)` antes de `Parse()` → el SDLXLIFF queda con `source-language="en-US" target-language="es-ES"`.
- Sin asignarlo, el SDLXLIFF sale con `source-language="en"` (inferido por sniffer) y **sin target-language**.

### 1.5 `SettingsBundle`: requisito obligatorio para filtros Office

- Síntoma si no se asigna: el filtro DOCX revienta con `NullReferenceException` en `Sdl.FileTypeSupport.Filters.MicrosoftOffice.Word.Parser.DocxParser.SetFileProperties` (primer error real del estudio; evidencia `probe6b_docx_min.txt`).
- Causa y solución probadas: asignar al manager un bundle **no nulo** antes de crear conversores:

```powershell
$settingsAsm = [AppDomain]::CurrentDomain.GetAssemblies() | Where-Object { $_.GetName().Name -eq 'Sdl.Core.Settings' } | Select-Object -First 1
$bundle = [Activator]::CreateInstance($settingsAsm.GetType('Sdl.Core.Settings.Implementation.Json.JsonSettingsBundle'))
$manager.GetType().GetProperty('SettingsBundle').SetValue($manager, $bundle, $null)
```

- Trampa documentada: `Sdl.Core.Settings.Implementation.Xml.SettingsBundle` es **interna** y **NO implementa `ISettingsBundle`** (es helper de serialización `IXmlSerializable`). La implementación pública correcta es **`Sdl.Core.Settings.Implementation.Json.JsonSettingsBundle`** (ctor público sin parámetros).
- Con el bundle vacío los filtros usan sus defaults (TXT, DOCX y PDF funcionaron). Para ajustes específicos habría que poblar `ISettingsGroup` por filtro: **no probado aquí**.

### 1.6 Modelo bilingüe (BilingualApi)

- `AbstractBilingualContentProcessor` (Framework.Core): `SetFileProperties`, `ProcessParagraphUnit`, `FileComplete`, `Complete`, `Output`. Es la base probada para segmentar/editar/leer.
- `IParagraphUnit`: `IsStructure`, `Source`/`Target` (son `IParagraph` → `IAbstractMarkupDataContainer`), `SegmentPairs`.
- `NativeApi.ISegmentPairProperties`: `SegmentId Id`, `ConfirmationLevel ConfirmationLevel`, `ITranslationOrigin TranslationOrigin`, `bool IsLocked`. El tipo del id es **`SegmentId`** (no `SegmentPairId`; ese nombre no existe en Framework.Core — se correlaciona con `sdl:seg-defs/@id` del SDLXLIFF). Valores observados: 1, 2, 3… por párrafo.
- Contenedores: `MoveAllItemsTo(container)`, `MoveItemsTo(...)`, `Add`, `Clear`; `IAbstractMarkupData.Clone()` (cast explícito a `IAbstractMarkupData`).
- `IDocumentItemFactory`: `CreateSegmentPairProperties()`, `CreateSegment(props)`, `CreateSegmentPair(src, tgt)`, `CreateText`, `CreateTagPair`, `CreatePlaceholderTag`…
- Procesador incluido útil: `Sdl.FileTypeSupport.Filters.Processors.CopySourceToEmptyTargetProcessor` (ctor sin parámetros) copia source→target vacío; también hay renumeradores y procesadores de contenido embebido. **No** hay procesador genérico de segmentación en esa DLL (ver §2).

## 2. Defaults del SDK: párrafos, no oraciones

- **La extracción nativa produce unidades de párrafo con `SegmentPairs` VACÍO.** El SDLXLIFF inicial tiene `<source>` con texto pero sin `<sdl:seg-defs>`; un target vacío de párrafo sin segmentar **se pierde al regenerar el nativo** (evidencia `probe2_txt.txt`: TXT regenerado de 8 bytes, solo saltos de línea).
- La segmentación oracional es un paso separado que en Studio ejecuta ProjectAutomation; el motor está en `Sdl.Core.LanguageProcessing.Segmentation` (`Segmentor`, `SegmentationEngine(Factory)`, `SegmentorUtility`, recursos SRX) pero **no está empaquetado como `IBilingualContentProcessor` listo para insertar**. Queda como trabajo futuro si se quiere segmentación oracional idéntica a Studio.
- Solución probada (granularidad párrafo, misma que el TXT actual de LumenCAT): promocionar el párrafo completo a un único segment pair dentro del procesador:

```csharp
ISegmentPairProperties sp = factory.CreateSegmentPairProperties();
ISegment src = factory.CreateSegment(sp);
paragraphUnit.Source.MoveAllItemsTo(src);
ISegment tgt = factory.CreateSegment(sp);
paragraphUnit.Source.Add(src);
paragraphUnit.Target.Add(tgt);
// pair.Target.Clear(); foreach (item in pair.Source) Target.Add((IAbstractMarkupData)item.Clone());
// ConfirmationLevel = Translated  (por reflexión o binding directo)
```

Tras la promoción, `paragraphUnit.SegmentPairs` expone el par y el SDLXLIFF resultante incluye `seg-source`, `<target/>` y `sdl:seg-defs` con ids.

## 3. Pipeline mínimo probado (ida y vuelta completa)

Código esencial del que derive toda la evidencia (script completo: `probe4_pipeline_v2.ps1`):

```powershell
# 0) Registro de resolución de ensamblados EN C# (Add-Type), NUNCA handler PS (stack overflow conocido)
#    + LoadFrom de: Sdl.Core.Settings, Sdl.Core.Globalization(.Async), Sdl.Core.PluginFramework,
#      Sdl.FileTypeSupport.Framework.Core(.Settings)(.Utilities)(.Implementation)

$manager  = [DefaultFileTypeManager]::CreateInstance($true)      # vía reflexión del método estático
$manager.SettingsBundle = new JsonSettingsBundle()                # ver §1.5
$best     = $manager.GetBestMatchingFileTypeDefinition($doc, $handler)   # Pair(def, SniffInfo)

# Ida: nativo -> SDLXLIFF (segmenta, traduce)
$converter = $manager.GetConverterToDefaultBilingual($native, $out.sdlxliff, $null)
$docProps  = $converter.ItemFactory.CreateDocumentProperties()
$docProps.SourceLanguage = [Sdl.Core.Globalization.Language]::new([Globalization.CultureInfo]::GetCultureInfo('en-US'))
$docProps.TargetLanguage = [Sdl.Core.Globalization.Language]::new([Globalization.CultureInfo]::GetCultureInfo('es-ES'))
$converter.SetDocumentInfo($docProps, $true)
$converter.AddBilingualProcessor([SegmentProcessor2]::new($converter.ItemFactory, $true, ' [LCT-TR]'))
$converter.Parse()

# Vuelta: SDLXLIFF -> nativo (offline; original ausente)
$provider   = [NativeOutputProvider]::new($targetPath)   # fija INativeOutputFileProperties.OutputFilePath
$converter2 = $manager.GetConverterToNative($sdlxliff, $provider.Provider, $null)
$converter2.AddBilingualProcessor([SegmentProcessor2]::new($null, $false, $null))   # solo lectura/verificación
$converter2.Parse()
```

`OutputPropertiesProvider` es un delegado `void (INativeOutputFileProperties, IPersistentFileConversionProperties, IOutputFileInfo)`; basta con fijar `outputProperties.OutputFilePath`. `IOutputFileInfo.Filename` es solo sugerencia.

## 4. Resultados ejecutados

### TXT (filtro «Plain Text v 1.0.0.0», `Native.RegEx_1_1`) — `probe4_txt.txt`

- Sniffing: gana «Plain Text» (también reclama .txt «Tab Delimited» en catálogo).
- 3 párrafos → 3 segment pairs (`SegmentId` 1–3); líneas vacías = structure units con tags `<x/>`.
- Target con sufijo `[LCT-TR]` + `Translated`; SDLXLIFF 3422 B con idiomas en-US→es-ES.
- Regenerado offline (original movido): TXT de 209 B con las 3 líneas traducidas, **café intacto**, línea en blanco conservada.

### DOCX (filtro «Microsoft Word 2007-2019», `WordprocessingML v. 2`) — `probe4_docx.txt` + `probe9_offline_only.txt`

- Fixture DOCX mínimo sintético (985 B); 2 párrafos → 2 segment pairs; traducción con sufijo.
- El SDLXLIFF (4571 B) contiene **`<header><reference><internal-file form="base64">` con el binario DOCX embebido** (~960 B) y `<sdl:ref-files>` con `id="Docx.DependencyFileId"` y el `o-path` original.
- Regeneración con **cero copias de `sample.docx` en disco** (todas renombradas a `.hidden`): DOCX válido de 1002 B, 3 partes zip, `word/document.xml` con el texto traducido, `sectPr` y `xml:space` intactos. **El merge offline no necesita el archivo original ni su ruta.**

### PDF (filtro «PDF v 3.0.0.0», `Native.PDF`) — `probe7_diagnostics.txt` + `probe8_pdfcheck.txt`

- PDF sintético válido (614 B, texto Helvetica). Extracción OK standalone: SDLXLIFF 11,7 KB cuyo skeleton embebido base64 es **un ZIP/DOCX** («PK…»), es decir, la conversión PDF→Word ocurre **in-process, sin lanzar Microsoft Word** (la instalación incluye `Rws.FileTypeSupport.Filters.AsposeCommon.dll`, consistente con conversión gestionada). Coherente con que la entrega final del filtro PDF sea Word, no PDF (ver [PARIDAD_FORMATOS_TRADOS_FUENTES.md](PARIDAD_FORMATOS_TRADOS_FUENTES.md)).
- Texto extraído en `trans-unit` con tags de formato `<g>`; 1 unidad de párrafo, sin segmentar (esperable sin segmentador).
- **No probado**: regeneración nativa desde ese SDLXLIFF, calidad de conversión/OCR sobre PDF reales, límites del subcontenido.

## 5. Dependencias y requisitos del puente

1. **Trados Studio 19 instalado** y su registro: `HKLM\SOFTWARE\Trados\Studio19` (+ `Studio19License`). El autodescubrimiento de `CreateInstance(true)` funcionó con CWD ajeno a Studio; los 51 filtros se cargan desde la instalación. No se redistribuye ninguna DLL.
2. **.NET Framework 4.8** (los ensamblados cargaron limpio en PowerShell 5.1; no se probó .NET moderno/pwsh).
3. **Resolución de ensamblados con `AssemblyResolve` implementado en C#** (`Add-Type`), apuntando al directorio Studio — mismo patrón que `scripts/utils/trados_memory_bridge.ps1`. Un handler de `AssemblyResolve` en PowerShell puro provoca stack overflow (conocido del puente SDLTM).
4. **Cuidado con identidades de ensamblado**: instanciar tipos de `Sdl.Core.Settings` por reflexión desde el ensamblado ya cargado, no por `LoadFrom` repetido; y los tipos internal (como `Xml.SettingsBundle`) no sirven aunque `Assembly.CreateInstance` parezca crearlos.
5. **Licensing**: ninguna puerta de licencia se activó en el flujo probado (no se instanció nada de `Sdl.TranslationStudioAutomation.Licensing`). Esto no es una autorización: el uso del runtime de Studio desde otra aplicación queda sujeto a la licencia RWS del usuario; decisión legal pendiente. Recomendado: presentar la función como «requiere Trados Studio instalado y licenciado», dependencia opcional explícita.
6. **Sin GUI**: todo el pipeline es consola/reflexión; no se abrió Studio ni se tocó el foreground.

## 6. Límites y no-promesas

- **51 definiciones de filtro ≠ 51 formatos**: el catálogo cargado (ver `work/txt/filter-catalog.txt`) mezcla filtros, plantillas y perfiles; extensiones solapadas (.txt: Plain Text y Tab Delimited; .html: 4 filtros). La selección correcta es por **sniffing** (`GetBestMatchingFileTypeDefinition` con `EventHandler` real; con handler nulo o sin inspeccionar el `Pair` devuelto parece «null»), **no** por manifiesto ni por extensión. No se promete catálogo derivado de manifiestos.
- Probado en 3 filtros (Plain Text, DOCX, PDF) con fixtures mínimos. **No probado**: el resto de los 42 manifiestos, formatos binarios heredados (DOC/XLS/PPT pueden depender de componentes que exijan Office real), ajustes de filtro no por defecto, segmentación oracional, contenido embebido/subcontenido, verificadores, archivos grandes, documentos reales del usuario, aperturas en las aplicaciones destino (eso exige el flujo GUI de verificación de LumenCAT).
- `GetConverterToNative` regenera desde el SDLXLIFF; para formatos donde el skeleton sea externo (no embebido) haría falta `DependencyFileLocator` o el blob original — en lo probado (TXT/DOCX/PDF) el skeleton va **embebido** en el SDLXLIFF.
- `FileTypeManagerFactory` de la documentación pública no existe en estos binarios: cualquier diseño debe partir de `DefaultFileTypeManager.CreateInstance` (o de `Sdl.ProjectAutomation` completo, fuera de alcance).

## 7. Arquitectura recomendada para `ImportedDocument`

Objetivo pedido: blob original autosuficiente + SDLXLIFF con skeleton incrustado, merge offline sin depender del path original. Los hechos probados la sostienen así:

1. **Al importar** (con Studio disponible): ejecutar Fase A (§3) por documento, con idiomas del proyecto; conservar:
   - `original blob` (bytes nativos, por si un filtro futuro necesita dependencia externa o para re-extracción con otros ajustes),
   - `documento.sdlxliff` resultante (**contiene el skeleton/base64** en formatos no textuales; tag-defs inline en textuales),
   - metadatos: `FileTypeDefinitionId` sniffeado (p. ej. `WordprocessingML v. 2`), versión de filtro (`SDL:FileTypeDllVersion` en el header), idiomas, hash del original, ajustes (bundle JSON si se customizan).
2. **Al traducir/editar**: unidades = segment pairs del SDLXLIFF; enumeración vía `IBilingualContentProcessor` probada en ambas fases; `SegmentId` estable como clave.
3. **Al exportar/merge offline** (aunque el path original haya desaparecido): Fase B con `GetConverterToNative` + `OutputPropertiesProvider`; para DOCX se regeneró el nativo con **cero** copias del original en disco. Si un filtro futuro exigiera el original, el blob del punto 1 lo cubre (restaurándolo a un path temporal y/o `DependencyFileLocator`).
4. **Puente de proceso**: hijo `powershell.exe -File …document_bridge.ps1` con reporte JSON UTF-8 sin BOM y exit codes, réplica del contrato de `trados_memory_bridge.ps1`; timeouts y aislamiento por proceso (el estado de Studio no se toca). Implementación: fuera del alcance de este informe.

## 8. Inventario del probe y reproducción

```text
output/verification/document-sdk-probe/
  probe0_fixtures.ps1      fixtures TXT/DOCX sintéticos (regenerables)
  probe1*_*.ps1            reflexión: firmas, DefaultFileTypeManager, interfaces
  probe2_pipeline.ps1      v1 (lección: sin segmentar se pierde el texto al regenerar)
  probe3*_*.ps1            búsqueda de segmentador; contenedores/SegmentId
  probe4_pipeline_v2.ps1   PIPELINE COMPLETO probado (TXT y DOCX ida/vuelta offline)
  probe5*_*.ps1            settings: ISettingsBundle, tipos Word
  probe6*_*.ps1            diagnóstico del NRE del filtro Word (causa: bundle null)
  probe7_diagnostics.ps1   sniffing Pair, registro, extracción PDF
  probe8_pdfcheck.ps1      texto extraído del PDF; escaneo LanguagePair
  probe9_offline_only.ps1  regeneración DOCX sin ningún original en disco
  evidence/*.txt           transcripciones de cada ejecución
  work/                    sdlxliff y targets generados (artefactos de evidencia)
```

Reproducción de la ida/vuelta (≈40 s):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File probe0_fixtures.ps1 -FixtureDir fixtures
powershell.exe -NoProfile -ExecutionPolicy Bypass -File probe4_pipeline_v2.ps1 `
  -StudioDirectory 'C:\Program Files\Trados\Trados Studio\Studio19' `
  -NativeInput fixtures\sample.docx -WorkDir work\docx-v2 -EvidenceFile evidence\probe4_docx.txt
```

## 9. Fuentes

| Fuente | Uso |
| --- | --- |
| `Studio19\Sdl.FileTypeSupport.Framework.Core.XML` (doc XML local) | Firmas oficiales de `IFileTypeManager`, `GetConverterTo*`, `DefaultBilingualFileTypeDefinition`. |
| Reflexión en ejecución sobre la instalación | Firmas reales donde el doc XML no llega: `DefaultFileTypeManager`, `Pair<...,SniffInfo>`, `SegmentId`, visibilidad de tipos. |
| [File Type Support — RWS Studio API docs](https://developers.rws.com/studio-api-docs/apiconcepts/filetypesupport/overview.html) | Modelo conceptual: extracción a SDLXLIFF, regeneración del nativo, dependencia del original solo cuando el formato lo necesita. |
| Evidencia local `output/verification/document-sdk-probe/` | Todo lo afirmado como «probado» en este documento. |
| [PARIDAD_FORMATOS_TRADOS_FUENTES.md](PARIDAD_FORMATOS_TRADOS_FUENTES.md) | Catálogo 42 manifiestos, tabla Studio 2026, advertencias PDF/licencias; este informe añade la **ejecución** del SDK. |

Nada de lo anterior acredita apertura de entregas en Word/Trados/maquetación: para eso, el flujo GUI de verificación del proyecto (`output/verification/`, controlador en cola) cuando se autorice.
