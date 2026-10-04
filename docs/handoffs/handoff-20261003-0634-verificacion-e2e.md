# Handoff: cobertura E2E completa de LumenCAT

**Ubicación del handoff:** `C:/Proyectos/LumenCAT/docs/handoffs/`. El trabajo y los artefactos descritos siguen en el worktree `C:/Users/matia/AppData/Local/Temp/opencode/lumencat-trados`; las rutas relativas de este documento se refieren a ese worktree. No se copiaron ni sincronizaron cambios de código al checkout principal.

## Encargo y punto de parada

El usuario pide: «orquesta agentes para que vayan creando esas skills si no existen, vayan probando la app y arreglando bugs. TODAS LAS FEATURES DE LA APP DEBEN QUEDAR CON UNA GUIA/TEST E2E».

Después pidió detener esta ejecución y preparar este handoff para otro agente. **No continuar implementando el roadmap Trados durante esta tarea.** Cubrir todas las capacidades actualmente implementadas, distinguir guías de pruebas ejecutadas y corregir defectos reproducidos.

## Workspace y seguridad

- Repo: `C:/Users/matia/AppData/Local/Temp/opencode/lumencat-trados`, rama `feat/trados-core`.
- No tocar `C:/Proyectos/LumenCAT`: contiene cambios previos ajenos.
- Base publicada previamente: `eb09f277112d5c452a3070e8edc663b9860add92`; remoto `https://github.com/Mchicao/LumenCAT.git`. Esta pasada **no hizo commits, push ni PRs**.
- Los archivos nuevos están sin commit. No descartarlos ni asumir que están terminados. No hay índice Codegraph.
- GUI exclusivamente background; no hay permiso para tomar foreground. Nunca actuar sobre proyectos/ventanas del usuario ni consumir proveedores pagados.

## Lo que existe ahora

| Artefacto | Estado |
|---|---|
| `.cursor/skills/verify-lumencat/SKILL.md` | Skill principal escrita, registro en `AGENTS.md`; validación end-to-end incompleta |
| `features/README.md` y `references/coverage.md` dentro de la skill | Índice y niveles de evidencia escritos |
| `features/project.md`, `editor.md`, `search.md`, `qa.md`, `input.md`, `grid.md` | Creados por Luna; revisar instrucciones transitorias y resultados por subfunción |
| `features/formats.md`, `memories.md`, `terminology.md`, `recovery.md` | GLM dejó archivos parciales antes de la cancelación; revisar contenido |
| `features/legacy.md` | **Falta**; el índice ya lo referencia |
| `scripts/utils/control_lumencat.ps1` | Launch/doctor/snapshot y acciones GUI; persiste evidencia; cleanup tiene fallo pendiente |
| `docs/technical/VERIFICACION_E2E.md` | Registro preliminar, no declaración de cobertura total |

No se creó `tests/workflow_e2e.rs`. No hay reporte final GLM ni reporte GUI final Luna. No se generó aún el registro `.atl/skill-registry.md`.

El mapa exige cuatro H2 por receta: `Sub-features`, `How to get to it (user POV)`, `Driving it with cua-driver`, `Gotchas`. Mantener IDs estables y cubrir todas las entradas/negativos/resultados observables, no solo un camino cómodo.

## Agentes: identidades, tareas y estado

Se verificó el catálogo y se usaron los modelos exactos pedidos mediante OpenCode 2.0.22:

- `openai/gpt-6-luna#high`, sesión `ses_eff92bde6ffeBbgSbVaQ4wm3vg`: primera fase terminó; segunda fase GUI fue **cancelada por petición del usuario**.
- `zai-coding-plan/glm-5.3-flash#high`, sesión `ses_eff92bdeaffe0mxkt5e2nJiqhV`: primera fase **cancelada**, con cuatro recetas escritas.

Los procesos de agentes fueron detenidos; las dos tareas background notificaron exit 1 por cancelación. No duplicar ejecuciones sin comprobar estado.

Contratos completos locales, ignorados por Git: `.cache/verification/luna-task.md`, `luna-gui-task.md`, `glm-task.md`. Informe terminado: `.cache/verification/luna-report.md`. Logs: `logs/tests/verification-luna-agent.jsonl`, `verification-luna-gui-agent.jsonl`, `verification-glm-agent.jsonl`.

Para continuar una sesión: `opencode run --standalone -m '<provider/model#high>' --session <ses_...> --format json --file <contrato> '<tarea>'`. Confirmar `opencode run --help` antes de usar flags. Las escrituras autónomas fueron autorizadas; `--auto` se usó con límites explícitos, no como permiso para producción. T3 no estaba expuesto y su transporte ACP tampoco.

## Evidencia realmente obtenida

