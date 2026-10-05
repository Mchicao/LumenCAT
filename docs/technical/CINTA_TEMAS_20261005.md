# Cinta y temas — 5 de octubre de 2026

Se retiró el bloque de marca de la esquina superior izquierda y se añadieron las pestañas Archivo, Inicio, Revisión, Avanzado, Ver, Complementos y Configuración. Agrupan comandos existentes; no amplían el soporte de formatos ni declaran paridad con Trados.

## Alcance

| Pestaña | Comportamiento |
|---|---|
| Archivo | Vista lateral: nuevo/abrir proyecto, importar, guardar, exportar, configuración y salir |
| Inicio | Navegación, copiar origen, bloqueo, etiquetas y confirmar/avanzar |
| Revisión | QA del segmento activo, términos, confirmación sin avanzar y bloqueo |
| Avanzado | Gestor de memorias, TMX, concordancia y reemplazo |
| Ver | Mostrar/ocultar documentos y resultados; restaurar paneles |
| Complementos | Informa que no existe todavía un sistema de plugins |
| Configuración | Claro/oscuro y Azul/Verde petróleo/Violeta, aplicados y guardados localmente |

La apariencia se guarda con reemplazo atómico en `%LOCALAPPDATA%/LumenCAT/settings/appearance.json`. El controlador usa `--settings-dir` bajo el RunId para no modificar preferencias personales. Tab/Shift+Tab recorren controles también cuando el editor está oculto; Escape vuelve a Inicio. Los selectores de apariencia exponen rol RadioButton y estado seleccionado.

## Verificación ejecutada

| Prueba | Resultado y evidencia |
|---|---|
| Todas las pestañas, Archivo, gestor y paneles | GUI PASS en [primera pasada](../../output/verification/ribbon-20261005/results.md): capturas y acciones; ocultar/restaurar conserva título y selección |
| Nuevo y abrir proyecto | GUI PASS: Nuevo creó `created.lcat` vacío mediante Guardar; Abrir recuperó `project.lcat` con el título confirmado |
| DOCX complejo | GUI PASS: 20 segmentos, título editado y confirmado; el resto del origen se copió por botones solo para completar la muestra de exportación |
| Exportar DOCX | ARTIFACT PASS, primera build: título nuevo en XML, inventario ZIP idéntico y 11 partes distintas de `word/document.xml` idénticas byte a byte; original intacto |
| Negativo de exportación | Rechazo observado con destinos vacíos, sin crear entrega; no se ocultó ni se eliminó esta protección |
| Build final: importación, confirmación, temas y reapertura | GUI PASS en [confirmación](../../output/verification/ribbon-final-20261005/results.md); texto/estado y tema oscuro/verde petróleo recuperados |
| Tab → Espacio en Configuración | GUI PASS con activación temporal autorizada: desde Modo claro se pasó a Modo oscuro sin cambiar Violeta; respuesta del driver no se usó como prueba sin leer el resultado |
| Núcleo y calidad | CORE PASS: Clippy all-targets `-D warnings`, 47 pruebas sin fallos, formato y release. [Log](../../logs/tests/ribbon-final-validation.log) |

Primera build: SHA-256 `2E182926D8A051302D0E044C810B389E56A93035B146B6B6CEB299FFCE91F6A6`. Tras ajustar foco, semántica accesible y textos vacíos, build final: `8922F77D1D90FC0D4C2D446D6C05EF83B07FA123066D4C39709691DB93412996`. El núcleo de documentos no cambió entre ambas.

La entrega inicial `ribbon-target.docx` tiene SHA-256 `1931A31EAFE71983378BD52C3DF7640574E5599A2ACFA5B0A4E94F418AB87EED`. La fuente conserva `24CB656C0D3FB4A26023F32DDD4988111792C012F47D2A7F4EDBE819D47577E5`.

## Límites y entrega

- No se volvió a abrir Word ni se ejecutó otro benchmark de Trados en esta pasada. La evidencia Word anterior está en [el benchmark](BENCHMARK_TRADOS_20261004.md); no acredita universalidad.
- Importación/exportación TMX, otros atajos físicos, IME/RTL y lectores de pantalla no se reacreditaron por GUI en esta pasada. Las pruebas Rust son evidencia auxiliar.
- No se añadieron paquetes nuevos: Serde/serde_json ya estaban en el lock y ahora se usan directamente para preferencias. No se instalaron GPUI Kit ni Ely.
- Cleanup cerró únicamente instancias propias y liberó la cola; Trados quedó abierto. El EXE de uso diario `output/lumencat.exe` no se sustituyó durante esta tarea.
- Cambios locales en `refactor/solo-gpui`; sin commit, push ni publicación en `main`.
