# Verificación E2E de LumenCAT

Fecha de esta entrega: 3 de octubre de 2026. Superficie principal: GPUI; egui sigue mantenida mediante `--legacy-egui`.

## Contrato de cobertura

La [skill local](../../.cursor/skills/verify-lumencat/SKILL.md) y su [mapa](../../.cursor/skills/verify-lumencat/features/README.md) son la fuente de recorridos por funcionalidad implementada. Las features futuras de `BRECHAS_TRADOS.md` no se convierten en capacidades por tener un plan.

Cada receta distingue entradas, subfunciones, pasos observables y limitaciones. El estado **guía disponible** no equivale a **GUI PASS**: validar funciones desde Rust no acredita conducción de GPUI, entrada física, Word ni interoperabilidad con Trados.

| Nivel | Significado |
|---|---|
| GUI PASS | Camino real del usuario, acciones y capturas, efectos de archivos/datos cuando corresponda |
| CORE PASS | Comandos reales worker/almacenamiento/parser y aserciones sobre resultados |
| NOT RUN | Receta disponible pero no ejecutada en esta entrega |
| BLOCKED | Transporte o dependencia impide un paso concreto; nunca declarar aprobado |
| FAIL | Resultado observado contradice la expectativa; reproducir antes de corregir |

## Orquestación

- OpenCode `openai/gpt-6-luna#high`: proyecto, editor, búsqueda, QA, entrada y grid; ejecución GUI y corrección acotada del mensaje QA.
- OpenCode `zai-coding-plan/glm-5.3-flash#high`: formatos, memorias, terminología, recuperación y legacy; workflow compartido y verificación de recursos.
- Coordinador: controlador de instancia, integración del mapa, validación de artefactos y registro común. Archivos y ventanas asignados explícitamente para no colisionar.

No se delegó producción, consumo de proveedores, proyectos del usuario, commits ni push. Los logs de agentes viven en `logs/tests/verification-*-agent.jsonl`, ignorados por Git.

## Instancia reproducible y segura

```powershell
cargo build --locked --bin lumencat
pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId e2e-nuevo
pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action doctor -RunId e2e-nuevo
pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action snapshot -RunId e2e-nuevo
pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action cleanup -RunId e2e-nuevo
```

Cada launch copia la build real a una carpeta nueva e inmutable de ejecución, genera TXT/XLIFF/TMX y abre un proyecto vacío con `--project`. No siembra traducciones mediante SQLite. El diálogo nativo importa los archivos mediante acciones del usuario. La copia del EXE permite reconstruir el proyecto sin modificar el binario de otra verificación.

Doctor comprueba PID, hora de inicio, ruta/hash del EXE y HWND. En esta máquina Windows oculta `Process.Path` y `CommandLine` al shell: Cua aporta la ruta mediante `debug_window_info`; la línea de comandos devuelve `commandLineVerified=false`. La captura debe confirmar el proyecto esperado. No se anuncia una comprobación que Windows no permitió.

Antes/acción/después se conservan bajo `output/verification/<RunId>/`. Cleanup cierra únicamente el proceso registrado y conserva PNG, JSON, corpus y resultados. ForceStop es una escalada explícita con pérdida de borradores, no evidencia de cierre normal.

## Registro de esta entrega

- `e2e-20261003-gpui`: primer intento encontró diferencias de observabilidad Windows; se corrigió doctor y se cerró normalmente la instancia. Evidencia preservada. (Worktree `lumencat-trados`.)
- `e2e-20261003-gpui-02`: launch/doctor/snapshot aprobados; importación TXT por botón, escritura UIA de ruta y botón Abrir. La captura `20261003-032834-4991-after-click.png` muestra cinco segmentos y documento en→es. No se alteró SQLite para simular importación. (Worktree.)
- `e2e-trados-001` (checkout principal, 3 oct 2026): validación completa de la skill tras el rediseño del editor GPUI a cuadrícula Trados con editor inline. launch → importación TXT por diálogo real → escritura del destino inline → «Confirmar y avanzar» → avance al segmento 2 → cleanup con cola liberada. Detalle por ID en `output/verification/e2e-trados-001/results.md`. La instancia huérfana del worktree (PID 15992) ya no existía al retomar; verificado con `tasklist` antes de actuar.
- Campaña con tres agentes en paralelo (cola `output/verification/.app-lock.json`, ownership disjunto de recetas, sin toques a `src/` por parte de agentes):
  - `e2e-ed-001` — editor/grid/input/QA: 17 subfunciones GUI PASS (autoguardado, confirmar y avanzar, navegación, copiar origen, bloqueo con negativo, undo/redo, QA activo con aparición/retirada de incidencias, progreso, clipboard Unicode, persistencia/reopen). Atajos con modificador BLOCKED por transporte (degradan a «c» literal); funciones equivalentes acreditadas por botones. `results.md` + `bugs-editor.md`.
  - `e2e-fm-001/002` — proyecto/formatos y memorias/búsqueda: importaciones TXT/XLIFF/DOCX (códigos intactos, rechazo XML roto y códigos inline), exportaciones verificadas byte a byte o por zip/XML (TXT CRLF, XLIFF envelope+state, DOCX runs, TMX 1.4), noclobber, originales intactos (sha256), negativo de códigos ausentes, TMX import/apply/concordancia/export, búsqueda y reemplazo documento/segmento con undo, rechazo TMX 1.1, persistencia tras reopen. `project.language-pair`, `formats.language-conflict` y el gestor/aprendizaje de memorias quedan NOT RUN por no implementados en este checkout (existen en `origin/main`, 5 commits adelante; sin pull autorizado). `bugs-formats.md`.
  - `e2e-rc-001/002` — legacy y recuperación: smoke egui (apertura y cierre GUI PASS; importación BLOCKED por BUG-L1/BUG-L2), flujo reopen GPUI completo GUI PASS con destino confirmado persistido, CORE recovery/storage 5/5. `terminology.md` reescrita como receta-estado (termbase NO implementada). `bugs-recovery.md`.
