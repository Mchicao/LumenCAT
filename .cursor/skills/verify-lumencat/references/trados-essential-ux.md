# Trados Studio: UX esencial de escritorio para comparar con LumenCAT

**Referencia:** Trados Studio **2024 SR1**, documentación oficial RWS, publicación `1187677`. **Consulta:** 2026-10-04. No se fija una actualización acumulativa (CU) ni un build instalado. La web comercial actual anuncia **Studio 2026**; sus novedades no se atribuyen a 2024 SR1. [versión] [actual]

**Alcance:** el recorrido y E01–E15 son criterios documentales, no pruebas aprobadas. La sección «Observación y comparación» registra la pasada live por separado. «Esencial» es una prioridad para trabajo individual, no una categoría comercial de RWS. Parecido visual no demuestra paridad funcional, calidad lingüística ni interoperabilidad.

## Recorrido documentado

1. **Entrar por archivo o proyecto.** `Translate Single Document` permite traducir, verificar y generar un documento. `New Project` añade idiomas, recursos, terminología y preparación; admite plantillas. La interfaz separa `Projects`, `Files`, `Editor` y otras vistas, con cinta de comandos y paneles reorganizables. `Manager`, disponible en Studio 2024, reúne proyectos, archivos e informes, pero no elimina las vistas separadas. [archivo] [proyecto] [vistas] [manager]
2. **Preparar y estimar.** La preparación convierte contenido traducible a `.sdlxliff` y crea versiones por idioma destino. La secuencia predeterminada es `Prepare without Project TM`; no confundir «sin TM de proyecto» con «sin acceso a memorias». Se pueden crear memorias locales y configurar recursos. `Analyze Files` mide coincidencias/repeticiones; `Pre-translate Files` aplica recursos con umbral, política de sobrescritura y opciones de confirmación/bloqueo. Son capacidades del flujo, no pasos obligatorios en toda traducción. [preparación] [recursos] [análisis] [pretraducción]
3. **Traducir en contexto.** El editor muestra origen a la izquierda y destino a la derecha, segmentados, con estados y bloqueos. Paneles asociados muestran coincidencias, concordancia, términos, comentarios y mensajes. `Ctrl+Enter` confirma según el modo activo y avanza al siguiente segmento no confirmado, omitiendo bloqueados. Confirmar puede actualizar la memoria y ejecutar verificación del segmento: **guardar no equivale a confirmar**. El porcentaje TM representa coincidencia, no aprobación humana. [editor] [paneles] [confirmación] [estados]
4. **Verificar y revisar.** Hay tres alcances distintos: segmento al confirmar, archivo activo mediante `Review > Verify`, y varios archivos mediante `Batch Tasks > Verify Files`. QA incluye omisiones, inconsistencias, puntuación, números y otros controles; terminología y etiquetas tienen verificadores propios. Muchas opciones QA están desactivadas inicialmente; etiquetas se verifican por defecto. Abrir para revisión permite aprobar/rechazar y usar comentarios/control de cambios; no exige colaboración en nube. [verificación] [qa] [qa-lote] [revisión]
5. **Entregar un destino, no solo texto.** Guardar el bilingüe conserva `.sdlxliff`. `File > Save Target As` genera el formato original; `Generate Target Translations` lo hace por lote desde los destinos bilingües —por ejemplo, Word → Word—. La evaluación debe comprobar el artefacto generado, no solamente que aparezca un botón de exportación. [guardado] [destino] [destino-lote]

## Matriz de criterios observables

**E:** capacidad esencial para esta comparación individual. Las pruebas son propuestas de auditoría, no recorridos ejecutados ni afirmaciones sobre LumenCAT. No requieren copiar nombres, posición de paneles o atajos de Trados.

