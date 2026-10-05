# XLIFF 1.2 segmentado: filas independientes sin perder el documento

**CORE + XML PASS. GUI y consumo en Trados: NOT RUN.** LumenCAT importa cada `mrk mtype="seg"` de `seg-source` como una fila, persiste sus ediciones y reconstruye el `target` sin cambiar `source`, segmentación, notas ni alternativas. La paridad completa sigue abierta.

## Contrato

| Caso | Comportamiento |
|---|---|
| Marcadores con `mid` | Correlación por identificador; conserva el orden original del destino aunque difiera del origen. Rechaza duplicados y correspondencias ambiguas. |
| Todos los marcadores sin `mid` | Correlación posicional, sin inventar identificadores XML. El destino existente debe tener el mismo número de marcadores. |
| Destino ausente, vacío o autocerrado | Crea el destino sobre la estructura segmentada del origen; conserva separadores, grupos exteriores, prefijos, namespaces locales y `xml:space`. Sin edición conserva el original byte a byte. |
| Códigos dentro de una fila | Reutiliza `Fragment` y los códigos protegidos del editor; exige conservación/balance al confirmar y reconstruye los códigos nativos al exportar. |
| Unidades ordinarias y segmentadas juntas | Conserva la representación de las unidades ordinarias y la identidad de cada fila segmentada. No requiere migración SQLite. |
| Estado y bloqueo | XLIFF 1.2 define `state` en `target` y `translate` en `trans-unit`, no en cada marcador. El destino queda traducido solo cuando todas sus filas están confirmadas; una fila borrador rebaja la unidad. Bloqueos distintos dentro de la unidad impiden exportar con explicación, sin expandirlos silenciosamente. |

El proyecto conserva los estados por fila. Al exportar/reimportar XLIFF 1.2, un estado mixto se reduce conservadoramente a borrador para todas las filas de la unidad. `approved="yes"` se rebaja al cambiar el contenido o dejar una fila sin confirmar.

## Evidencia y reproducción

- `tests/xliff_segmented.rs`: **7 pruebas aprobadas**. Importación, edición/confirmación, guardado, cierre/reapertura del proyecto, exportación y reimportación; destino reordenado por `mid`, códigos, metadatos, marcadores vacíos, CDATA/entidades equivalentes, prefijos, ausencia de destino, estados/bloqueos y negativos sin archivo parcial ni cambio del original.
- Antes de implementar, tres recorridos positivos fallaron por rechazo de `seg-source`: `logs/tests/xliff-segmented-red.log`.
- Formato, Clippy all-targets con `-D warnings` y **61 pruebas ordinarias aprobadas**: `logs/tests/xliff-segmented-validation.log`. La prueba SDLTM SDK permanece ignorada en este comando; no se contabiliza como ejecutada.
- Artefactos persistidos: `output/verification/xliff-segmented-20261005-1/` (`segmented.xlf`, `project.lcat`, `translated.xlf`). `System.Xml` validó ambos XLIFF contra el XSD estricto oficial de OASIS y recuperó independientemente el orden `b,a`, las traducciones y `g id="bold"`: `independent-consumer.json`. No es consumo en Trados ni GUI.
- Release compilada en `.cache/target/release/lumencat.exe`, SHA-256 `5A6D3EBEDC867BCE5653D934DD770E366618FFE3F87BD3C39DA49E5456BE859F`; log `logs/tests/xliff-segmented-release.log`. No se abrió ni sustituyó `output/lumencat.exe`.

```powershell
cargo test --locked --test xliff_segmented --test xliff_inline --test formats
```

Para conservar el proyecto y los archivos de la primera prueba, define `LUMENCAT_XLIFF_TEST_RUN_DIR` con una ruta **nueva** bajo `output/verification/`. El test exige `create_dir`, sin sobrescribir una ejecución anterior.

## Frontera y rollback

No admite identificadores parciales, marcadores anidados, destinos parcialmente segmentados o sin límites identificables, ni cambios de contenido entre `source` y `seg-source`. La comparación tolera CDATA/entidades, orden de atributos y elementos vacíos equivalentes; conserva los espacios, no adivina equivalencias lingüísticas ni clona etiquetas. Pares nativos que cruzan filas, `sub`, extensiones inline, comentarios traducibles, XLIFF 2.x y SDLXLIFF con estados/skeleton propietarios siguen abiertos. Los grupos exteriores se conservan como estructura inmutable, no como etiquetas movibles de cada fila.

Rollback: retirar `src/formats/segmented.rs` y su integración en `src/formats.rs`, junto con este test/documentación; no afecta al esquema de proyectos ni a TXT, DOCX o memorias. Los proyectos con filas segmentadas necesitan esta capacidad para exportar: una build anterior rechazará su `seg-source`, no debe reinterpretarlos como una sola fila.

Fuentes primarias: [OASIS XLIFF 1.2, §2.9 y §3.2.5](https://docs.oasis-open.org/xliff/v1.2/os/xliff-core.html), [XSD estricto](https://docs.oasis-open.org/xliff/v1.2/os/xliff-core-1.2-strict.xsd). Los esquemas descargados para la verificación permanecen junto a la evidencia.
