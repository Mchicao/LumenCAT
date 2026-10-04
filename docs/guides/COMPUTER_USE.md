# Computer Use en Windows

La interfaz GPUI expone accesibilidad nativa mediante AccessKit/UIA. Usar el EXE release actualizado en `output/lumencat-gpui.exe`; acepta `--project C:/ruta/proyecto.lcat` y la ruta posicional anterior. El frontend egui permanece separado.

## Operación eficiente con cua-driver

1. Lanzar con `launch_app`, guardar PID/HWND y observar `get_window_state`.
2. Elegir el control por rol/nombre del snapshot y actuar con su `element_token`. Los AutomationId son estables, pero el token caduca al tomar otro snapshot del mismo HWND, incluso desde el CLI de verificación.
3. Usar `Invoke`/`SelectionItem` para botones, documentos, filas y pestañas; `set_value` para escribir el campo completo. Verificar el valor y el estado desde un snapshot nuevo. La escritura del destino comparte autoguardado, historial y QA con la entrada humana.
4. Entre capturas visuales, usar `include_screenshot:false` y `query` para consultar solo el campo o acción necesarios. `query` reduce la respuesta, no el coste de recorrer el proveedor. En diálogos nativos, `max_depth:3` evita recorrer archivos y navegación de Explorer; ante timeout puede servir profundidad 2 para el ComboBox «Nombre:», seguido de lectura del Edit a profundidad 3.
5. Esperar «Guardado» y que la acción esté habilitada antes de cambiar documento o exportar. Un mensaje «Error» debe quedar resuelto; no interpretar la entrega del comando como prueba de su efecto.

| Acción | AutomationId / nombre |
| --- | --- |
| Proyecto / importar / exportar | `project-open`, `document-import`, `document-export` |
| Seleccionar documento / segmento | `document-{id}`, `segment-{id}`; nombres de archivo / «Segmento N» |
| Leer origen / escribir destino | `active-source` / `target-editor`; «Origen del segmento activo» / «Destino» |
| Copiar / insertar etiqueta / confirmar | `source-copy`, `insert-next-tag`, `segment-confirm-next` |
| Anterior / siguiente / bloquear | `segment-previous`, `segment-next`, `segment-lock` |
| Historial | `undo`, `redo`; «Deshacer», «Rehacer» |
| Buscar / reemplazar / concordancia | `search-query`, `search-submit`, `replacement-text`, `replace-apply`, `concordance` |
| TMX / aplicar coincidencia | `memory-menu`, `tm-import`, `tm-export`, `tm-apply-{índice}` |
| TM / QA / estado | `tm-tab`, `qa-tab`, `application-status` |

El origen, las celdas del grid, el destino bloqueado y el editor sin segmento son de solo lectura. Los controles que no pueden actuar están deshabilitados. Un documento con cero segmentos conserva sus imágenes y permite exportar, con editor vacío y acciones de traducción deshabilitadas. El pie permanente con mensajes promocionales se oculta; quedan el progreso y los estados que requieren atención.

## Límites comprobados

Las etiquetas se dibujan como badges; UIA entrega el texto íntegro con `<g>`/`<x/>` para conservarlo al escribir. SetValue puede quitar códigos del destino, pero la exportación DOCX rechaza códigos ausentes/alterados y no publica un archivo parcial. «Insertar siguiente etiqueta protegida» regenera el siguiente código que falta.

En esta máquina, los modificadores enviados por PostMessage no llegaron a GPUI: Ctrl+Enter no avanzó y Ctrl+, escribió una coma. Para agentes, usar las acciones UIA equivalentes. Probar los atajos físicos requiere entrada foreground y autorización explícita; no se ha realizado en esta corrida. No hay TextPattern/selección parcial del texto por UIA; SetValue reemplaza el campo completo. IME/RTL, lectores de pantalla y otras máquinas quedan sin validar.

El grid completo no está virtualizado. Sus filas accesibles usan Invoke y una descripción de selección porque AccessKit Windows 0.34 no ofrece SelectionItem para Row. El cliente .NET UIAutomationClient devolvió un árbol vacío; cua-driver sí leyó y operó los controles. No se garantiza compatibilidad con cualquier cliente UIA ni un ahorro fijo de tokens.

## Comprobación reproducible

Con la app abierta en un proyecto de verificación:

```powershell
& scripts/utils/verify_gpui_uia.ps1 -ProcessId <pid> -WindowId <hwnd> `
    -ExpectedSegments 20 -ExpectedSource '<origen esperado>' `
    -ExpectedTarget '<destino esperado>' -OutputDirectory .cache/verification/<corrida>/uia
```

El script consulta la app real mediante cua-driver, comprueba campos, filas y estado, y guarda JSON. `-Locked` comprueba que confirmar no esté habilitado; las negativas de escritura se prueban con cua-driver y lectura posterior. No escribe en el proyecto ni sustituye la prueba de exportación y Word. El detalle de la corrida está en [CONTINUACION_DOCX](../technical/CONTINUACION_DOCX.md).