| ID | Capacidad y referencia documental | Prueba observable propuesta |
|---|---|---|
| E01 | Archivo/proyecto, idiomas y recursos. [archivo] [proyecto] [recursos] | Crear un proyecto desechable con dos archivos y un par lingüístico; identificar archivo activo y recursos asociados. Abrir también un archivo por la ruta simple. |
| E02 | Preparación y análisis con bandas fuzzy y repeticiones entre archivos. [preparación] [análisis] | Usar corpus conocido con exacta, parcial, repetición y segmento nuevo; consultar informe y reconciliar recuentos, indicando opciones de análisis y alcance. |
| E03 | Pretraducción controlada. [pretraducción] | Aplicar un umbral explícito; comprobar qué destinos se rellenan, qué traducciones existentes se conservan y que un bloqueo no se sobrescribe. No asumir confirmación automática universal. |
| E04 | Editor bilingüe segmentado y navegación. [editor] [paneles] | Editar el destino de una fila, cambiar de segmento/archivo y volver; comprobar alineación, contenido y foco. Ver origen y contexto sin perder la edición. |
| E05 | Guardado y trabajo pendiente. [guardado] | Distinguir archivo modificado de guardado; guardar, cerrar y reabrir. Comparar destinos y estados recuperados, sin confundir el bilingüe con la entrega nativa. |
| E06 | Confirmación, edición posterior y actualización TM configurable. [confirmación] [estados] | Confirmar un borrador; observar estado y navegación. Con actualización TM habilitada, reutilizarlo en otro segmento/proyecto. Editar lo confirmado y comprobar que vuelve a borrador. |
| E07 | Bloqueos y progreso por archivo/proyecto. [editor] [confirmación] [progreso] | Intentar editar una fila bloqueada; comprobar que confirmar omite esa fila. Reconciliar estadísticas de confirmación con estados reales, registrando si cuentan palabras o porcentajes. |
| E08 | Etiquetas inline/estructura y formato, no meros caracteres decorativos. [etiquetas] [insertar-etiquetas] [verificación] | Insertar un par de etiquetas y un elemento aislado donde el formato lo admita; crear un error deliberado y detectarlo. Comprobar estructura/formato del destino generado. |
| E09 | Coincidencias exactas, contextuales y fuzzy con procedencia. [coincidencias] [estados] | Obtener exacta y parcial conocidas; revisar diferencias, porcentaje y memoria de origen; aplicar una candidata y editarla. Un porcentaje alto no acredita traducción correcta. |
| E10 | Concordancia sobre TM, distinta de buscar en el documento. [concordancia] | Buscar una frase parcial presente en la memoria pero ausente del documento; ver origen/destino e insertar texto seleccionado. La búsqueda por destino depende de la configuración de la TM. |
| E11 | Terminología conectada al segmento y al QA. [términos] [añadir-término] [verificación] | Asociar una base, reconocer un término del origen, insertar su equivalente y añadir otro desde el editor. Comprobar persistencia y detección de un término prohibido con el verificador habilitado. |
| E12 | Búsqueda/reemplazo y filtros de trabajo. [buscar] [paneles] [versión] | Buscar y reemplazar texto destino; filtrar segmentos y volver a la vista completa sin perder cambios. Los filtros deben expresar el alcance; no sustituyen concordancia TM. |
| E13 | QA del archivo y QA global por lote. [verificación] [qa] [qa-lote] [mensajes] | Habilitar controles explícitos e introducir omisión, cifra incorrecta, inconsistencia y etiqueta defectuosa en dos archivos, uno sin abrir. Verificar ambos, identificar incidencias, corregir y repetir; conservar informe y exclusiones. |
| E14 | Revisión separada de traducción. [revisión] [aprobación] [comentarios] | Abrir para revisión, aprobar un segmento y rechazar otro; distinguir esos estados de «traducido». Adjuntar comentario y comprobar su asociación/persistencia. Sign-off adicional solo si la política lo exige. |
| E15 | Generación de destino individual y múltiple. [destino] [destino-lote] | Generar archivos de destino desde el trabajo bilingüe guardado. Inspeccionar texto, etiquetas, formato y apertura en una herramienta compatible; comprobar rutas/idiomas y fuente original intacta. Un TXT no acredita round-trip DOCX. |

## Esencial individual frente a extras

| Clase | Qué separar al comparar |
|---|---|
| **Base local individual** | Proyecto/archivo, TM, editor, etiquetas, terminología, QA y entrega de la matriz. Revisar archivos locales, comentar y usar control de cambios no son por sí mismos funciones de nube. En **2024 SR1 no hace falta instalar MultiTerm** para usar terminología en Studio; su edición avanzada sigue siendo una herramienta diferenciada. [archivo] [revisión] [versión] [sr1] |
| **Ayudas locales / requisitos del encargo** | AutoSuggest, PerfectMatch, vista previa y sign-off pueden ampliar el flujo; no son condición universal de su recorrido mínimo. **TQA** (evaluación con criterios de calidad) requiere licencia Professional para proyectos locales o suscripción para proyectos cloud: no confundirlo con QA Checker ni con aprobar un segmento. [proyecto] [paneles] [revisión] |
| **Colaboración y nube** | Publicar/asignar en GroupShare, recursos cloud, editor online y gestión de equipos son escenarios distintos. Studio 2024 incorpora recursos cloud en procesamiento local para usuarios con suscripciones Team/Accelerate/Enterprise; eso no convierte la nube en requisito del editor local. Paquetes y gestión de permisos requieren criterios propios si el encargo los necesita. [proyecto] [manager] |
| **IA / traducción automática opcional** | AI Assistant puede proporcionar traducciones o ayudar a poseditarlas. En 2024 SR1, la integración LLM documentada usa apps y suscripción propia; la integración local Hugging Face descrita estaba en beta. Smart Review se presentó en 2024 con derecho cloud y app para escritorio. No sustituir QA determinista, confirmación o revisión humana por una puntuación IA. [sr1] [manager] |

