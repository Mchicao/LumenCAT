# Handoff: paridad documental y de memorias con Trados

Para: agente que continúa el encargo en `C:\Proyectos\LumenCAT` (Windows, repo git, rama `main`).
De: sesión que publicó `afbadc3..2cf45f4` en GitHub `main` el 2026-10-05.

## Encargo vigente

Alcanzar paridad con Trados Studio en documentos que LumenCAT puede importar/traducir/generar y memorias que puede leer/actualizar. «IMPLEMENTA TODO LO NECESARIO». Trabajar en `main` local y publicar cortes verificados periódicamente en GitHub `main` (autorización permanente del usuario para `git push origin main`; nunca force). La aceptación es por operación: importar → editar → persistir → exportar → **consumo independiente**, no por extensión aceptada.

Mapa maestro: `docs/architecture/PARIDAD_FORMATOS.md`. Léelo primero; este handoff no lo sustituye.

## Estado actual (verificado)

Último push: `2cf45f4` en `origin/main`, working tree limpio. Cortes acumulados y su nivel de evidencia:

- TXT/XLIFF 1.2 inline/TMX inline/DOCX conservador — CORE PASS, publicados antes.
- SDLTM vía SDK (`afbadc3`) — CORE+SDK PASS (43→44 TUs, exact/fuzzy).
- XLIFF 1.2 segmentado (`2a7afb5`) — CORE+XML PASS (XSD estricto OASIS + `System.Xml` independiente). **GUI NOT RUN.**
- TMX UTF-16 LE/BE con BOM (`2cf45f4`) — CORE+SDK PASS (motor Trados recuperó unidad con `🌍`/códigos). **GUI NOT RUN.**
- Suite: 65 pruebas ordinarias + 2 SDK ignoradas (ejecutar aparte). Release nueva en `.cache/target/release/lumencat.exe` (SHA-256 `5A6D3EBE…859F`), **no** instalada en `output/lumencat.exe`.

Verificación de cada corte está en `docs/technical/<CORTE>_20261005.md` (XLIFF_SEGMENTADO, TMX_UTF16, XLIFF_INLINE, TMX_INLINE, SDLTM_SDK) y en `docs/technical/TRADOS_DOCUMENT_ENGINE_FUENTES.md` (probe del motor documental, ejecutado, no integrado).

## Mejoras propuestas, por prioridad y coste

### 1. Lectura SDLTM sin SDK (Fase A) — el mejor ratio valor/coste

Especificación ya escrita en `docs/technical/PARIDAD_MEMORIAS_TRADOS_FUENTES.md` §2.1, §7-FaseA y §3.3 (esquema real, mapeo Segment↔códigos). Puntos clave:

- Abrir SQLite `mode=ro` o sobre copia (patrón `snapshot()` en `src/formats/sdltm.rs`, ya hace backup consistente).
- Fingerprint de `sqlite_master` antes de leer; rechazar esquemas desconocidos.
- Mapear XML `Segment` (`Text`→texto, `Tag Start/End`→par con `Anchor`, standalone→atómico) al `Fragment` existente (`src/formats/inline.rs`); variantes de `Tag` no modelables (`LockedContent`, `TextPlaceholder`, `AlignmentAnchor≠0`) → rechazo explícito con diagnóstico, nunca silencio.
- Idiomas desde `translation_memories`; variantes/props del sistema a `raw_xml` como hace TMX.
- Integración en `import_memory` (`src/storage.rs`): SDK presente → puente actual; ausente → lector nativo con menos garantías (declarar la diferencia, p. ej. sin `update_sdltm`).
- Fixtures locales listas: `output/verification/sdltm-inspect/English-German.sdltm` (43 TUs con tags, creada por motor viejo 8.10). Crear además una SDLTM moderna con el Studio 19 instalado vía SDK para comparar fingerprints (riesgo documentado: DDL embebido en `TranslationMemoryImpl.dll` tiene columnas nuevas).
- Prueba de aceptación: importar la fixture sin SDK, coincidencias exact/fuzzy funcionan, reexportar TMX y reimportar.

### 2. GUI de los cortes sin recorrido — deuda de evidencia

`formats.import-xliff-segmented` y TMX UTF-16 no tienen recorrido GUI. Las recetas ya están actualizadas en `.cursor/skills/verify-lumencat/features/formats.md` y `memories.md`. Reglas duras en `.cursor/skills/verify-lumencat/SKILL.md`: controlador `scripts/utils/control_lumencat.ps1` con `-RunId` nuevo y `-WaitSeconds`, nunca lanzar el EXE solo, nunca conducir la instancia del usuario (comprobar `Get-Process lumencat` y `.app-lock.json` antes), corpus en `output/verification/<RunId>/`. Recorrido mínimo acreditable para segmentado: importar `output/verification/xliff-segmented-20261005-1/segmented.xlf` en proyecto en→es, 3 filas, editar/confirmar 2, guardar/reabrir, exportar y contrastar `mid` con consumidor XML.

### 3. Integración del motor documental vía SDK — el mayor salto de paridad

