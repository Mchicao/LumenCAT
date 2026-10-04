---
name: verify-lumencat
description: "Trigger: verificar LumenCAT, E2E, GPUI o legacy. Conduce la app de escritorio aislada y conserva acciones, capturas y resultados reales."
license: Apache-2.0
metadata:
  author: "Mchicao"
  version: "1.0"
---

## Activation Contract

Activa esta skill para verificar comportamiento implementado de LumenCAT, reproducir bugs o validar entregas de escritorio.

## Hard Rules

- Usa únicamente proyectos nuevos en `output/verification/<RunId>`. Nunca conduzcas una instancia del usuario ni escribas SQLite para simular una acción GUI.
- La app es un recurso en cola: el controlador mantiene `output/verification/.app-lock.json`. Lanza y conduce con `-WaitSeconds 900`; si otro RunId sostiene la cola, espera o ejecuta su cleanup. Nunca lances el EXE por tu cuenta fuera del controlador.
- Usa `cua-driver` en background. Obtén captura antes y después de cada acción. Deriva coordenadas de la captura actual, no de recetas antiguas.
- No escales a foreground sin autorización. `unverifiable` exige leer la captura; `background_unavailable` bloquea esa acción, no acredita un bug de la app.
- Conserva evidencia tras cleanup. No declares E2E GUI por tests Rust, seeds, lectura estática o una captura final sin acciones.

## Decision Gates

| Superficie | Ruta |
|---|---|
| GPUI principal | Controlador, `-Surface gpui` |
| egui secundaria | Ejecución distinta, `-Surface legacy` |
| Backup/migraciones sin UI | Pruebas de núcleo; etiqueta ese nivel, no inventes botones |
| Función pendiente o frontera pagada | Registra pendiente/bloqueada; no implementes ni consumas red como efecto lateral |

## Execution Steps

### Launch

Desde la raíz, ejecuta `cargo build --locked --bin lumencat`. Luego `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action launch -RunId e2e-001 -WaitSeconds 900`. El controlador crea corpus sintético y proyecto vacío nuevos, abre mediante Cua y devuelve PID/ventana/hash/rutas. Lee `*-ready.png`: exige proyecto abierto antes de actuar.

### Doctor

Ejecuta `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action doctor -RunId e2e-001`. Comprueba proceso, hora de inicio, hash y ventana. Si Windows oculta la línea de comandos, devuelve `commandLineVerified=false`: exige captura del proyecto esperado. Es read-only; no repara ni migra proyectos.

### Drive

Lee [el mapa](features/README.md) y la receta. Usa `-Action snapshot`, después `-Action click -X <x> -Y <y>` o `-Action type -X <x> -Y <y> -Text '<texto>'`, siempre con `-RunId e2e-001 -WaitSeconds 900`. El controlador guarda antes/acción/después y refresca el latido de la cola. En diálogos usa HWND del mismo PID y `-Label 'Nombre:' -Role Edit`. Para teclado: `-Action key -Keys Enter`; no asumas entrega de hotkeys.

El editor GPUI es una cuadrícula Trados: cada fila es `# | origen | destino | barra de estado`. La fila activa contiene el editor de destino dentro del grid (`target-editor`, etiqueta UIA «Destino») y su origen lee «Origen del segmento activo» (`active-source`). Las filas inactivas exponen celdas «Origen»/«Destino»/«Estado» de solo lectura. La barra de estado lateral: rojo sin destino, ámbar borrador, verde confirmado, gris bloqueado.

### Evidence

Guarda resultados por subfunción en `output/verification/<RunId>/results.md`: acción, expectativa, resultado, artefactos y límite. Verifica también archivos exportados, persistencia y fuentes intactas. Aplica [los niveles de prueba](references/coverage.md); tests de núcleo son evidencia auxiliar.

Gotcha de consola: los valores UIA con caracteres no ASCII (p. ej. «café») pueden llegar con doble codificación cuando el agente compara desde Git Bash. Compara cadenas no ASCII dentro del mismo PowerShell que leyó el snapshot, o usa expectativas ASCII.

### Cleanup

Ejecuta `pwsh -NoProfile -File scripts/utils/control_lumencat.ps1 -Action cleanup -RunId e2e-001`. Cierra por UIA únicamente el PID registrado y libera la cola. `-ForceStop` pierde borradores: úsalo solo para la instancia desechable si cierre normal falla. Para persistencia, `-Action reopen` reutiliza el proyecto sin borrar evidencia. Comprueba que no queda proceso y sobreviven capturas/resultados.

### Helpers

El controlador anterior es el helper ejecutable. [Verificación](references/coverage.md) lista checks de núcleo y corpus especializados. Comprobación de campos UIA: `scripts/utils/verify_gpui_uia.ps1 -ProcessId <pid> -WindowId <hwnd> -OutputDirectory <dir>` (en proyectos con segmento activo).

## Output Contract

Devuelve subfunciones verificadas, fallidas, no ejecutadas y bloqueadas, con rutas de evidencia. Una guía escrita no equivale a ejecución aprobada.

## References

- [Mapa completo de funciones](features/README.md).
- [Cobertura y niveles de evidencia](references/coverage.md).
- [Guía de uso](../../../docs/guides/INICIO.md).
