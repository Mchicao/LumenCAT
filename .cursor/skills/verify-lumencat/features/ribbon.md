## Sub-features

- `ribbon.tabs`: Archivo, Inicio, Revisión, Avanzado, Ver, Complementos y Configuración cambian la superficie/comandos; no son etiquetas decorativas.
- `ribbon.file`: vista Archivo completa con navegación azul y acciones a la derecha. Abrir, Nuevo y Documentos del proyecto cambian el contenido; Guardar, Guardar destino como y Salir son acciones directas. Nuevo usa Guardar; Abrir selecciona un archivo existente.
- `ribbon.exit`: Salir y la X requieren confirmación. Cancelar conserva la sesión; Guardar y salir espera el commit y cierre; Salir sin guardar descarta solo lo pendiente, nunca revierte commits ni operaciones iniciadas.
- `ribbon.review`: Verificar segmento solicita QA del activo y abre el panel; no acredita QA de documento completo. Confirmar sin avanzar y bloqueo reutilizan los comandos del editor.
- `ribbon.view`: ocultar/mostrar documentos/resultados y restaurar paneles no pierde selección ni borrador.
- `appearance.mode-theme`: claro/oscuro y Azul/Verde petróleo/Violeta cambian la paleta completa, incluidos controles y estados.
- `appearance.persist`: reapertura recupera modo y tema del JSON local; escritura atómica y error visible sin sustituir el tema activo ante fallo.

## How to get to it (user POV)

La cinta está encima del editor. **Archivo** reemplaza la cinta por la navegación lateral y las acciones de la sección elegida; **Volver al editor** o Escape recupera Inicio. **Revisión** indica el alcance de QA con «Verificar segmento». **Avanzado** contiene los botones TMX y «Memorias del proyecto». **Ver** cambia la visibilidad de paneles. **Complementos** informa que no se cargan plugins. **Configuración** muestra «Modo claro», «Modo oscuro» y tres temas; Escape vuelve al editor.

No se muestran opciones de paquetes, cuenta, nube o complementos de Trados como si estuvieran disponibles. No reaparece el bloque «LumenCAT / Local Translation Studio» retirado del encabezado.

## Driving it with cua-driver

1. Compila la build actual y lanza mediante el controlador con `-WaitSeconds 900`. Exige proyecto/ajustes propios del RunId; no conduzcas la instancia del usuario.
2. Pulsa cada pestaña por Role UIA `TabItem`, toma captura después y comprueba sus comandos/página. En Archivo importa un documento por diálogo real; al cerrar el selector debe aparecer el editor con sus segmentos.
3. Escribe un destino, confirma o espera autoguardado. En Revisión pulsa «Verificar segmento» y comprueba el panel QA. En Ver oculta/muestra cada panel y restaura; destino y selección deben permanecer iguales.
4. En Configuración pulsa «Modo claro» y «Violeta» (Role UIA `RadioButton`), vuelve a Inicio y comprueba legibilidad del grid. Repite con «Modo oscuro» y «Verde petróleo». Compara capturas, no solo respuestas del driver.
5. Cierra/reabre por controlador. Comprueba `settings/appearance.json` de solo lectura y la selección/colores recuperados en Configuración. Exporta el documento por Archivo y comprueba contenido/original intacto. Guarda acciones y resultados por ID.
6. En Archivo pulsa Salir: exige el diálogo con sus tres opciones. Cancela y comprueba sesión/destino intactos. Repite desde la X de Windows. Guarda y sal, exige fin del proceso y reabre: comprueba persistencia.
7. Para el negativo de guardado, bloquea temporalmente una conexión SQLite del **proyecto desechable** sin modificar tablas. Edita por UIA, exige error y borrador intacto. Abre la confirmación; libera el bloqueo y prueba Guardar y salir/reapertura. Repite con otro borrador y Salir sin guardar: la reapertura debe conservar el destino anterior, no el descartado. No uses este bloqueo sobre archivos del usuario.

## Gotchas

- La apariencia se guarda fuera de SQLite. En uso normal vive bajo `%LOCALAPPDATA%/LumenCAT/settings`; el controlador usa `--settings-dir` para aislarla. Las preferencias no se prueban escribiendo directamente ese JSON.
- Si el JSON es inválido, la app abre con la paleta por defecto y muestra un aviso en Configuración; no lo reescribe por cargarlo. Si guardar falla, conserva la apariencia anterior.
- Cambiar de pestaña no confirma, descarta ni aprende una traducción. Las páginas que ocultan el editor bloquean su entrada de texto; Escape vuelve a Inicio.
- Los botones deshabilitados indican requisitos reales: proyecto/segmento/guardado. No fuerces acciones para simular capacidades.
- `cargo test --locked --test appearance` acredita persistencia, rechazo sin modificación y contraste básico de colores; no sustituye las capturas/recorridos GUI.