El probe ya probó la vía completa (informe `TRADOS_DOCUMENT_ENGINE_FUENTES.md`, scripts reproducibles en `output/verification/document-sdk-probe/`). Para integrar:

- Puente por proceso hijo copiando el contrato de `scripts/utils/trados_memory_bridge.ps1` (JSON UTF-8 sin BOM, exit codes, `AssemblyResolve` en C# —nunca handler PowerShell, stack overflow—).
- `DefaultFileTypeManager.CreateInstance(true)` + `JsonSettingsBundle` obligatorio; sniffing con `GetBestMatchingFileTypeDefinition` (devuelve `Pair`, no la definición); nunca selección por extensión/manifiesto.
- Idiomas vía `SetDocumentInfo` (no existe `LanguagePair` en este API).
- Promoción párrafo→segment pair probada; segmentación oracional queda fuera (motor en `Sdl.Core.LanguageProcessing`, no empaquetado como processor).
- Arquitectura `ImportedDocument`: blob original + SDLXLIFF generado (skeleton embebido en TXT/DOCX/PDF) + metadatos (filter id, idiomas, hashes) = merge offline sin el archivo original.
- Presentarlo en UI como «requiere Trados Studio instalado y licenciado», dependencia opcional explícita. Decisión legal/licencia queda del lado RWS/EULA del usuario.

### 4. Bilingües: XLIFF 2.0 y SDLXLIFF

Después de 1–3. XLIFF 2.0 cambia el modelo (`<file>/<unit>/<segment>`, estados propios); SDLXLIFF añade `sdl:seg-defs`, conf levels y skeleton — el corte 3 entrega SDLXLIFF ya generado, lo que simplifica el lector. Mantener la regla del corte segmentado: estados con granularidad que el formato no representa → conservador o rechazo explicado, nunca invención.

### 5. Memorias: `sub`, cabecera TMX original, aprendizaje con códigos

- `sub` dentro de `bpt/ept/ph/it` (XLIFF y TMX): texto traducible anidado en código nativo; requiere decisión de representación en el editor.
- Conservar la cabecera TMX importada en `raw_xml` de la colección y reemitirla al exportar (hoy se regenera).
- Aprender unidades con códigos al confirmar (hoy `LearningOutcome::UnsupportedCodes`): reutilizar `Fragment::render` del source como contexto del target aprendido.

### 6. Ampliación DOCX y Office

`src/formats/docx.rs` es deliberadamente conservador (rechaza campos, tracked changes, headers/footers, `w:tab/w:br`, SmartArt con mensaje explícito). Ampliar por historias (`header/footer/footnotes`), tablas anidadas y campos simples, manteniendo el patrón «rechazo explícito > soporte falso». XLSX/PPTX/ODF después; el motor SDK (corte 3) puede cubrirlos antes si se integra.

### 7. Deuda técnica menor (opcional, sin urgencia)

- `segmented::Unit.fragments` usa tuplas `(String, Fragment, Fragment)`; un struct con nombres mejoraría legibilidad.
- `canonical()` en `src/formats/segmented.rs` normaliza atributos elemento a elemento; no cubre equivalencias de namespaces declarados en ancestros distintos.
- El validador XSD de evidencia descarga los XSD en cada RunId; podría cachearse bajo `.cache/`.

## Reglas del proyecto que no son negociables

- `AGENTS.md` (raíz) y el global de `~/.config/opencode/AGENTS.md`: español por defecto, respuestas cortas, commits convencionales sin atribución IA, sin `git push --force`, nunca tocar producción/instancias del usuario sin autorización.
- Verificación E2E: `.cursor/skills/verify-lumencat/SKILL.md` + su `features/README.md`. La app es un recurso en cola (`output/verification/.app-lock.json`). Distinguir siempre CORE PASS / GUI PASS / NOT RUN en docs y mensajes.
- Pruebas SDK (`#[ignore]`) exigen directorio nuevo bajo `output/verification/` vía variable de entorno (usan `fs::create_dir`); nunca sobrescriben ejecuciones anteriores.
- Cada corte: código + check observable + doc técnica con límites + `cargo fmt` + `cargo clippy --locked --all-targets -- -D warnings` + suite completa. Red→green: conserva el log rojo en `logs/tests/`.
- Logs en `logs/tests/`, artefactos en `output/verification/`, nada de eso en la raíz.

## Suggested skills

- `tldr` — formato de respuestas (arranque y cierres de turno).
- `verifying-completion` — antes de declarar DONE/pasado cualquier cosa.
- `lint-and-validate` — bucle post-cambio.
- `work-unit-commits` — dividir el corte en commits revisables.
- `research` (subagent background) — para investigaciones con fuentes primarias; no bloquees la implementación esperándolo.
- Proyecto: `.cursor/skills/verify-lumencat/` (no es skill del harness; se lee por ruta).

## Primer paso sugerido

Corte 1 (SDLTM sin SDK): crea `src/formats/sdltm_native.rs` con el lector sobre `snapshot()` existente, prueba primero contra `output/verification/sdltm-inspect/English-German.sdltm` en rojo, y despacha en `import_memory` según disponibilidad del SDK. Publica con GUI aún pendiente si el CORE queda verde.
