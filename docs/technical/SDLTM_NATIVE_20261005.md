# Lectura SDLTM nativa sin SDK — 2026-10-05

Corte 1 del plan de paridad de memorias ([mapa](../architecture/PARIDAD_FORMATOS.md), [investigación de fuentes](PARIDAD_MEMORIAS_TRADOS_FUENTES.md) §7-Fase A). Implementa `src/formats/sdltm_native.rs`: importar `.sdltm` en modo de solo lectura **sin Trados Studio instalado**. La actualización (`update_sdltm`) sigue requiriendo el puente del SDK.

## Despacho

`formats::import_memory` con `.sdltm`: si el SDK está disponible (4 DLL de `LUMENCAT_TRADOS_SDK_DIR` o `ProgramFiles/Trados/Trados Studio/Studio19`) usa el puente actual —fidelidad completa del motor, props `x-*` del dialecto—; si no, el lector nativo lee las tablas directamente. El original nunca se abre en escritura (`SQLITE_OPEN_READ_ONLY`).

## Esquema aceptado (evidencia de ambas generaciones)

Puerta estructural, no lista blanca de DDL: se exigen las tablas `translation_memories`/`translation_units` y las columnas que el lector usa (`source_language`, `target_language`, `tucount`; `id`, `source_segment`, `target_segment`, fechas/usuarios de sistema, `last_used_date`, `usage_counter`). Falta cualquiera → rechazo explícito nombrando la columna. Verificado contra dos esquemas reales:

| Fixture | Motor | Fingerprint (sha256 de `sqlite_master` normalizado, 16 hex) | Diferencias |
|---|---|---|---|
| `output/verification/sdltm-inspect/English-German.sdltm` (43 TUs) | viejo 8.10 | `78e3fc21b700440f` | base |
| `output/verification/sdltm-native-20261005-1/modern-43.sdltm` (43 TUs) | 19.0.0.3043, creada vía SDK con `FileBasedTranslationMemory(tmFilePath, …)` e importación TMX del propio SDK | `2a1ff2d29016ecee` | `translation_units` añade `fragment_hash`/`source_tags`/`target_tags`; índices `idx_tus_hashes`/`idx_tus_fragment_hashes`; **elimina** `fuzzy_index1/2/4`, `fi*_tuid`, `tus_main`, `idx_translation_unit_contexts`; `data_version` sigue en 1 |

El DDL embebido en `TranslationMemoryImpl.dll` con `confirmationlevel`/`relaxed_hash`/`tm_id`/`attributes_v2` **no** aparece en la TM creada por el motor 19.0: queda como variante no observada. Si una futura versión cambia columnas que el lector usa, la puerta estructural la rechaza con diagnóstico en vez de leer mal.

## Mapeo Segment→TMX (mismas unidades y códigos en ambas vías)

- `Text/Value` → texto visible; escapado estándar.
- `Tag Start` → `<bpt i="{Anchor}" type="{TagID}" x="{AlignmentAnchor}">…</bpt>` (`x` solo si `AlignmentAnchor ≠ 0`).
- `Tag End` → `<ept i="{Anchor}">…</ept>` (sin `type`: el motor 19.0 ya no serializa `TagID` en `End`).
- `Tag Standalone` → `<ph type="{TagID}" x="{AlignmentAnchor}">…</ph>`.
- El XML original de cada `Tag` se conserva **como contenido nativo** dentro de `bpt/ept/ph`, de modo que `raw_xml` es fiel ida/vuelta sin depender de los atributos.
- Alineado con el exporter oficial observado (`old-export.tmx` del SDK: `<bpt i="1" type="1" x="1" />`, `<ept i="1" />`, `<ph type="1" />`).
- Variantes no modelables (`TextPlaceholder`, `LockedContent`, tipos desconocidos), `Start/End` sin `Anchor`, XML desbalanceado → **rechazo explícito con diagnóstico**; nunca texto aplanado ni silencio.

Campos de sistema → atributos TMX `creationdate/creationid/changedate/changeid/lastusagedate` (fecha SQLite `YYYY-MM-DD HH:MM:SS` → `YYYYMMDDThhmmssZ`; valor ausente o no verificable → atributo omitido, sin inventar) y `usagecount`. `flags`, `guid` y atributos de campos no se mapean (bitfield/campo sin documentación pública).

## Controles de la vía nativa

Par exacto declarado vs solicitado (insensible a mayúsculas, mensaje espejo del puente); exactamente una fila en `translation_memories`; reconciliación `tucount` vs unidades leídas (desviación → importación rechazada); segmentos `NULL`/vacíos → rechazo con id de unidad; cancelación por fila.

## Verificación

- **CORE PASS (sintético)**: `tests/sdltm_native.rs` — mapeo de tags/campos de sistema, coincidencia exacta con códigos, concordancia, round-trip export/import TMX, rechazos (desviación de tucount, par erróneo, `TextPlaceholder`, `End` sin `Start`, archivo ajeno, esquema incompleto, cancelación) y despacho a vía nativa sin SDK (variable `LUMENCAT_TRADOS_SDK_DIR` apuntando a ruta inexistente). Log rojo inicial en `logs/tests/sdltm-native-red-20261005.log`.
- **CORE PASS (fixtures reales)**: `output/verification/sdltm-native-accept-20261005-1` y `-2` — fixture pública 8.10 (43 TUs, TU con par de tags recuperada con `<g id="1">`, exact match en store, TMX 43/43 reimportado, original byte a byte intacto) y fixture moderna 19.0 (43 TUs, **conjunto de fuentes idéntico** al de la vía antigua).
- **CORE+SDK PASS (regresión)**: `output/verification/sdltm-sdk-regress-20261005-1` — el puente sigue intacto (43→44, corrección, hash original sin cambios).
- Suite completa: 68 ordinarias + 3 ignoradas; `cargo clippy --locked --all-targets -- -D warnings` y `cargo fmt` limpios.
- **GUI NOT RUN.**

## Límites declarados

- Sin Trados **no hay `update_sdltm`**: la actualización exige el SDK (regla del mapa: nunca escribir SDLTM por SQL propio).
- La vía nativa no sintetiza las props `x-*` del exporter del SDK (`x-Origin`, `x-ConfirmationLevel`, …): quien las necesite debe importar con Trados instalado.
- `CultureName` por segmento se ignora; el par autoritativo es el de `translation_memories`.
- El parser exige XML compacto del motor (sin espacios entre elementos de `Segment`): un archivo re-serializado con pretty-print se rechaza.
- Observación del propio SDK documentada: el `Standalone` de la TU 36 de la muestra no sobrevive al circuito export→import del motor (la fixture moderna quedó sin él); el lector nativo sí lo mapea cuando existe.

## Procedimiento de las fixtures modernas

`output/verification/sdltm-native-20261005-1/create_modern.ps1` (bajo Windows PowerShell 5.1; pwsh rompe con `System.ServiceModel` de los ensamblados .NET Framework): crea la TM por reflexión del constructor de 8 argumentos, exporta la muestra antigua con el puente de producción e importa el TMX resultante a la moderna. `probe_ctors.ps1`/`probe_params.ps1` documentan la reflexión de la API.
