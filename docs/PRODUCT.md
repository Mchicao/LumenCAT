# LumenCAT

<!-- impeccable:product-schema 1 -->

## Platform

Escritorio nativo en Rust con GPUI 0.2.2 como interfaz predeterminada; egui queda disponible mediante `--legacy-egui`. Windows es la plataforma comprobada. No es una aplicación web ni móvil; la portabilidad a otros sistemas no está validada.

## Users

Traductores profesionales que trabajan durante horas con documentos y memorias de traducción. Necesitan editar y revisar segmentos con fluidez por teclado, conservar el documento y decidir qué sugerencias incorporar.

## Product Purpose

Permitir traducir documentos con rendimiento, protección del trabajo y autoridad humana. La siguiente etapa prioriza traducir DOCX reales conservando contenido y formato, incluidos tablas y texto dentro de visualizaciones.

El éxito se evalúa sobre el documento entregado y el flujo real del traductor: contenido traducible cubierto, estructura conservada, cambios durables y edición ágil. Una compilación correcta o una barra de progreso completa no bastan para demostrarlo.

## Positioning

La razón propuesta para elegir LumenCAT frente a Trados u otras CAT es combinar rendimiento de una aplicación nativa con asistencia LLM integrada y solicitada por el traductor. Es una dirección de producto confirmada, no una superioridad comparativa demostrada.

El traductor podrá pedir una propuesta para un segmento o consultar y explicar el contexto de un párrafo usando lectura del documento y búsqueda web. La asistencia respeta su trabajo: no traduce el documento automáticamente ni sustituye el destino sin una aceptación explícita. La integración LLM todavía no está implementada.

## Operating Context

El flujo principal es abrir un proyecto local, importar documentos, trabajar en el grid bilingüe, consultar memoria y QA, revisar y exportar. SQLite mantiene el trabajo y su historial; la memoria TMX se importa localmente. La interfaz representa segmentos, no la maquetación de Word.

La edición, persistencia y asistencia local actual funcionan offline, sin cuentas ni sincronización. La búsqueda web prevista requiere conexión. Queda por decidir si los modelos serán locales, remotos o ambos; la futura asistencia conectada no redefine el núcleo local como un servicio obligatorio.

## Capabilities and Constraints

### Implementado

- Edición GPUI con fila activa, navegación y auto-scroll, tags visuales, inserción del siguiente tag, búsqueda y reemplazo por segmento o documento, estadísticas y progreso.
- Autoguardado, historial y undo/redo persistidos, bloqueos y operaciones transaccionales SQLite con WAL y `synchronous=FULL`.
- Memoria TMX textual local, coincidencias y concordancia; aplicación humana explícita de sugerencias y avisos QA.
- Idiomas de importación configurables y persistidos; conflicto lingüístico XLIFF rechazado. Colecciones TM internas, selección de escritura y aprendizaje de texto plano al confirmar, con suspensión al corregir y undo/redo de sus efectos. Ver [memorias](guides/MEMORIAS.md).
- Terminología local: bases activables, conceptos con expresiones por idioma y estados preferido/permitido/prohibido. GPUI permite añadir un par manualmente; reconocimiento y QA consultan las bases compatibles. El núcleo conserva variantes, notas, dominio y procedencia. Ver [alcance y límites](guides/TERMINOLOGIA.md).
- Importación y exportación TXT UTF-8, XLIFF 1.2 textual y un subconjunto conservador de DOCX. Word admite párrafos, celdas y tablas simples; los runs deben compartir formato inline. La exportación preserva las otras partes del paquete.

### Límites actuales

- Se rechazan DOCX con texto en gráficos, SmartArt, cuadros de texto e historias secundarias, así como formato mixto y otras estructuras no soportadas. No se promete compatibilidad universal Word ni paridad visual con Microsoft Word.
- Los badges de tags del editor no amplían el soporte de códigos inline de los importadores XLIFF/TMX ni el formato mixto de Word.
- El grid GPUI usa virtualización nativa y hasta 1.024 filas en caché, además del segmento activo. La búsqueda aún reúne resultados completos. Se comprobó apertura y edición del último segmento de un corpus sintético de 100.000 filas; no hay métricas p50/p95 ni escala garantizada para corpus reales. Ver [evidencia](technical/GRID_VIRTUALIZADO.md).
- El progreso mide destinos no vacíos, no confirmación ni QA aprobada. El porcentaje TM aplicado es metadata de sesión.
- La selección parcial por ratón y la navegación del cursor por clic siguen pendientes. Los proyectos nuevos empiezan en→es y permiten configurar otro par para próximas importaciones; esto no relabela documentos existentes.

### Próxima prioridad y decisiones abiertas

La prioridad confirmada es ampliar la fidelidad DOCX para documentos reales, incluidas tablas y visualizaciones. La asistencia LLM forma parte de la visión; no se ha acordado una secuencia de implementación posterior a esa prioridad.

Quedan abiertos proveedores y modelos, funcionamiento local/remoto, alcance del contexto enviado, reglas de privacidad y costes de las consultas conectadas. No se ha elegido un objetivo cuantitativo de rendimiento, un modelo comercial ni una promesa de compatibilidad con paquetes SDLXLIFF.

## Brand Commitments

LumenCAT es el nombre de trabajo; no hay confirmación de marca definitiva. La voz es profesional, precisa y sobria. La experiencia debe respetar la autoría y las decisiones del traductor, y describir sus capacidades y límites sin exagerarlos.

## Evidence on Hand

- [Guía de uso y límites actuales](guides/INICIO.md).
- [Verificación GPUI y DOCX del 30 de septiembre de 2026](technical/GPUI_DOCX_VERIFICACION.md): pruebas nativas en Windows, traducción y exportación de un DOCX público con tabla, y rechazos explícitos de muestras con visualizaciones.
- `scripts/utils/verify_docx.rs` permite reproducir importación, edición, exportación y reimportación de documentos reales. Los artefactos locales de la verificación están en `output/verification/`.
- Tests de persistencia, recuperación y rollback apoyan la integridad; no sustituyen las pruebas de entrega de documentos ni acreditan ergonomía o rendimiento comparativo.

La visión se confirmó con el responsable del producto en esta conversación. No hay entrevistas documentadas con traductores, testimonios ni benchmarks comparativos que justifiquen promesas comerciales.

## Product Principles

1. Proteger el trabajo: integridad, persistencia y fidelidad documental condicionan cada ampliación de soporte.
2. Mantener al traductor al mando: las propuestas TM o LLM requieren decisión humana; la asistencia LLM se activa a petición.
3. Priorizar fluidez nativa: el trabajo por teclado y el rendimiento se evalúan con documentos y sesiones reales.
4. Entregar documentos fieles: rechazar contenido no soportado antes que omitirlo silenciosamente o aparentar compatibilidad.
5. Conservar un núcleo local: la edición y el trabajo guardado no dependen de asistencia conectada; las capacidades web se identifican como tales.

## Accessibility & Inclusion

Estados comprensibles mediante texto, foco visible, atajos y legibilidad durante sesiones prolongadas. No hay conformidad WCAG evaluada. IME, RTL, lectores de pantalla y escalado necesitan validación real; la entrada nativa conectada no certifica esos escenarios.
