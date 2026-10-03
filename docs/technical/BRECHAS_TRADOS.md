# Brechas frente a Trados y plan de implementación de LumenCAT

Fecha: 3 de octubre de 2026. Comparación documental de Trados y revisión estática del árbol de trabajo actual de LumenCAT (`main`, HEAD `68cd8e8`, con cambios locales). No se ejecutó Trados ni se repitió la validación GUI/Word de LumenCAT. Las prioridades son recomendaciones, no un roadmap aprobado.

Actualización del mismo día: implementación autorizada por entregas, con publicación de avances verificados en GitHub `main`. Los nombres de tipos, tablas y módulos del plan son propuestas, no obligaciones. El registro siguiente distingue núcleo implementado, integración de interfaz y validación pendiente; un corte no acredita el paquete completo.

## Avance de implementación

| Paquete/corte | Estado y alcance |
|---|---|
| F00.1/F00.2 | PASS del núcleo: backup SQLite cancelable sin sobrescritura, recuperación a copia nueva, migración v1→v2 con respaldo e historial agrupado durable. `cargo test --locked --tests`: 33 pruebas; `cargo fmt --check` y Clippy all-targets sin errores. UI de respaldo/recuperación pendiente. |
| F01–F18 | Pendientes. F18 sigue siendo una decisión de producto opcional, no una dependencia del núcleo local. |

La rama de trabajo aislada parte de `68cd8e8`; no incorpora los cambios locales previos de GPUI/lockfile/documentación del checkout principal. Los respaldos y migraciones se prueban únicamente sobre proyectos temporales. Ver [recuperación de proyectos](../guides/RECUPERACION.md).

## Conclusión

LumenCAT cubre el núcleo inicial: editor bilingüe, confirmación y bloqueos, guardado e historial, TMX con coincidencias exactas/fuzzy y concordancia, QA textual y exportación TXT/XLIFF/DOCX. Las brechas principales son memoria que aprende al confirmar, terminología, cobertura documental, segmentación lingüística, reutilización por lotes y entrega/revisión para agencias. La asistencia LLM tampoco está implementada y Trados ya ofrece integración de IA: por sí sola no constituiría una diferenciación.

La web oficial ya anuncia Studio 2026 Release; algunos resultados indexados todavía describen 2024. Para capacidades consolidadas se usan también manuales, SDK y formación oficial de versiones anteriores, identificando su alcance. No se certifica la matriz comercial completa de 2026.

## Matriz de brechas

«Ausente» significa que no se encontró un flujo implementado en los módulos, almacenamiento y UI examinados. «Parcial» significa que existe una base, pero no equivalencia funcional. Los números de línea corresponden al árbol examinado y pueden desplazarse.