- `cargo build --locked --bin lumencat --bin lumencat-gpui`: aprobado antes de esta pasada GUI. Advertencia preexistente `proc-macro-error2 v2.0.1`.
- Parser PowerShell del controlador: aprobado tras las modificaciones.
- Luna: `cargo test --locked --test qa --test replacement --test search_cancel` (3 pruebas); dos checks unitarios focalizados del grid/Unicode (2 pruebas). Logs `logs/tests/luna-core.log`, `luna-unit.log`. **No sustituyen E2E GUI.**
- `output/verification/e2e-20261003-gpui`: primer launch, ajuste de doctor Windows y cierre normal; evidencia preservada.
- `output/verification/e2e-20261003-gpui-02`: proyecto nuevo abierto; TXT importado por **Importar documento → diálogo real → Nombre: → Abrir**. Captura `20261003-032834-4991-after-click.png`: 5 filas y documento en→es.
- Luna GUI alcanzó escritura Unicode y observó autoguardado. No hay informe final: revisar acciones/PNG antes de acreditar IDs. La ventana se minimizó antes del clic de confirmación; el controlador rechazó coordenadas. No se acredita confirmación, QA limpio, exportación ni reapertura en esta pasada.

Los 47 tests históricos y recorridos F03/F17 previos están descritos en `.agent/memory/PROJECT_MEMORY.md` y los documentos existentes; no presentarlos como validación de los nuevos artefactos.

## Bloqueo inmediato: instancia temporal pendiente de cerrar

La instancia `e2e-20261003-gpui-02` seguía registrada y viva en la última comprobación: **PID 15992, HWND 13961450**, EXE original `.cache/target/debug/lumencat.exe`. Revalidar PID/hora/ruta antes de actuar; podrían cambiar.

Se intentó cleanup normal y luego `cleanup -ForceStop`. El último terminó con **«La instancia no terminó»** y `Get-Process` aún devolvió PID 15992. No afirmar que se cerró ni borrar evidencia. Investigar primero el resultado de Cua y el estado real; no repetir fuerza ciegamente ni matar por nombre. El primer PID 18592 sí cerró.

El controlador fue modificado después del primer launch para copiar la build al directorio de cada ejecución; **esa variante launch-copy aún no se probó en vivo**. Los runs existentes usan el EXE original. Cleanup/doctor deben seguir aceptando su manifiesto anterior.

Windows oculta `Process.Path` y `CommandLine` al shell. Doctor usa `cua-driver debug_window_info` para comprobar ruta y compara hora como `DateTime`; reporta `commandLineVerified=false`. La captura debe identificar el proyecto esperado. GPUI UIA solo expone controles nativos de ventana; diálogos sí tienen campos/botones semánticos.

## Defecto detectado, todavía sin corregir

QA limpio afirma que no hay `length anomalies`, pero `src/qa.rs` no evalúa longitud. Evidencia estática en `.cache/verification/luna-report.md`; reproducción GUI pendiente.

Repro propuesta: fila `Hello world`, destino `x`, confirmar, volver a la fila y abrir QA. Capturar QA cero y mensaje. Fix mínimo: corregir el mensaje en `src/gpui_app/mod.rs`, **no inventar una nueva regla de longitud**. Volver a ejecutar sobre build nueva. Al detenerse, `git status` no mostraba cambios de `src/`.

## Plan de continuación y aceptación

1. **Recuperar control seguro:** inspeccionar estado Git, agentes cancelados y manifiestos; resolver/cerrar solamente la instancia temporal pendiente, conservar evidencia. Probar launch-copy/doctor/cleanup con RunId nuevo y una feature real.
2. **Cerrar inventario:** contrastar `src/gpui_app/`, `src/app.rs`, `worker::Task`, parsers, storage y guías. Completar `legacy.md`, revisar recetas parciales, reparar links y quitar frases como «el coordinador posee la ventana» de recetas permanentes. Separar features futuras de actuales.
3. **Delegar con propiedad disjunta:** Luna: editor/búsqueda/QA/input/grid y repro/fix QA; GLM: formatos/TM/terminología/recuperación/legacy. Un PID/proyecto por agente; asignar cualquier fix de `src` explícitamente para evitar colisiones. Pueden reutilizar sesiones y contratos existentes.
4. **Ejecutar todo el mapa:** registrar cada ID con `GUI PASS`, `CORE PASS`, `FAIL`, `BLOCKED` o `NOT RUN`. Probar acciones humanas, resultados visibles, exportaciones reales, fuentes intactas, persistencia/undo/TM juntos. Backup/migraciones sin UI se prueban por API/proceso y se etiquetan CORE, nunca GUI. Hotkeys rechazadas por `Zed::Window` quedan BLOCKED sin foreground.
5. **Corregir solo bugs reproducidos y verificar:** actualizar recetas y `docs/technical/VERIFICACION_E2E.md`; ejecutar fmt, Clippy all-targets, tests y builds locked del scope. Confirmar que cleanup conserva pruebas y no deja procesos. Actualizar memoria duradera y registro de skills si corresponde.

**Terminado significa:** todas las funcionalidades implementadas tienen receta/test mantenido, no faltan entradas ni casos críticos del inventario, cada ejecución tiene evidencia y los bloqueos están separados de PASS. No prometer «todo validado» mientras queden NOT RUN/BLOCKED. No certificar Word/Trados/proveedores externos sin artefactos, aplicaciones y autorización necesarios.

## Skills sugeridas

`tldr`, `create-verification-skill`, `multi-provider-agent-orchestration`, `cua-driver` (leer también `WINDOWS.md`), `diagnosing-bugs`, `skill-creator`, `lint-and-validate`, `verifying-completion`; `maintain-verification-skill` para auditoría final del mapa. No iniciar SWARMS ni crear más ceremonia por defecto.
