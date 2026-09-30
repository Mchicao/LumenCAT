# Interfaz nativa inicial

Traductor en escritorio iluminado, revisando documentos durante horas: superficies claras neutras, texto oscuro de alto contraste, acento teal reservado a acción/selección. Estrategia contenida; sin tarjetas decorativas ni animaciones. Base perceptual acento OKLCH(0.450 0.074 200), adaptada a sRGB que requiere egui; neutros acromáticos. Un tipo sans de egui, tamaños estables y zoom del SO/egui.

Fila activa separa source inmutable de target editable completo. Navegación virtualizada muestra previews acotados y estados escritos. Panel derecho muestra TM y acción humana Insertar; nunca sustituye target al seleccionar fila. Footer distingue pendiente/durable/error/recuperación. QA es advertencia visible, no modificación automática. Rutas iniciales explícitas para import/export/proyecto; no placeholders que parezcan funciones completas.

El diseño está basado en flujos documentados, aún sin pruebas reales con traductores. No declarar ergonomía/scroll/IME verificados por compilar.