| Función documentada de Trados | Estado de LumenCAT y evidencia local | Impacto práctico |
|---|---|---|
| **Actualizar TM al traducir/confirmar**, conservando variantes según operación. [SDK de actualización TM](https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/updating_translation_memories.html). | **Ausente en el flujo de confirmación.** `confirm_active` → `save` → `Task::Edit` → `ProjectStore::edit` → `apply_edit` cambia segmento/historial, sin insertar una TU. Véanse `src/gpui_app/mod.rs:355`, `src/worker.rs`, `src/storage.rs:242,496`. Las inserciones TM se realizan mediante importación y `insert_tm_units`. | Las traducciones nuevas quedan guardadas en el documento, pero no pasan a ser sugerencias de la memoria. Es la brecha más inmediata del ciclo CAT. |
| **Bases terminológicas**: reconocer, recuperar y añadir términos durante la traducción; múltiples termbases. La edición avanzada se apoya en MultiTerm. [Studio 2024 SR1](https://www.trados.com/blog/whats-new-in-trados-studio-2024-sr1/). | **Ausente.** `src/model.rs`, esquema de `src/storage.rs`, `src/worker.rs` y paneles GPUI no contienen una termbase ni operaciones de términos. TM y concordancia no sustituyen un glosario. | No se pueden aplicar reglas de vocabulario del cliente ni distinguir términos preferidos/prohibidos. |
| **Segmentación lingüística**, separar/fusionar segmentos y autopropagar repeticiones. [Temario oficial Studio 2024](https://www.trados.com/media/es/images/Trados%20Studio%202024%20-%20Level%201%20-%20course%20outline%20EN_tcm238-165629.pdf), [reglas de segmentación RWS](https://www.rws.com/glossary/segmentation-rules/). | **Ausente.** DOCX usa un segmento por párrafo (`src/formats/docx.rs:1`); TXT por línea (`src/formats.rs:110`). XLIFF usa las unidades importadas y rechaza `seg-source` (`src/formats.rs:242`). Sin comandos de dividir, fusionar o propagar en `Task`. | Los párrafos largos reducen la reutilización de frases y las repeticiones requieren intervención individual. |
| **TM con contexto y presentación de diferencias fuzzy**; concordancia que encuentra variantes derivadas. [SDK de búsquedas](https://developers.rws.com/studio-api-docs/16.1/apiconcepts/translationmemory/performing_translation_memory_lookups.html). | **Parcial.** Hay exact/fuzzy y concordancia (`src/tm.rs:88,162`), pero el propio motor declara recuperación léxica aproximada, sin context matches. `TmUnit` no almacena la secuencia contextual; la concordancia usa FTS de origen con tokens unidos por OR. | Un 100 % no distingue contexto; se pierde ayuda para escoger variantes y adaptar coincidencias. |
| **AutoSuggest, fragmentos y reparación fuzzy**: sugerencias mientras se escribe y reutilización de partes de segmentos. [Curso de fragmentos](https://www.trados.com/training/eLearning-trados-studio-level-2-segment-fragments/), [evolución oficial de TM](https://www.trados.com/blog/the-past-and-present-of-translation-memory-technology/). | **Ausente.** Se aplican coincidencias de segmento completas (`insert_tm_match`); no se encontró autocompletado lingüístico, extracción de fragmentos ni reparación de una traducción fuzzy. | Menos reutilización cuando solo parte de una frase coincide. |
| **Análisis y pretraducción por lotes**, plantillas y automatización de preparación/generación. [Project Automation SDK](https://developers.rws.com/studio-api-docs/17.0/apiconcepts/projectautomation/overview.html). | **Ausente en el producto.** Hay documentos y progreso local, pero no tareas de análisis de repeticiones/fuzzy, pretraducción con umbral ni plantillas. `src/storage.rs:249` mide destinos no vacíos, no esfuerzo ni aprobación QA. | Falta estimar trabajo y preparar documentos repetitivos. La pretraducción sería opcional: no exige traducción automática desatendida. |
| **Alineación** de originales y traducciones previas para crear TM. [Curso oficial de alineación](https://www.trados.com/training/eLearning-trados-studio-level-2-alignment/). | **Ausente.** Se importan TMX ya construidos; no hay flujo para emparejar documentos ni revisar alineaciones. | No permite aprovechar directamente un archivo histórico de documentos bilingües separados. |
| **Más de 50 tipos de archivo y filtros configurables**. [Tipos de archivo oficiales](https://www.trados.com/ecosystem/languages-and-file-types/), [configuración de filtros SDK](https://developers.rws.com/studio-api-docs/17.1/apiconcepts/filetypesupport/file_type_settings.html). | **Parcial.** Solo TXT UTF-8, XLIFF 1.2 textual y DOCX (`src/model.rs:23`, `src/formats.rs:89`). Sin Excel, PowerPoint o InDesign. XLIFF/TMX rechazan códigos inline. DOCX admite tablas, formato mixto e imágenes estáticas, pero rechaza hyperlinks, campos, revisiones y otras estructuras (`src/formats/docx.rs:264,452`). Historias secundarias también siguen limitadas. | Restringe los encargos aceptables y el intercambio profesional. El número de formatos de Trados no demuestra compatibilidad perfecta de cada documento. |
| **QA configurable y verificación terminológica**, durante/después de traducir y perfiles reutilizables. [Curso QA/Term Verifier](https://www.trados.com/training/eLearning-trados-studio-level-2-qa-checker-and-term-verifier/), [temario de perfiles QA](https://www.trados.com/media/images/trados-studio-level-2-qa-check-and-term-verifier-elearning_tcm234-220875.pdf). | **Parcial.** `src/qa.rs:4` revisa vacío, identidad, espacios, dígitos, tokens, puntuación y confirmación. GPUI consulta el segmento activo (`src/gpui_app/mod.rs:444`). Sin termbase, perfiles, corrector ortográfico ni informe QA de documento/proyecto. El exportador DOCX sí valida sus códigos: no es ausencia total de protección de tags. | Falta una revisión global reproducible y adaptada al cliente. |
| **Revisión y retorno de correcciones**, incluida exportación bilingüe a Word y reimportación. [Respuesta de soporte RWS sobre Bilingual Review](https://community.rws.com/product-groups/trados-portfolio/trados-studio/f/studio/20219/how-to-make-bilingual-word-file), [funciones por suscripción](https://www.trados.com/spotlight/trados-studio-subscription-benefits/). | **Ausente como flujo editorial.** Solo estados Draft/Confirmed (`src/model.rs:47`) e historial técnico. Sin comentarios de revisión, aceptación/rechazo editorial o round-trip Word bilingüe. Exportar DOCX destino es otra operación. | Un revisor externo no puede devolver cambios estructurados al proyecto. |
| **Paquetes y retornos de proyecto**: archivos y recursos para traductor/revisor. [SDK de paquetes](https://developers.rws.com/studio-api-docs/15.2/apiconcepts/projectautomation/about_packages.html), [ediciones Studio](https://www.trados.com/spotlight/trados-studio-subscription-benefits/). | **Ausente.** No hay SDLPPX/SDLRPX ni soporte declarado SDLXLIFF; aceptar XLIFF textual no acredita compatibilidad con extensiones, skeleton y estados Trados. | Barrera para encargos de agencias que exigen un paquete de retorno. Freelance abre paquetes; la creación de paquetes de proyecto corresponde a Professional según la comparación consultada. |
| **PerfectMatch** para reutilizar versiones anteriores considerando contexto. [Explicación oficial](https://www.trados.com/blog/whats-new-in-trados-q2-2024-round-up/), [curso con alcance Professional](https://www.trados.com/media/images/studio-level-2-working-with-perfectmatches_tcm234-234101.pdf). | **Ausente.** No se comparan nuevas versiones con bilingües anteriores ni se preserva aprobación por correspondencia contextual. | Las actualizaciones documentales no tienen un flujo especializado de reutilización. En desktop es capacidad Professional, no base universal. |
| **Traducción automática e IA contextual** con proveedores y recursos lingüísticos. [Studio actual](https://www.trados.com/product/studio/), [novedades 2026](https://www.trados.com/resources/whats-new-in-trados-studio-2026-release/). | **Ausente.** `docs/PRODUCT.md` la declara visión; no se encontró proveedor MT/LLM ni operación correspondiente en `src/worker.rs`. | La ventaja futura debe depender del contexto, control del traductor, privacidad y calidad del flujo. «Tenemos IA» ya no bastaría. Proveedores externos pueden requerir cuenta, conexión y costes propios. |
| **SDK y plugins** para proveedores, filtros, integración y automatización. [RWS Developers](https://developers.rws.com/), [SDK de Studio](https://developers.rws.com/studio-api-docs/apiconcepts/overview.html). | **Ausente como interfaz pública de extensión.** Módulos Rust y utilidades internas no son un contrato para plugins ni una API de integración documentada. | Menos vías para ampliar formatos y conectar herramientas de terceros. No todos los plugins vienen incluidos en Studio. |

## Orden recomendado

1. **Cerrar el ciclo local de TM:** confirmar → incorporar a memoria, con reglas explícitas para variantes y correcciones. Añadir terminología local y selección de idiomas en GPUI; hoy los valores iniciales son `en`/`es` (`src/gpui_app/mod.rs:149`).
2. **Entregar documentos fiables:** ampliar Word con hyperlinks, saltos/tabulaciones, encabezados/pies y notas; luego segmentación por oración y QA de documento. Mantener pruebas sobre el documento exportado real.
3. **Reutilizar trabajo:** propagación opcional, pretraducción controlada, análisis, alineación y recursos TM distinguibles por cliente/proyecto. El esquema actual reúne las TU en una tabla sin identidad de memoria separada.
4. **Facilitar encargos de agencias:** XLIFF con códigos y estados, intercambio SDLXLIFF/paquetes validado con fixtures y revisión externa. Priorizar antes si las agencias son el canal comercial inmediato; implica mayor coste de compatibilidad que un núcleo local propio.
5. **Asistencia contextual a petición:** integrar IA después de asegurar el contexto documental, las memorias y la terminología. Respeta la dirección de producto actual y evita ampliar infraestructura cloud sin necesidad.

No hace falta copiar toda la plataforma. Cloud, colaboración centralizada y gestión empresarial pertenecen a capacidades y planes adicionales de Trados; su coste de infraestructura puede contradecir el núcleo local sin cuenta de LumenCAT. MultiTerm avanzado, PerfectMatch Professional, AppStore y servicios de proveedores deben distinguirse de Studio base.

## Límites y discrepancias

- La comprobación de LumenCAT fue estática: no prueba ergonomía, rendimiento, accesibilidad ni compatibilidad universal de los formatos.
- `docs/PRODUCT.md` conserva límites antiguos que dicen que DOCX rechaza formato mixto y dibujos. El parser actual y `docs/technical/CONTINUACION_DOCX.md` reflejan soporte de formato mixto e imágenes estáticas. Conviene reconciliarlo en una tarea aparte; no se modificó aquí.
- GPUI solicita datos por páginas, pero construye filas para `0..count` dentro de un contenedor con scroll (`src/gpui_app/mod.rs:1593`). Esa paginación no constituye virtualización visual. No se midió su rendimiento ni se compara con Trados.
- Algunas páginas devolvieron protección JavaScript o 403; la lista detallada de filtros no pudo consultarse íntegra. Se usaron páginas oficiales accesibles, resultados indexados oficiales y fuentes alternativas RWS. No se dedujo una lista exhaustiva ni la disponibilidad de todos los filtros en cada edición.
- Studio 2026 se verificó documentalmente en páginas oficiales actuales; las funciones históricas aquí citadas no se probaron en una instalación de esa versión. La edición exacta, cuota y licencia de servicios conectados requieren comprobación al comprar o integrar.

## Plan técnico: objetivo y reglas de ejecución

Convertir LumenCAT en una CAT local utilizable para encargos reales, manteniendo la autoridad del traductor, la integridad del proyecto y la fidelidad de los documentos. Se cubren las brechas anteriores sin asumir que sea necesario copiar todas las ediciones o servicios de Trados.

Este plan amplía M2/M3/M4 de [MVP.md](../architecture/MVP.md); [PLAN_LUMENCAT.md](../codex/plans/PLAN_LUMENCAT.md) describe la entrega inicial y contiene información histórica. No tomar su ausencia antigua de remoto, sus conteos de tests o su interfaz egui como estado actual. Al ejecutar, actualizar las decisiones y guías afectadas por cada capacidad; no convertir una propuesta de este archivo en una capacidad implementada en `PRODUCT.md`.

Reglas compartidas:

- Reutilizar `ProjectStore`, `Task`/`Data`, `Cancellation`, parsers, exportación a archivo nuevo y controles accesibles. Crear módulos solo cuando exista lógica concreta que separar; conservar el paquete Rust único.
- El worker posee la escritura SQLite. La UI envía comandos con IDs y revisiones esperadas, conserva borradores y descarta resultados obsoletos. Parsing, búsqueda y red no deben ejecutarse dentro del render.
- Guardar, confirmar, aprobar, aplicar una propuesta y exportar son acciones diferentes. Una respuesta de commit acredita durabilidad; una barra de progreso no acredita aprobación ni compatibilidad de entrega.
- Mantener las fuentes inmutables, códigos validables y el esqueleto original. Una estructura desconocida se rechaza antes de crear un proyecto parcial o un archivo aparentemente completo.
- Las operaciones sobre varias filas requieren previsualización, revisiones esperadas, cancelación y un grupo de historial. No se confirma automáticamente lo que propone una TM, un algoritmo o un LLM.
- Implementar GPUI como superficie principal. Cuando cambie una regla del núcleo, adaptar también `src/app.rs` si sigue habilitado `--legacy-egui`; las pantallas nuevas pueden ser exclusivas GPUI y deben declararlo. Evitar reglas contradictorias entre las dos interfaces.
- Credenciales y preferencias de máquina permanecen fuera del proyecto portable. El núcleo funciona sin cuentas ni red. Cada llamada conectada debe responder a una acción autorizada por el traductor.

### Orden de entregas y dependencias

La secuencia es por capacidades terminadas, sin fechas ni estimaciones inventadas. Las tareas pueden dividirse, pero una fase no sale solo porque compile.

| Entrega | Paquetes | Resultado observable y dependencia |
|---|---|---|
| E0 — Base segura | F00, F01 y primer corte F17 | Proyecto anterior recuperable, idiomas elegibles, grid grande operable. Antes de cualquier cambio de schema. |
| E1 — Memoria útil | F02, F03 | Confirmar alimenta la memoria elegida; términos del cliente se reconocen y consultan. Depende de E0. |
| E2 — Word y calidad | F04, F05, F06 | Códigos verificables, más contenido Word traducible y QA global. F04 habilita la ampliación Word; F06 usa F03. |
| E3 — Reutilización | F07, F08, F09, F10 | Oraciones editables, repeticiones controladas, análisis, alineación y ayudas TM. Requiere mapeo documental F04/F07 y transacciones F00. |
| E4 — Intercambio | F11, F12, F13 | XLIFF con códigos, revisión externa y paquetes validados. F13 depende de F11 y fixtures Trados. |
| E5 — Asistencia contextual | F15 | Propuestas MT/LLM solicitadas y aceptadas por el traductor. Usa E1 y contratos documentales de E2; puede comenzar antes de completar todos los paquetes de E4. |
| E6 — Ampliación selectiva | F14, F16; F18 opcional | Nuevos formatos justificados por demanda, API estable y, solo si se decide, colaboración. No bloquea el núcleo local. |

F17 acompaña las entregas; no se deja rendimiento y entrada de texto para el final. La prioridad de Word incluye texto en visualizaciones, como indica `PRODUCT.md`, aunque su soporte exige cortes de complejidad distinta. Si aparecen clientes de agencias, adelantar F11/F13 después de sus dependencias, sin saltarse fidelidad ni recuperación.

### Arquitectura y evolución de datos propuestas

No ejecutar el siguiente diseño como un bloque SQL único. Cada paquete introduce su migración mínima y comprobable, con backup previo. No hay números definitivos de versión ni obligación de crear todas las tablas ahora.

| Área | Diseño propuesto | Invariante |
|---|---|---|
| Configuración | Metadatos de proyecto y configuración por par de idiomas; referencias a memorias, termbases y perfiles. | Idiomas de un documento importado no cambian por cambiar la configuración del proyecto. |
| Historial | Cabecera de operación y entradas before/after por entidad; importar historial v1 conservando su orden/cursor. | Deshacer una operación masiva revierte toda la operación y sus efectos locales asociados. |
| Memorias | Colecciones identificables; TU importadas y aprendidas con procedencia, versión y estado activo; asociación de aprendizaje al segmento/operación. | Datos importados conservados, variantes explícitas y búsqueda filtrada por memoria/idiomas. |
| Terminología | Concepto, expresiones por idioma, estado preferred/allowed/forbidden y metadata; selección de bases por proyecto. | Una TU no es un término; no se reduce una base multilingüe a pares ambiguos. |
| Documento | Contenedores originales y unidades editoriales vinculadas a parte/rango/ID externo, con mapa de códigos. | Cambiar el número de segmentos editoriales no altera la identidad ni el número de contenedores originales. |
| Calidad/revisión | Perfil QA versionado, resultados ligados a revisión, comentarios y estado editorial independiente. | Un informe previo o una aprobación dejan de ser actuales al cambiar el destino. |
| Integración | Propuestas y tareas ligadas a proyecto, documento, revisión y generación de recursos. | Ningún resultado externo tiene permiso directo de escribir o confirmar. |

No construir un bus de eventos, un motor de plugins o un framework de workflows antes de necesitarlo. El cambio mínimo debe respetar estos contratos; las nuevas abstracciones se justifican por dos consumidores reales o por una frontera de datos necesaria.

## Paquetes de implementación

### F00 — Migraciones, respaldo e historial de operaciones

**Dependencias:** ninguna. **Archivos:** `src/storage.rs`, `src/model.rs`, `src/worker.rs`, tests de almacenamiento/recuperación; crear `src/storage/migrations.rs` solo si separarlo simplifica el módulo.

1. Reemplazar el rechazo de toda versión distinta de v1 por un camino de migraciones conocidas. Verificar `application_id`, versión y objetos existentes antes de modificar; seguir rechazando bases ajenas y versiones futuras. En una base existente, abrir sin permiso de creación para no convertir una ruta mal escrita en un proyecto vacío.
2. Usar la feature `backup` de rusqlite ya declarada para crear un backup SQLite coherente, incluyendo commits presentes en WAL, en un destino nuevo. No copiar solo el `.lcat` abierto. Validar el backup y registrar la ruta antes de migrar. Véanse [API de rusqlite 0.37](https://docs.rs/rusqlite/0.37.0/rusqlite/backup/index.html) y [SQLite Backup](https://www.sqlite.org/backup.html).
3. Ejecutar cada migración y el cambio de `user_version` en una transacción. Comparar conteos, claves, relaciones y muestras de texto antes/después; comprobar `foreign_key_check` e integridad. Si falla, rollback y conservar el backup; no borrar tablas para volver a empezar.
4. Evolucionar `history`: una operación contiene una o varias entradas y un cursor durable. Los registros v1 se convierten en operaciones individuales conservando qué se puede deshacer/rehacer. Una edición nueva tras undo invalida únicamente la rama redo y sus efectos asociados.
5. Definir un resultado de comando que devuelva entidades modificadas, IDs afectados y revisión de operación. Invalidar páginas, búsqueda, TM, QA y estadísticas de los documentos afectados en ambas interfaces, sin recargar todo el proyecto.
6. Implementar restauración como apertura de una copia recuperada en una ruta nueva, con el proyecto original cerrado; no restaurar sobre una base abierta ni reutilizar sus WAL/SHM. Mostrar el resultado del respaldo sin revelar contenido.

**Aceptación:** un proyecto v1 con documentos, historial y TM migra sin pérdida y su copia de backup abre con la versión anterior; una falla inducida deja el proyecto anterior consistente; una edición masiva se deshace/rehace como unidad tras reiniciar. Extender los tests existentes con datos temporales y usar el probe de proceso para interrupciones, sin tocar proyectos del usuario.

### F01 — Idiomas, configuración de proyecto y recursos

**Dependencias:** F00. **Archivos:** `src/model.rs`, `src/storage.rs`, `src/worker.rs`, `src/gpui_app/mod.rs`; adaptar configuración legacy existente.

1. Añadir origen/destino en creación/importación GPUI en lugar de fijar `en`/`es`. Persistir valores predeterminados y permitir variantes regionales; conservar el identificador original del archivo y validar el dialecto admitido por cada formato.
2. Para XLIFF, contrastar configuración con `source-language`/`target-language`: mostrar la discrepancia y exigir una elección explícita. No relabelar automáticamente datos ya importados ni mezclar `pt-BR` y `pt-PT` en exact matching.
3. Asociar recursos al proyecto y al par lingüístico. En el primer corte son colecciones dentro del `.lcat`; import/export TMX permite compartir. Memorias externas compartidas entre proyectos quedan como ampliación con contrato propio de bloqueo/versiones, sin introducir ahora una segunda base escritora.
4. Añadir configuraciones de filtros, perfil QA y política de IA solo cuando sus paquetes estén implementados. Las plantillas futuras contienen valores y referencias portables, sin credenciales ni rutas de máquina asumidas.

**Aceptación:** crear `fr→es` desde GPUI, importar/traducir/exportar y reabrir conserva idiomas; un TMX de otro par no aparece como candidato; un XLIFF con conflicto lingüístico no entra silenciosamente con metadata equivocada.

### F02 — Memorias independientes y aprendizaje al confirmar

**Dependencias:** F00/F01; usar F04 para segmentos con códigos. **Archivos:** `src/tm.rs`, `src/storage.rs`, `src/model.rs`, `src/worker.rs`, GPUI y `src/app.rs`.

**Cambio de decisión:** la decisión 002 dice «Confirmar es humano y no escribe TM automáticamente». El nuevo flujo propuesto es confirmación humana que alimenta una memoria de escritura seleccionada; modificar ese contrato en `DECISIONES.md` cuando se implemente. Guardar un borrador continúa sin aprender.

1. Añadir colecciones con nombre, par lingüístico, prioridad, penalización y modo lectura/escritura. Migrar las TU actuales a una colección «Memoria importada» sin cambiar texto ni `raw_xml`. Elegir una colección de escritura del proyecto; permitir desactivar aprendizaje.
2. Introducir un comando de confirmación separado de `Edit`: incluye destino actual, revisión esperada y política de aprendizaje. Los handlers de GPUI y egui usan el mismo comando. No deducir la intención solo de `state=Confirmed`, porque importación, undo y bloqueo también pueden restaurar ese estado.
   Si hay autosave en vuelo, conservar una intención de confirmación ligada al serial del borrador y enviarla después del ack correspondiente con la revisión actualizada. No perder la confirmación por `active.saving`, confirmar otra versión ni avanzar dando éxito antes del commit; si el traductor vuelve a editar, resolver o invalidar la intención explícitamente.
3. Confirmar guarda segmento, historial y contribución a TM en la misma transacción. Rechazar destinos vacíos y códigos estructuralmente inválidos; los avisos lingüísticos no bloquean salvo regla de perfil explícita. Si no hay memoria de escritura, confirmar sigue funcionando y el resultado informa que no hubo aprendizaje.
4. Contribución aprendida identificada por memoria+segmento, con versiones conservadas y una versión activa. Reconfirmar texto idéntico es idempotente; corregirlo sustituye solo la contribución de ese segmento, no otras variantes ni TU importadas. La consulta agrupa duplicados equivalentes preservando procedencias.
5. Propuesta para evitar sugerencias obsoletas: al editar el destino de un segmento confirmado, suspender su contribución aprendida hasta reconfirmar. Guardar ese efecto junto al historial. Undo/redo restaura destino, estado y contribución como una unidad; no elimina una TU importada igual ni las contribuciones de otros segmentos.
   Aplicar esta regla en la capa común de comandos/storage, revisando edición normal, reemplazo, revisión externa y aceptación de propuestas. No implementarla únicamente en el handler de teclado de GPUI.
6. Añadir índices y filtrado por memoria/estado. El schema actual solo tiene trigger FTS de inserción: resolver también sincronización de actualización/borrado de contenido indexado, o versiones inmutables más filtro de estado activo. Verificar búsqueda después de editar y deshacer, no solo tabla TM.
7. TU aprendida textual usa el serializador TMX existente con `raw_xml` vacío. Una TU importada modificada no puede mantener XML raw que describe el destino anterior: regenerar el contenido soportado preservando metadata compatible y revalidar. Hasta F04/F11, excluir aprendizaje de segmentos con códigos con aviso explícito; nunca exportarlos como texto escapado fingiendo TMX inline compatible.
8. Refrescar el panel de coincidencias tras commit/invalidation. Mostrar memoria y procedencia; indicar ambigüedad si hay varios destinos para la misma fuente. No restar o sumar porcentajes al texto bruto sin separar score y penalización.

**Aceptación:** confirmar A permite sugerir su traducción al abrir una repetición B y tras reiniciar; editar A suspende la versión vieja; reconfirmar publica la nueva; undo/redo revierte ambas cosas. Confirmación repetida no duplica contribuciones, memoria de lectura no se modifica y export/reimport TMX conserva contenido. Una falla a mitad de confirmación no deja segmento confirmado sin su efecto de memoria.

### F03 — Terminología local y verificación de vocabulario

**Dependencias:** F00/F01; F04 para ignorar códigos al buscar. **Archivos:** crear `src/terminology.rs`, ampliar modelo/storage/worker y panel de recursos GPUI.

1. Modelar concepto y expresiones por idioma con estado preferido, permitido o prohibido, notas, dominio y procedencia. Añadir varias bases seleccionables; no forzar una correspondencia única para un término polisémico.
2. Primer import/export TSV con encabezados y reglas de escape documentadas, preview y validación de columnas/idiomas. Elegir una dependencia de lectura tabular solo si la biblioteca estándar no cubre el contrato elegido. Importación atómica, cancelable y deduplicación que conserve variantes; comparar filas recibidas, aceptadas, duplicadas y rechazadas.
3. Reconocimiento inicial por secuencias de palabras y frases en texto visible, normalización Unicode auxiliar y política de mayúsculas explícita. Mantener offsets en el texto original para resaltar. No usar substring que encuentre «art» dentro de «party». Lenguas sin espacios requieren reglas propias; no prometer morfología general.
4. Añadir consulta manual y acción «Añadir término» usando selección origen/destino cuando F17 permita selección fiable; mantener entrada manual como alternativa. Inserción solo por acción humana, sin cambiar estado a confirmado.
5. QA verifica presencia de una equivalencia permitida y uso de formas prohibidas, con excepciones por concepto/segmento. Emparejar términos fuente con sus variantes destino, sin exigir todas las traducciones de un concepto simultáneamente.
6. Después, implementar un dialecto TBX explícito contra especificación y corpus. Registrar qué metadata se soporta y qué causa rechazo; no anunciar TBX universal. El soporte `.sdltb`/MultiTerm requiere estudio separado y no se deduce de leer TBX.

**Aceptación:** «privacy notice» reconoce la frase y ofrece el término correcto sin confundir palabras parciales; una forma prohibida genera issue localizado; importar/reexportar conserva variantes Unicode y estado; cancelar no incorpora media base. Un término ambiguo muestra alternativas y procedencia.

### F04 — Códigos inline y mapeo entre editor y documento

**Dependencias:** F00; no requiere haber terminado F03. **Archivos:** `src/model.rs`, `src/editing.rs`, `src/formats.rs`, `src/formats/docx.rs`, storage y componentes/input GPUI.

1. Extraer el contrato de códigos a partir de DOCX existente: texto, apertura/cierre de región y marcador atómico, identidad estable y restricciones de movimiento. Mantener la representación persistida compatible cuando sea posible; un modelo tipado se incorpora donde lo necesitan validación, matching o exportación.
2. Separar identidad de código de su posición y de IDs específicos de un documento. Validación distingue códigos ausentes, duplicados, desconocidos, cierre incorrecto y movimientos prohibidos. Buscar, contar palabras, QA numérico y resaltar TM operan en texto visible y conservan un mapa de offsets al texto real.
3. Formalizar contenedores originales: parte del paquete, elemento/unidad externa y localización estable dentro del original. Vincular unidades editoriales a esos contenedores. No usar `ordinal` como identidad, ni offsets de archivo exportado para reconstruir el original de la siguiente exportación.
4. Aplicar una coincidencia con códigos solo si puede construirse una correspondencia inequívoca por función/estructura. Si no puede, ofrecer texto como referencia sin reemplazar destino. Separar texto literal que contiene `<g>` de códigos estructurales para no convertir datos ordinarios en markup.
5. Mantener protección ante borrado/pegado y validación dura antes de exportación. En el primer corte no hace falta editor WYSIWYG: badges y selección atómica son suficientes. Manejar UTF-8/UTF-16, IME y clipboard sin cortar códigos ni caracteres.

**Aceptación:** tags válidos sobreviven import→editar→buscar/reemplazar→export; pérdida de un código impide exportar a un archivo final; QA ignora IDs numéricos de tags. Una TM con IDs de otro documento se aplica solo con remapeo probado. Los casos DOCX ya soportados conservan su resultado.

### F05 — Ampliación progresiva de Word

**Dependencias:** F04; F07 antes de exportar múltiples oraciones de un mismo párrafo. **Archivos:** `src/formats/docx.rs`, modelo/mapeo documental, `tests/docx.rs`, verificadores Word/DOCX existentes.

Cada corte sale por separado, con fixture real y lista explícita de estructuras admitidas. La estructura por partes y runs está descrita en [Microsoft WordprocessingML](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document). Reutilizar `zip` y `quick-xml`; no convertir Word a HTML como formato intermedio.

| Corte | Cómo implementarlo | Prueba de entrega |
|---|---|---|
| F05.1 — Tabulaciones y saltos | Representar `w:tab`, `w:br` y `w:cr` como códigos con tipo/atributos preservados. Ampliar runs que mezclan texto y estos marcadores. | Word muestra saltos y tabulaciones equivalentes; texto traducido y códigos no filtrados. |
| F05.2 — Hipervínculos | Tratar su texto como región traducible y conservar wrapper, propiedades, relación o ancla. No navegar el enlace. Permitir anidamiento de estilos dentro de vínculo solo cuando el modelo lo admita. | URL/ancla intacta, texto traducido, enlace conservado al abrir Word. |
| F05.3 — Historias secundarias | Enumerar headers/footers/footnotes/endnotes por relaciones OPC, con IDs parte+contenedor. Modificar solo las partes aceptadas; distinguir contenido auxiliar no traducible. | Traducción visible en cada historia; referencias, secciones y numeración conservadas. |
| F05.4 — Listas y campos | Preservar definiciones y referencias de numeración. Para campos, separar instrucciones, delimitadores y resultado; autorizar solo tipos explícitos. Campos calculados pueden regenerar su resultado en Word: no traducir indiscriminadamente cachés ni instrucciones. | Numeración intacta y campos funcionales; tipos desconocidos aún rechazados. |
| F05.5 — Texto en cuadros y visualizaciones | Empezar por cuadros de texto DrawingML soportados; luego charts/SmartArt, localizando texto en partes relacionadas y cachés. Reconciliar fuente de datos/cachés si un gráfico usa workbook. Separar alt text, etiquetas y números. | Texto no queda en el idioma fuente, visualización sigue abriendo y su geometría/datos no cambian salvo texto autorizado. |
| F05.6 — Estructuras editoriales complejas | Bookmarks, content controls, comentarios y revisiones requieren identificar límites/protección y una política explícita para el texto visible. No aceptar un documento con seguimiento activo solo ignorando `w:ins`/`w:del`. | Archivo abre sin reparación; marcadores y protección conservados; política de revisiones verificable. |

Texto dentro de una imagen rasterizada necesitaría OCR y edición de imagen: no incluirlo bajo «SmartArt soportado». Imágenes flotantes, VML, Strict OOXML y documentos con macros se mantienen fuera hasta un corte y corpus propios. Priorizar F05.1–F05.3, pero no marcar la prioridad de visualizaciones terminada antes de F05.5.

**Aceptación de cada corte:** importar desde GPUI, traducir todo el contenido esperado, confirmar, exportar y abrir en Word real sin reparación. Comparar textos por parte, tablas/celdas, imágenes, links, secciones y relaciones; partes ajenas al cambio conservan payload. Renderizar páginas afectadas, permitiendo reflow del idioma destino. Validar también destinos largos, no latinos y caso de rechazo recuperable. Usar directorios nuevos y originales inmutables.

### F06 — QA global, perfiles y corrección ortográfica

**Dependencias:** F00/F03/F04. **Archivos:** `src/qa.rs`, modelo/storage/worker, panel QA y filtros GPUI.

1. Evolucionar `QaIssue`: regla estable, severidad, segmento, revisión evaluada y rango cuando proceda. Separar errores de integridad/exportabilidad de advertencias lingüísticas. Cada perfil selecciona reglas y parámetros; registrar su versión en el informe.
2. Reutilizar `qa::check` sobre texto visible y añadir tags, terminología, consistencia de fuentes repetidas con destinos diferentes, longitud y reglas numéricas según locale. Separar la heurística de puntuación de errores estructurales: convenciones de idioma distintas no implican traducción incorrecta.
3. Ejecutar documento/proyecto por páginas fuera del render, con cancelación y navegación al issue. El informe identifica snapshot o revisiones evaluadas; las filas editadas invalidan su resultado. Un análisis interrumpido se muestra incompleto, nunca como «QA aprobado».
4. Perfiles exportables sin rutas privadas; excepciones justificadas ligadas a regla/segmento/revisión. Añadir regex solo si hay demanda: motor sin backtracking exponencial, límite de patrón/input y errores de compilación comprensibles; revisar primero las dependencias disponibles.
5. Corrector ortográfico por proveedor local: investigar Windows Spell Checking API antes de añadir motor/diccionarios. Mantener lenguaje elegido, excepciones de nombres propios y disponibilidad por idioma. Si el diccionario no existe, mostrar «no evaluado», sin cambiar texto automáticamente.
6. Filtros por estado, bloqueado, origen e issues, combinados con búsqueda. Resumen separado: con destino, confirmados, aprobados y errores pendientes. El significado del progreso se decide y se etiqueta, sin redefinir silenciosamente el porcentaje existente.

**Aceptación:** QA encuentra un número alterado, un tag perdido, término prohibido y repetición inconsistente en filas no activas; clicar issue abre la fila correcta. Corregir/incrementar revisión invalida el issue anterior. Cancelar produce informe incompleto; las excepciones de un perfil no afectan otro.

### F07 — Segmentación lingüística, split y merge

**Dependencias:** F00/F04; ampliar antes los exportadores que dependan de igualdad de conteos. **Archivos:** nuevo `src/segmentation.rs`, modelo/storage, `src/formats.rs`, `src/formats/docx.rs`, worker y GPUI.

1. Investigar reglas y fixtures por idioma. Evaluar Unicode sentence boundaries como base, con excepciones declaradas para abreviaturas, decimales e iniciales. SRX sería importación de reglas explícita, no una afirmación de que cualquier segmentador Unicode equivale a Trados.
2. Mantener contenedor original y concatenación/recomposición separadas de segmentos editoriales. Los exportadores actuales comparan cantidad de targets y unidades/slots: cambiarlos junto al nuevo mapeo, antes de habilitar split en UI.
3. Propuesta de persistencia: cada unidad editorial tiene ID estable y vínculo al contenedor; split crea IDs descendientes conservando trazabilidad y el original. Guardar límites/delimitadores para recomponer TXT y párrafos Word sin inventar espacios ni duplicar saltos.
4. En regiones con formato, no dividir un código atómico; si una oración cruza una región pareada, el mapeo por spans recompone el wrapper original. No resolverlo clonando tags con el mismo ID en segmentos independientes. Aplazar ese caso con explicación si el corte inicial no lo soporta.
5. Merge inicialmente solo dentro del mismo contenedor/documento y sin locks incompatibles. No fusionar celdas ni párrafos estructuralmente. Ante destinos existentes, previsualizar distribución/concatenación y exigir decisión humana; no inferir dónde se divide la traducción.
6. Invalidar aprobación, contexto, QA y aprendizaje de unidades reemplazadas dentro de la operación. Undo restaura IDs, límites y contribuciones; referencias externas permanecen en su contenedor. XLIFF mantiene segmentación recibida salvo una operación soportada por el adaptador.

**Aceptación:** «Dr. Smith paid 3.50. Thank you.» produce límites correctos según reglas configuradas; split/merge seguido de undo y reinicio preserva texto/IDs y exporta Word/TXT con estructura original. Un corte dentro de imagen/tag o a través de dos celdas se rechaza antes de cambiar datos.

### F08 — Repeticiones, análisis, plantillas y pretraducción

**Dependencias:** F00/F01/F02/F04; F07 para análisis por oración. **Archivos:** storage/tm/worker; crear `src/batch.rs` cuando agrupar estas operaciones reduzca duplicación; UI de proyecto.

1. Indexar repeticiones por fuente, idiomas y firma compatible de códigos, conservando el texto bruto. Separar repetición dentro del documento y entre documentos; no agrupar solo por texto normalizado como si fuera una equivalencia exacta.
2. Propagación propuesta y opcional: mostrar destinos afectados y excluir bloqueados, aprobados o con contenido humano salvo elección explícita. Aplicar como borradores con procedencia del segmento origen, grupo de undo y revisiones esperadas. No copiar IDs de tags entre contenedores.
3. Análisis de esfuerzo: segmentos/palabras, repetidos, exact, contexto y bandas fuzzy declaradas. Conteos deben ser disjuntos o explicar su solapamiento; reportar método de palabras por idioma y limitaciones de recall. Fijar versión de documento y recursos; invalidar análisis cuando cambien.
4. Pretraducción primero con TM: umbral, memorias, reglas de empate y selección de destinos vacíos. Mostrar preview y dejar resultados Draft. En un primer corte, texto plano únicamente; habilitar códigos tras remapeo F04. MT por lotes es una ampliación distinta, con coste y política de red propia.
5. Calcular candidatos en snapshot/propuesta y hacer commit tras revalidar revisiones. Una selección grande debe tener límites; hasta diseñar aplicación por bloques con agrupación durable y recuperación, rechazar una operación que exceda el límite. Cancelar antes del commit no aplica cambios; no anunciar rollback total si se eligió un modo incremental.
6. Plantillas reutilizan idiomas, recursos, filtros, perfil QA y políticas implementadas. Al importarlas, resolver recursos faltantes sin inventarlos. No crear un workflow configurable genérico.

**Aceptación:** repeticiones vacías reciben propuesta y quedan Draft; un lock y una traducción humana elegida para conservar permanecen intactos. Una operación se deshace por completo tras reiniciar; un conflicto de revisión se informa sin sobreescribir. Los conteos del análisis se reconcilian con el total importado.

### F09 — Contexto TM, concordancia y sugerencias parciales

**Dependencias:** F02/F04 y límites de segmentación F07; integrar rendimiento F17. **Archivos:** `src/tm.rs`, modelo/storage, edición y panel de sugerencias.

1. Guardar contexto estructural y vecino de las TU aprendidas. Propuesta inicial: fuente actual exacta, fuente/traducción previa exactas y misma firma estructural; solo entonces etiquetar «coincidencia de contexto». El primer segmento o la ausencia de contexto no se convierte en CM por defecto.
2. Tipar clase de match y distinguir score lexical, penalizaciones, prioridad y confianza contextual. Usar contexto para ordenar, sin convertir la suma de prioridades en una similitud ficticia superior a 100 %.
3. Presentar diferencias source-source a nivel de palabras y códigos compatibles, con fallback de caracteres para idiomas apropiados. La aplicación del destino sigue siendo humana; no renderizar contenido de TM como HTML ejecutable.
4. Concordancia origen/destino y modo frase/token explícito, filtros por memoria y resaltado. Para destino, añadir índice de búsqueda correspondiente. Reusar búsqueda literal y FTS existentes; morfología/fuzzy concordance se incorpora por idioma tras demostrar utilidad.
5. AutoSuggest primer corte desde termbases, AutoText del usuario y candidatos recuperados. Aceptación por acción diferenciada que no capture navegación normal ni composición IME. Resultados llevan versión de consulta, segmento, revisión y recursos; descartar respuestas viejas.
6. Fragmentos/reparación fuzzy requieren evidencia de correspondencia bilingüe. No usar substrings parecidos como traducción confirmada. Empezar con spans respaldados por términos o alineaciones conocidas; medir precisión sobre corpus revisado. Reparar solo entidades con equivalencia inequívoca y mostrar diff; resto permanece como referencia.
7. Reutilización de versiones equivalente al objetivo de PerfectMatch: comparar documentos originales y bilingües previos por contenedor, texto, código y contexto; generar mapa de correspondencia antes de aplicar. Coincidencias ambiguas o texto cambiado quedan para revisión. Mantener procedencia y no transferir aprobación sin política explícita y correspondencia demostrada.

**Aceptación:** misma frase en dos contextos muestra variantes correctas; sin contexto no aparece CM; concordancia destino encuentra la expresión real. Un resultado antiguo no entra en el nuevo segmento. La actualización de un documento conserva traducciones inequívocas y señala cambios; una reparación propuesta nunca corrompe tags.

### F10 — Alineación para crear memorias

**Dependencias:** F02/F04/F07. **Archivos:** nuevo `src/alignment.rs`, modelo/worker/storage y vista GPUI específica.

1. Importar pares de documentos usando los adaptadores existentes y el mismo segmentador. Registrar nombres, idiomas, hashes/identidad de fuentes y configuración usada. No modificar los documentos ni insertar TU mientras se calcula el alineamiento.
2. Reusar un alineador mantenido si cumple licencia, Rust/nativo y necesidad. Si no existe uno adecuado, implementar un primer algoritmo acotado de longitudes con anclas fiables; documentar techo, incertidumbre y ventana de cálculo. Evitar comparación cuadrática de documentos completos.
3. Resultado propuesto con conexiones 1:1, 1:n, n:1 y omitidos; confianza heurística no equivale a porcentaje de calidad de traducción. Vista origen/destino permite mover enlaces, dividir/fusionar y excluir contenido.
4. Exportar/importar a memoria únicamente conexiones aprobadas por el traductor. Deduplicar conservando procedencia y metadata de revisión; los códigos requieren correspondencia válida o quedan fuera con diagnóstico.

**Aceptación:** una traducción con párrafo omitido y dos frases fusionadas no desplaza silenciosamente todo el archivo. El traductor corrige conexiones y solo las aprobadas aparecen en TM. Cancelar conserva fuentes y memoria; conteos de conexiones revisadas e importadas se reconcilian.

### F11 — XLIFF y TMX con códigos e interoperabilidad declarada

**Dependencias:** F04; F07 si se permite editar subsegmentación. **Archivos:** `src/formats.rs` — separar adaptadores al crecer —, modelo/storage y tests de formatos.

1. Extender XLIFF 1.2 por subconjuntos: códigos `g/x`, luego `bpt/ept/ph/it` con semántica validada; `seg-source` y `mrk` con IDs, estados y vínculos conservados. Guardar nombres expandidos de XML, no depender de prefijos concretos. Especificación: [OASIS XLIFF 1.2](https://docs.oasis-open.org/xliff/v1.2/cs01/xliff-core.html).
2. Preservar envelope, notas, extensiones y skeleton; modificar solo destinos/estados admitidos. Probar namespaces, varias unidades/archivos y códigos anidados antes de aceptar cada variante. Si el documento solo puede exportarse como bilingüe, indicarlo; no prometer destino nativo sin un skeleton soportado.
3. TMX inline es un adaptador distinto: identidad, pares, metadata raw y remapeo a cada documento. Export/reimport con códigos no debe degradar markup a caracteres literales. Mantener importación streaming/cancelable y límites.
4. XLIFF 2.x necesita parser/modelo de versión separado: no procesar como 1.2 cambiando un atributo. Elegir primera versión objetivo y matriz de elementos, consultar su norma y fixtures. Las demás se rechazan hasta demostrar su soporte.
5. Documentar compatibilidad por versión+perfil+operación: importar, editar, preservar metadata, exportar bilingüe y generar destino. «XLIFF compatible» sin esta matriz no es un criterio de salida.

**Aceptación:** el corpus admitido round-tripea texto, códigos, estados y metadata; el export abre en el consumidor de referencia autorizado. Un documento fuera del perfil se rechaza sin proyecto parcial. Comparar conteos fuente/destino y que cada ID externo siga identificando la misma unidad.

### F12 — Revisión editorial y Word bilingüe de ida y vuelta

**Dependencias:** F00/F04/F06; F11 para intercambio de estados avanzado. **Archivos:** modelo/storage/worker, vista de revisión; nuevo `src/review.rs` y adaptador de documento de revisión cuando sean necesarios.

1. Mantener confirmación de traducción y estado editorial separados: sin revisar, aprobado o requiere cambios. Comentarios referencian IDs estables y revisiones; autor local es metadata de atribución, no autenticación de equipo.
2. Edición del target invalida aprobación y resultados QA asociados. Aceptar/rechazar un cambio revisado pasa por comandos revisionados; historial técnico no sustituye trazabilidad editorial.
3. Generar un Word bilingüe propiedad de LumenCAT, con origen protegido, destino revisable e identificador estable de cada unidad, revisión base y versión de esquema. Reutilizar ZIP/XML si basta; no intentar importar ese archivo mediante el parser DOCX de documentos fuente, que tiene otro contrato.
4. Importación de revisión detecta IDs desconocidos/duplicados, source alterado, destino inválido, eliminación/reordenado de filas y conflictos con trabajo posterior. Presentar diff de destino y conflictos antes del commit. No confiar solo en el número de fila o un hash global.
5. Aplicar solo correcciones aceptadas, con un grupo de undo; preservar códigos en su forma revisable y validarlos al retornar. El Word de revisión no debe incluir originales completos, memorias o credenciales innecesarios.
6. Primer corte pide devolver cambios aplicados en el texto de destino. Interpretar revisiones pendientes de Word y comentarios de Word es un corte posterior con política explícita; no ignorar markup de seguimiento como si ya estuviera resuelto.

**Aceptación:** exportar revisión, corregir destino en Word real e importar actualiza las unidades correctas. Si LumenCAT editó una de ellas en paralelo, se detecta conflicto y no se sobreescribe. Fuente alterada o IDs duplicados impiden aceptación; undo y reinicio conservan el resultado editorial coherente.

### F13 — SDLXLIFF, paquetes y retornos para agencias

**Dependencias:** F11; F12 para los flujos editoriales que el paquete requiera. **Archivos propuestos:** adaptador SDLXLIFF y `src/packages.rs`, modelo/worker/storage.

1. Exploración obligatoria con proyectos/paquetes autorizados, licencia de herramientas de referencia y documentación pública. Inventariar extensión, versión, manifiesto, archivos, relaciones, recursos y tareas; no basarse en que el contenedor es un ZIP. Separar apertura de paquetes de creación/retorno y sus requisitos comerciales.
2. SDLXLIFF conserva IDs, estados, extensiones, skeleton y dependencias. Mapear únicamente semánticas conocidas; conservar metadata no interpretada cuando sea seguro. Señalar recursos que no puedan usarse en LumenCAT en lugar de fingir que fueron cargados.
3. Abrir paquetes con límites ZIP, nombres/rutas normalizados, detección de duplicados y extracción en directorio aislado. No seguir rutas absolutas, `..`, enlaces ni instrucciones del manifiesto; no ejecutar contenido. Registrar la tarea y lista exacta de archivos entregables.
4. Recursos `.sdltm`/`.sdltb` no se vuelven compatibles por estar dentro del paquete. Definir conversiones autorizadas o importaciones TMX/TBX auxiliares y cómo afecta esto al trabajo. Bloquear el flujo si un recurso obligatorio no puede conservarse para retorno.
5. Crear retorno con los archivos de la tarea y metadata requerida; no empaquetar la base local completa. Mantener bloqueos/estados y comprobar todas las dependencias antes de generar.
6. Probar el retorno en Trados con un proyecto preparado para la prueba: importación del retorno y generación del destino sin reparación ni pérdida de contenido. Si no hay instalación/licencia/fixtures, dejar el paquete marcado «experimental/no validado», aunque el ZIP/XML pase.

**Aceptación:** una agencia puede abrir el retorno del corpus declarado en Trados y generar el documento final conservando contenido/estructura. Paquete hostil o dependencia ausente se rechaza con diagnóstico; un retorno no mezcla tareas ni proyectos. No anunciar paridad universal con paquetes Trados.

### F14 — Otros formatos y filtros de extracción

**Dependencias:** F04/F07; filtros de proyecto F01. **Archivos:** adaptadores en `src/formats/`, enum de formato, worker y guía de compatibilidad.

Orden propuesto: PPTX y XLSX según encargos reales, luego HTML/XML de perfil definido y, solo con demanda, IDML u otros. Cada adaptador debe extraer→editar→recomponer→abrir en aplicación de referencia; contar tipos de archivo no es el objetivo.

- **PPTX:** texto de shapes/tablas/notas por partes, conservando runs, hyperlinks y campos. Filtrar diapositivas/notas/objetos ocultos según política visible. Charts y SmartArt requieren relaciones y fuentes de datos, no un `replace` en todos los `a:t`.
- **XLSX:** celdas de texto y shared strings, excluyendo fórmulas/valores numéricos por defecto. Si varias celdas comparten una entrada y solo algunas se traducen, crear una entrada nueva para las seleccionadas y actualizar referencias; no modificar indirectamente celdas bloqueadas/omitidas. Rich text, comentarios, hojas ocultas y gráficos necesitan políticas/cortes propios.
- **HTML/XML:** perfiles que identifican texto/atributos traducibles y elementos excluidos. Preservar markup y espacios relevantes; scripts, estilos y recursos activos no se ejecutan para inspeccionar. Rechazar contenido incrustado fuera del perfil.
- **IDML/PDF:** estudio de licencia, filtros disponibles y fidelidad antes de elegir biblioteca. PDF es formato de presentación con reconstrucción difícil; OCR y layout son proyectos separados, sin prometer round-trip por extraer texto.

Reutilizar las dependencias ZIP/XML existentes cuando cubran el formato; evaluar una biblioteca mantenida antes de escribir un parser de alto nivel. Filtros tienen versión, configuración persistida y preview de cobertura/omisiones. Un preview inicial puede mostrar ubicación del texto; una maquetación completa depende de renderizador externo y no es requisito para aceptar un adaptador básico.

**Aceptación:** conteos de unidades y textos fuente concuerdan con el corpus; aplicaciones reales abren sin reparación; partes omitidas y datos numéricos/fórmulas siguen iguales. Mostrar claramente qué partes no se traducen. No habilitar una extensión hasta que exista exportación comprobada.

### F15 — MT y asistencia LLM contextual bajo control humano

**Dependencias:** F01/F02/F03/F04; aplicar límites/colas F17. **Archivos propuestos:** `src/assistance.rs` y adaptadores de proveedor; modelo/worker para aceptación, GPUI para contexto/propuestas.

1. Implementar la decisión 005: petición → vista de contexto → proveedor → propuesta/diff → aceptación humana → comando. El proveedor recibe un DTO limitado y no una referencia a storage. La propuesta incluye proyecto, segmento, revisión base, versión de recursos y procedencia.
2. Contexto mínimo: segmento, vecinos seleccionados, idiomas, términos relevantes y mejores TU. Permitir al traductor ver qué se enviará. Ampliar contexto de párrafo/documento con un presupuesto explícito, sin enviar el original completo por defecto ni convertir imágenes adjuntas en una capacidad implícita.
3. Primer contrato: proponer traducción y explicar una selección. Local-only, IA desactivada y veto cloud del proyecto se verifican en el despacho. Proveedores OpenAI-compatible/Ollama son candidatos ya previstos en MVP; investigar sus APIs y modelos vigentes antes de añadirlos. No fijar endpoints/modelos/costes a partir de este plan.
4. Red/inferencia separada del worker escritor en una cola acotada, con timeout, cancelación, límites de respuesta y progreso. Mantener la edición y autosave operables mientras responde el proveedor. Error o cancelación conserva destino; una respuesta tardía no se aplica en otra fila.
5. Validar códigos y formato de respuesta. Mostrar diff con origen del proveedor; aceptar produce Draft y es undoable. El modelo no confirma, aprueba, modifica TM/termbase ni exporta. Editar manualmente después actualiza la procedencia sin perder trazabilidad de la propuesta.
6. Credenciales en almacén del SO, fuera de `.lcat`, prompts registrados o exportes. Logs guardan eventos/tiempos y IDs, no texto sensible ni claves. Consultas conectadas muestran política/coste cuando el proveedor lo permita; no inventar un importe exacto sin datos de facturación.
7. Búsqueda web/contextual en un segundo corte: solo a petición, consulta minimizada visible y resultados con fuentes. Contenido del documento/web es dato y no instrucciones operativas para la app. Una explicación puede citar referencias, pero no escribir autónomamente cambios en el proyecto.
8. MT clásica usa el mismo contrato de propuesta/aceptación. MT por lotes requiere diseño de presupuesto, rate limits e idempotencia separado de pretraducción TM; no activar despacho de pago como prueba implícita de desarrollo.

**Aceptación:** proveedor simulado devuelve propuesta; destino permanece intacto hasta aceptar, luego Draft y undo funcional. Una revisión obsoleta se rechaza, IA desactivada/veto cloud impide despacho y un timeout no bloquea autosave. La validación real del proveedor exige configuración y autorización de consumo, separada de pruebas locales.

### F16 — API de automatización y extensibilidad gradual

**Dependencias:** estabilizar F00/F04 y al menos dos consumidores reales — por ejemplo análisis y paquetes — antes de publicar contrato.

1. Primer corte CLI Rust sobre los mismos servicios de dominio: inspeccionar proyecto, analizar, importar/exportar y tareas existentes. Resultados estructurados y códigos de salida; errores distinguen inválido, conflicto, cancelación y formato no soportado. No duplicar lógica en scripts.
2. Automatización escritora usa el mismo bloqueo de proyecto, validación, revisión e historial. Una CLI externa no puede escribir mientras GPUI es dueña del proyecto: ofrecer servicio de comandos de la instancia o exigir cierre, según necesidad demostrada. No acceder directamente a tablas como API pública.
3. Definir versión y capacidades del contrato; documentar límites y fixtures. DTO serializable solo cuando exista consumidor; dependencias transitivas del lockfile no son imports disponibles del crate y deben declararse directamente si se usan.
4. Extensiones internas mediante interfaces concretas para filtros/proveedores/verificadores ya existentes. No cargar bibliotecas nativas arbitrarias dentro del proceso principal por defecto. Plugins distribuidos podrían usar procesos aislados y mensajes versionados, pero se diseñan únicamente ante demanda y revisión de licencia/distribución.

**Aceptación:** una tarea automatizada produce el mismo documento/análisis que la operación de UI; una escritura externa con el proyecto abierto se rechaza sin corrupción. API de versión incompatible informa el problema; no deja un proyecto parcial si falla.

### F17 — Fluidez del editor, escalabilidad y accesibilidad

**Dependencias:** primer corte independiente; F00 para invalidación común. **Archivos:** `src/gpui_app/mod.rs`, `components.rs`, `input.rs`, worker/storage, benchmark y verificación UIA.

1. Sustituir construcción de `0..count` por una lista virtualizada nativa disponible en el commit GPUI fijado. Verificar su API en el código/declaraciones locales antes de elegirla; no escribir un virtualizador propio por desconocer la biblioteca. Rango visible+overscan pide páginas fuera del render y la cache tiene límite.
2. Preservar navegación por ID/ordinal, fila activa y foco al reciclar elementos. Previews de filas acotados; editor activo mantiene el texto completo. Requests por documento/generación descartan páginas viejas al cambiar de archivo.
3. Completar selección parcial por ratón, posición de caret, clipboard, graphemes, composición IME y texto RTL; convertir índices UTF-16/UTF-8 solo en fronteras correctas. No cortar emoji, combining marks ni badges. Probar atajos físicos en el transporte autorizado, sin dar por probado Ctrl+Enter con Invoke.
4. Revisar roles/nombres/estados UIA en elementos virtualizados y cómo exponer unidades no visibles mediante navegación. El verificador actual cuenta filas del snapshot: adaptarlo para comprobar contenido/navegación reales, no exigir todas las filas renderizadas ni un tipo de implementación.
5. Medir con datasets conocidos y corpora reales: apertura, desplazamiento, input, autosave, match y cancelación. Registrar hardware/build/tamaño, warm/cold con protocolo, p50/p95, RAM y recall donde corresponda. Reutilizar objetivos aspiracionales de MVP, sin convertirlos en resultados.
6. Si el worker queda bloqueado por consultas grandes, medir primero. Después introducir lector SQLite/TM separado con snapshot coherente o dividir jobs, manteniendo un solo escritor y colas acotadas. No resolver lentitud lanzando tasks ilimitadas.

**Aceptación:** recorrer un documento de 100 000 segmentos no crea una fila por segmento; memoria/cache se mantiene acotada y se puede editar/guardar sin esperar la carga completa. IDs/foco correctos al saltar inicio/fin, IME/RTL y selección cumplen escenarios reales. Medir latencias y declarar incumplimientos, no solo tiempo total del benchmark CLI.

### F18 — Colaboración y nube, solo si se decide ampliar el producto

**Dependencias:** F00/F12/F16 y una decisión de producto explícita. **Estado:** opcional, fuera de las entregas necesarias para la CAT local.

Empezar por paquetes/revisión e intercambio explícito; no sincronizar el archivo SQLite/WAL mediante carpetas cloud mientras está abierto. Si se requiere colaboración concurrente, diseñar protocolo por operaciones/IDs/revisiones, conflictos y permisos, con servicio central y modelo de costes. Memorias compartidas, asignaciones, login/roles y edición en navegador son iniciativas separadas: requieren operación del servicio, privacidad y verificación de aislamiento.

**Aceptación futura:** dos usuarios pueden editar según sus permisos, se detectan conflictos y una desconexión conserva trabajo local recuperable. Ningún estado central se anuncia «sincronizado» hasta confirmar la recepción durable. No elegir stack cloud ni proveedor dentro de esta planificación.

## Trazabilidad de brechas y definición de terminado

| Brecha original | Paquetes que la cubren |
|---|---|
| TM al confirmar | F00, F01, F02; F04/F11 para inline |
| Terminología | F03, F06 |
| Segmentación y autopropagación | F04, F07, F08 |
| TM contextual/diferencias/concordancia | F02, F09 |
| AutoSuggest/fragmentos/reparación | F03, F09 |
| Análisis/pretraducción/plantillas | F01, F08, F16 |
| Alineación | F07, F10 |
| Formatos/filtros | F04, F05, F11, F14 |
| QA configurable | F03, F04, F06 |
| Revisión/retorno Word | F06, F12 |
| Paquetes Trados | F11, F13 |
| Reutilización de versiones | F09; no se reutiliza código ni marca PerfectMatch |
| MT/LLM | F15 |
| SDK/plugins | F16; sistema de plugins distribuido sujeto a demanda |

Una entrega exige estas evidencias, con estado PASS/FAIL/PENDIENTE y alcance concreto:

1. Prueba de comportamiento del paquete sobre proyecto temporal: entrada real → comando → persistencia → resultado/export. Reutilizar `tests/storage.rs`, `recovery.rs`, `tm_retrieval.rs`, `formats.rs`, `docx.rs`, `qa.rs`, `replacement.rs` y `search_cancel.rs`; añadir un archivo nuevo solo para una responsabilidad nueva. No tests que inspeccionen strings del código o reproduzcan el algoritmo como expectativa.
2. Test de fallo pertinente: cancelación, revisión obsoleta, lock, falta de código o rollback; probar el riesgo real del cambio, no una lista infinita de combinaciones. Migrations/history/formatos requieren su prueba de recuperación o round-trip.
3. Flujo nativo GPUI para capacidades visibles; UIA y operaciones background primero, con IDs y lecturas frescas. Si una acción precisa otro transporte, declarar el alcance no probado hasta poder ejecutarla. Un test de storage no acredita la UI.
4. Formatos: aplicación de referencia abre el exportado y no pide reparación; comparar contenido/estructura y render cuando la fidelidad visual esté afectada. Word tiene helpers existentes; otros formatos necesitan su verificador específico. Una reimportación usando el mismo parser es evidencia adicional, no prueba independiente completa.
5. Actualizar guía, matriz de compatibilidad y decisión afectada con lo observado. Logs en `logs/tests/`, captures/exportes en `output/verification/<corrida>/`, trabajo temporal en `.cache/verification/<corrida>/`. No nuevas salidas sueltas en raíz.

### Comandos de verificación reutilizables

Son comandos para las futuras implementaciones, no checks ejecutados por esta actualización documental. Desde la raíz, seleccionar primero el test del scope tocado; antes de cerrar una entrega del núcleo compartido, pasar los checks del proyecto:

```powershell
cargo test --locked --test storage
cargo test --locked --test tm_retrieval
cargo test --locked --test docx
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release --bin lumencat --bin lumencat-gpui
```

Para Word, usar las opciones reales de `scripts/utils/verify_word.ps1`: `-Source`, `-Translated`, `-OutputDirectory`. `verify_docx` recibe un directorio nuevo de copias; no repetir sobre uno con exportes previos como fuentes. `verify_gpui_uia.ps1` requiere PID, WindowId y carpeta de salida. `scripts/utils/validate.ps1 -Release` actualmente construye `lumencat`/`benchmark`, por lo que no sustituye una build explícita del binario GPUI. No reemplazar el EXE de uso diario durante verificación: probar un artefacto/copia aislado.

## Primeras unidades de trabajo ejecutables

Dividir en cambios revisables que puedan quedar terminados sin activar toda la fase:

1. **F00.1:** backup coherente + migración mínima probada sobre copias v1. Sin modificar proyectos existentes del usuario ni cambiar todavía confirmación.
2. **F01.1:** selección/persistencia de idiomas GPUI y comprobación de conflicto con XLIFF. Adaptar legacy solo donde comparta la regla.
3. **F00.2:** grupos de historial y resultado común de invalidación, preservando undo/redo anterior; preparar el efecto TM dentro de la misma operación.
4. **F02.1:** colecciones de memoria + migración de las TU actuales + selección de memoria de escritura. Validar import/export antes de aprender.
5. **F02.2:** comando de confirmación y contribución aprendida de texto plano, con deduplicación, corrección, suspensión y undo/redo. Códigos aún excluidos con aviso hasta F04.

F17.1 puede adelantarse si medir el grid revela que impide estas validaciones. Tras estas unidades, entregar F03 y F04/F05.1–F05.3; mantener F05.5 visible en el backlog de la prioridad Word, sin confundir un primer corte con cobertura completa.

### Decisiones que requieren evidencia antes de cerrar diseño

- **Segmentación:** idiomas iniciales, reglas/abreviaturas y corpus; biblioteca elegida solo tras verificar capacidades y licencia. No escribir un segmentador universal de cero.
- **Aprendizaje:** el plan propone suspender la contribución del segmento al editar y conservar variantes importadas. Validar este comportamiento con un traductor antes de fijarlo como contrato público; es una propuesta concreta, no un bloqueo para preparar la implementación.
- **Intercambio:** disponibilidad de fixtures autorizados y Trados para validar retornos. Sin ello puede avanzarse XLIFF abierto, pero no cerrarse F13 como compatible.
- **Proveedores:** locales/remotos, privacidad, cuotas y coste real; no llamar servicios pagados ni instalar motores/modelos por el hecho de aparecer en el plan.
- **Formatos siguientes:** encargos/corpus que justifican PPTX frente a XLSX y cobertura requerida. Preservar primero la prioridad Word ya declarada.

No se asignan fechas ni se declara «paridad Trados» al completar un porcentaje de tickets. Cada capacidad se considera implementada únicamente para su matriz de formatos, idiomas, operaciones y evidencia de comportamiento.