- `e2e-fix-001` — smoke del build corregido (QA identity issues, Estado en español, guard de reentrada de diálogos): importar → editar → confirmar → exportar GUI PASS con un solo diálogo y cola liberada.
- Auditoría del mapa: 11 recetas enlazadas con los cuatro H2 obligatorios, sin archivos huérfanos; 7 corridas con `results.md`; 3 informes de bugs.

## Defectos y límites

- CORREGIDO: mensaje QA vacío mencionaba «length anomalies» sin regla de longitud → «identity issues».
- CORREGIDO: celda «Estado» mezclaba idiomas (`confirmed; editable` vs `draft; bloqueado`) → homogéneo en español (`confirmado/borrador; bloqueado/editable`), sin tocar `as_str()` de persistencia.
- CORREGIDO: diálogos nativos apilables (reentrada durante el diálogo bloqueante) → guard `native_dialog_open` en los cinco diálogos; camino normal reacreditado en `e2e-fix-001`.
- CONOCIDO, sin corregir: BUG-L1 — la superficie egui no pinta al lanzarse sin activación (ventana blanca real; UIA vivo; al maximizar swapchain obsoleto); con activación normal sí pinta. Superficie secundaria en mantenimiento; requiere tarea propia de render (glow/winit) con verificación visual.
- Transporte (no son bugs de la app): hotkeys con modificador sobre GPUI background se degradan a caracteres literales; teclado sintético sobre egui exige foreground. `type_text` inserta en el caret: dos escrituras sobre «Nombre:» concatenan la ruta.
- GPUI expone el grid en UIA tras el rediseño; el panel QA sigue sin exponer su contenido (solo captura).
- El checkout está 5 commits detrás de `origin/main`; las features presentes upstream y ausentes aquí quedaron NOT RUN, no FAIL.

## Defectos y límites

- QA mostraba un mensaje sobre ausencia de anomalías de longitud sin tener una regla de longitud. La inspección detectó la discrepancia; la reproducción GUI y verificación del fix quedan ligadas a las ejecuciones Luna.
- GPUI no expone sus controles interiores en UIA. Las acciones por píxel requieren captura fresca; esto limita selectores semánticos y pruebas de accesibilidad.
- Atajos background en `Zed::Window` pueden ser rechazados por el transporte. No se autorizó tomar foreground; ningún rechazo acredita un atajo de la app como aprobado o roto.
- Selección parcial, IME/RTL exhaustivo, rendimiento p50/p95, formatos futuros, retorno Trados y proveedores externos no están acreditados.

## Mantener cobertura

Actualiza mapa y evidencia al cambiar funcionalidades. `/maintain-verification-skill` audita recetas frente a fuente y app. Una nueva función no está verificada hasta disponer de recorrido ejecutable y resultado observable; la prueba debe fallar por comportamiento incorrecto, no por diferencias de implementación.

## Optimización y calidad — 3 de octubre de 2026 (segunda pasada)

- **Virtualización real del grid**: el contenedor `0..count` se sustituyó por `list()`/`ListState` de GPUI. Con 2000 segmentos, UIA expone solo las ~14 filas visibles (antes construía todas). Validado en `e2e-perf-004`: import de 2000, salto por búsqueda al segmento 1950, edición inline, confirmación y avance. Alturas variables soportadas con `remeasure_items` en Selected/Saved/Page y hint uniforme tras splice.
- **Bug de flujo corregido**: elegir un resultado de búsqueda ahora limpia el filtro y muestra el editor del segmento (antes `is_searching` bloqueaba la vista).
- **Bug de salto lejano corregido**: el segmento seleccionado se inserta en `rows` y el scroll se difiere al render (`pending_reveal`) con hint de altura; antes el editor no aparecía para segmentos fuera de las páginas cargadas.
- **Código muerto eliminado**: variante `PendingOp::Reload` (nunca construida) y directorio `views/` vacío.
- **BUG-L1 (egui en blanco sin activación): no reproducible** en esta build con el controlador actual (`e2e-l1-001`, captura ready con app pintada); queda como intermitente/upstream, no como defecto activo.
- **Tests**: auditoría de las 8 suites de integración y los 3 módulos unitarios: todos son de comportamiento (persistencia, recuperación, round-trips, cancelación); no se encontraron tests tautológicos ni de baja calidad que eliminar. Suite: 0 fallos con `cargo test --locked`.
