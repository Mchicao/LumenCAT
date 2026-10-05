# Idiomas de importación

Configura **Origen** y **Destino** en la barra lateral GPUI y pulsa **Guardar idiomas** antes de importar. Se guardan en el proyecto y se recuperan al reabrirlo.

- Usa códigos como `fr`, `es`, `pt-BR`, `pt-PT` o `zh-Hant`. Este corte valida un subconjunto sintáctico: no consulta un registro de idiomas ni certifica diccionarios para todos ellos.
- Cambiar estos valores afecta próximas importaciones, no los idiomas de documentos ya guardados. Las búsquedas TM usan el par del documento activo.
- XLIFF contrasta cada `source-language`/`target-language` declarado con el par elegido. Un conflicto rechaza la importación; selecciona el par del archivo y vuelve a importar. No se relabela el envelope.
- Los archivos XLIFF antiguos sin esos atributos conservan el comportamiento anterior: usan los idiomas elegidos explícitamente.

Verificación: `cargo test --locked --test storage --test formats` comprueba `fr→es`, exportación TXT, reapertura, variantes regionales, entradas inválidas y rechazo de conflictos XLIFF. La ventana GPUI fue lanzada e inspeccionada; el recorrido físico del formulario sigue pendiente por rechazo del teclado background. Las pruebas del núcleo no acreditan ese recorrido.

Rollback del corte: configuración v3, validación lingüística de formatos, comandos worker y controles de idiomas. Para abrir con un binario anterior, recupera el respaldo de la versión correspondiente como copia nueva.