## Observación y comparación — 4 de octubre de 2026

**Trados instalado:** proceso `Studio19`, FileVersion `19.0.0.3043`; no atribuirle automáticamente la versión de la ayuda 2024 SR1. Se condujo la instancia preexistente por Cua background: Proyectos → Archivos → sample abierto para Revisión → editor → Revisión/Verificar → Mensajes. Verificar produjo **18 incidencias**; el hash del sample quedó intacto. Se abrió el panel de resultados TM, pero no se aplicó una coincidencia. No se editó, confirmó, guardó ni exportó el sample; navegar desde una incidencia se intentó sin resultado acreditado.

**LumenCAT:** build propia de `524046f`; corrida `audit-main-gpui-20261004`. Evidencia local en `output/verification/<RunId>/` y Trados en `trados-audit-20261004/`; no se incluyen capturas ni documentos del sample en Git. Cada fila describe solo el alcance demostrado o la brecha encontrada en fuente.

| Criterios | LumenCAT en el corte auditado | Límite frente al recorrido Trados |
|---|---|---|
| E01 | GUI: documentos TXT/XLIFF/DOCX, selección, idiomas guardados y conflicto XLIFF rechazado. | Launch abre por `--project`; creación desde diálogo no conducida. Sin plantillas. |
| E02–E03 | Análisis/pretraducción por lote no implementados (`src/worker.rs`, `docs/technical/BRECHAS_TRADOS.md`, F08). | Aplicar una coincidencia individual no sustituye un lote con umbral/política. |
| E04–E05 | GUI: grid inline, edición y reapertura durable; 100.000 filas importadas, selección/render de medio y final por búsqueda. | Sin prueba de rueda manual, rendimiento comparativo, selección parcial ni colocación del caret por clic. |
| E06–E07 | GUI: vacío no confirma; confirmar avanza y aprende TM elegida; lock visible y botones deshabilitados. | Intento de escribir bloqueado no acredita rechazo: entrega sintética falló. Avanza al siguiente ordinal; progreso cuenta destinos no vacíos, no aprobación. |
| E08 | GUI: inserción de códigos DOCX, rechazo de confirmación sin códigos; exportación conserva texto/negrita y partes ajenas. | Códigos visibles como texto; no QuickPlace con candidatos ni XLIFF inline. |
| E09–E10 | GUI: exacta aprendida/importada, aplicación, concordancia y exportación TMX parseable. | Fuzzy no conducido en esta pasada; no context match ni resaltado completo de diferencias (`src/tm.rs`). |
| E11 | GUI: base/concepto prohibido, reconocimiento, `term-forbidden`, desactivar y reactivar tras reapertura. | No TBX/MultiTerm ni botón para insertar equivalente; variantes avanzadas solo núcleo. |
| E12 | GUI: búsqueda de dos filas, reemplazo y un Undo que restaura ambas. | Alcance segmento, filtros por estado y atajos no acreditados aquí. |
| E13 | GUI: QA textual del activo y QA terminológico. | Sin verificación de archivo/proyecto ni informe global; Trados sí produjo mensajes del archivo con Verificar. |
| E14 | Sin flujo de aprobación/rechazo editorial ni comentarios (`SegmentState` en `src/model.rs`). | Confirmado no significa aprobado por revisor. |
| E15 | GUI + archivos: TXT/XLIFF/DOCX exportados, texto/estructura comprobados y originales intactos. | Sin abrir en Word/Trados en esta pasada, sin entrega por lote ni paquetes SDLPPX/SDLRPX. |

**Prioridad práctica:** QA de documento/proyecto y entrada/selección fiable, después segmentación/reutilización y fidelidad Word ampliada. Si el destino comercial son agencias Trados, adelantar el intercambio bilingüe/paquetes con fixtures reales. Aprendizaje TM, idiomas, terminología local y migraciones ya existen: no repetir esas brechas históricas. Ninguna fila certifica paridad ni aptitud para una entrega de cliente.

## Fuentes primarias consultadas

Todas las referencias de ayuda siguientes corresponden a **Trados Studio 2024 SR1**, no al editor online, salvo que el propio apartado lo indique. Se leyó el contenido HTML oficial por HTTP; el extractor Markdown del portal devolvía solo títulos en algunas páginas. No se usaron artículos de terceros ni respuestas de foros.

