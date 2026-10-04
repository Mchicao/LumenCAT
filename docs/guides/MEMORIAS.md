# Aprender en memoria al confirmar

**Confirmar** guarda la traducción y la aprende en la memoria de escritura compatible, en la misma transacción. Guardar un borrador no aprende. La confirmación no aprueba QA ni revisión editorial.

## Elegir memoria en GPUI

1. Configura y guarda los idiomas de importación.
2. Abre **Memoria**. Crea una colección con nombre y selecciona **Usar para aprender**. Un proyecto nuevo en→es incluye una memoria de escritura inicial.
3. Traduce y confirma. Una repetición muestra la traducción aprendida; la sugerencia identifica colección y procedencia.

**Desactivar aprendizaje** permite confirmar sin escribir TM. Puedes pasar una colección a solo lectura o excluirla de búsquedas sin eliminar sus unidades. Cambiar el par del proyecto deja sin selección de escritura una memoria incompatible; selecciona una del nuevo par. El par del documento activo sigue gobernando búsquedas y aprendizaje.

Importar TMX añade unidades a una colección **Memoria importada** por par de idiomas. Exportar TMX exporta unidades importadas y versiones aprendidas activas de todas las colecciones, incluso las excluidas de búsquedas. Una importación de otra traducción no sustituye variantes existentes. Los recursos permanecen dentro del `.lcat`; todavía no se usan memorias externas compartidas.

## Correcciones y deshacer

| Acción | Efecto en memoria |
|---|---|
| Reconfirmar el mismo texto | No duplica la contribución ni crea otra operación. |
| Editar una traducción confirmada | Suspende las contribuciones de ese segmento hasta reconfirmar. |
| Reconfirmar la corrección | Activa la versión corregida en la memoria elegida. Las versiones anteriores permanecen guardadas e inactivas. |
| Deshacer/rehacer | Restaura texto, estado y actividad de las contribuciones como una unidad, incluso tras reiniciar. |
| Reemplazar en varios segmentos | Suspende sus contribuciones dentro del grupo de historial; undo las restaura juntas. |

Un autosave en vuelo no descarta la intención de confirmar. La interfaz espera su respuesta para usar la revisión actual; no avanza antes del commit de confirmación. Volver a editar invalida la intención anterior. Un error preserva el borrador y no comunica confirmación durable.

## Límites del corte

- Aprendizaje de texto plano; DOCX con códigos se valida al confirmar, pero no se aprende como TMX textual. El mensaje informa ese límite.
- Confirmar vacío o bloqueado se rechaza. Una importación de estado `Confirmed`, un lock o un undo no se interpreta como intención humana de aprender.
- La búsqueda agrupa destinos equivalentes y conserva las unidades en almacenamiento. La presentación completa de procedencias agregadas, prioridades, penalizaciones y coincidencias de contexto está pendiente.
- Ambas interfaces usan el mismo comando de confirmación. Administración de colecciones solo en GPUI en este corte.
- Build debug y pruebas del núcleo verificadas; el recorrido de teclado/formulario GPUI sigue pendiente por rechazo del transporte background. No se autorizó takeover del escritorio.

## Verificar y volver atrás

```powershell
cargo test --locked --test tm_learning --test docx
cargo test --locked --lib migration_tests
```

Las pruebas confirman sobre proyectos temporales, reabren, buscan, exportan/reimportan TMX y comprueban suspensión, variantes, lectura, aislamiento lingüístico, rollback inducido y grupos de undo/redo. DOCX usa un archivo real generado para comprobar pérdida de códigos y exportación posterior válida.

El esquema 4 migra v1/v2/v3 con respaldo previo. Para volver a un binario anterior, recupera una copia de ese respaldo; revertir código no degrada el proyecto. La frontera de rollback es la migración v4, aprendizaje/actividad TM, efectos `history_tm` y comandos/control de confirmación de ambas interfaces.