- [Versión: cambios de 2024 SR1][versión], RWS Documentation Center.
- [Recorrido de archivo único][archivo]; [asistente de proyecto][proyecto]; [recursos de traducción][recursos].
- [Organización del escritorio][vistas]; [Editor window][editor]; [paneles del Editor][paneles].
- [Secuencias de preparación][preparación]; [Analyze Files][análisis]; [Pre-translate Files][pretraducción].
- [Confirmación][confirmación]; [columna de estados][estados]; [estadísticas de confirmación][progreso]; [guardado][guardado].
- [Translation Results][coincidencias]; [concordancia][concordancia]; [búsqueda/reemplazo][buscar].
- [Etiquetas][etiquetas]; [inserción de etiquetas][insertar-etiquetas]; [reconocimiento de términos][términos]; [alta de términos][añadir-término].
- [Alcances de verificación][verificación]; [configuración QA Checker][qa]; [QA por lote][qa-lote]; [Messages][mensajes].
- [Revisión][revisión]; [aprobación][aprobación]; [comentarios][comentarios]; [generación individual][destino]; [generación por lote][destino-lote].
- [Introducing Trados Studio 2024][manager], Trados, **2024-07-16**: Manager, cloud e IA en esa versión.
- [What's new in Trados Studio 2024 SR1][sr1], Trados, **2025-08-07**: terminología, plugins LLM y límites de beta.
- [Página comercial actual][actual], Trados, consultada **2026-10-04**: anuncia Studio 2026; se usa únicamente para delimitar vigencia.

## Límites de la comparación

- Documentación disponible ≠ recorrido ejecutado. E01–E15 siguen siendo propuestas; la observación parcial se limita a la sección anterior y a sus artefactos locales. No hay resultados comparativos de rendimiento.
- Los ajustes importan: proyecto/par lingüístico, proveedor, umbral TM, verificador activo y exclusiones deben acompañar la evidencia; no atribuir fallos o éxitos a valores predeterminados supuestos. [pretraducción] [qa]
- La ayuda de reconocimiento conserva referencias históricas a MultiTerm; para la dependencia de instalación prevalece la nota explícita de **desacoplamiento en SR1**. No extrapolar esa arquitectura a Studio 2024 inicial ni a 2026. [términos] [versión]
- Round-trip nativo, lectura de `.sdlxliff`, intercambio de memorias y paquetes Trados son contratos diferentes. Tener cuadrícula bilingüe, exportar XLIFF o mostrar «TM» no acredita compatibilidad completa. Cualquier afirmación de interoperabilidad necesita archivos reales y prueba con el consumidor/versionado correspondiente. [guardado] [destino-lote]

[versión]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/changes-for-trados-studio-2024-sr1-1230624
[archivo]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/single-file-translation-workflow-346567
[proyecto]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/creating-projects-using-wizard-workflow-555632
[recursos]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/step-4%3A-adding-translation-resources-556221
[vistas]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/screen-layout-and-functionality-338262
[editor]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/editor-window-347945
[paneles]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/editor-view-screen-layout-and-functionality-340414
[preparación]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/batch-task-sequences-for-new-projects-359245
[análisis]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/running-batch-tasks%3A-analyze-files-548236
[pretraducción]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/running-batch-tasks%3A-pre-translate-files-548281
[confirmación]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/confirming-translations-352560
[estados]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/segment-status-column-340490
[progreso]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/tracking-file-confirmation-level-331928
[guardado]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/saving-files-521099
[coincidencias]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/translation-results-window-340852
[concordancia]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/performing-a-concordance-search-354706
[buscar]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/finding-and-replacing-text-354144
[etiquetas]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/working-with-tags-354244
[insertar-etiquetas]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/inserting-tags-into-the-target-segment-354278
[términos]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/term-recognition-window-340719
[añadir-término]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/adding-termbase-entries-with-review-354644
[verificación]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/verifying-translations-359741
[qa]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/specifying-settings-for-qa-checker-359765
[qa-lote]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/generating-verification-results-with-batch-tasks-359757
[mensajes]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/messages-window-340680
[revisión]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/reviewing-files-420478
[aprobación]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/approving-translations-353714
[comentarios]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/working-with-comments-354423
[destino]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/generating-translated-files-353536
[destino-lote]: https://docs.rws.com/en-US/trados-studio-2024-sr1-1187677/running-batch-tasks%3A-generate-target-translations-548248
[manager]: https://www.trados.com/blog/introducing-trados-studio-2024/
[sr1]: https://www.trados.com/blog/whats-new-in-trados-studio-2024-sr1/
[actual]: https://www.trados.com/product/studio/
