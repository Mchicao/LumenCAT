# TMX con códigos nativos y variantes

Estado: núcleo verificado con fixtures sintéticas. GUI y lectura/reimportación en Trados todavía no ejecutadas. No acredita actualización directa de SDLTM ni paridad completa de memorias.

## Alcance del corte

TMX 1.4/1.4b UTF-8 importa grupos nativos `bpt`/`ept` (emparejados por `i`), anotaciones `hi` y códigos atómicos `ph`, `it` y `ut`. El atributo `x` enlaza los códigos de origen y destino, incluso cuando cada idioma utiliza otro `i` o aparecen en distinto orden. Sin `x`, los pares usan `i` y otros códigos su orden de aparición.

El modelo protegido compartido con XLIFF conserva apertura, cierre y contenido nativo, sin exponer este último como texto traducible. Una unidad importada conserva en `raw_xml` sus atributos, propiedades TU y todas las variantes lingüísticas; la exportación las reutiliza sin aplanarlas. No se ha añadido escritura SQL en SDLTM.

La importación sigue siendo transaccional: una unidad inválida posterior revierte las anteriores. Un destino con pares incoherentes no entra en la memoria; el original no se modifica.

## Pruebas ejecutadas

`tests/tmx_inline.rs` comprueba una unidad multilingüe con destino antes del origen, identificadores `i` distintos y enlace `x`, importación al proyecto, búsqueda exacta, exportación, lectura posterior, propiedades/códigos conservados y original intacto. Otra prueba comprueba rollback completo ante un cierre nativo de identificador incorrecto.

La prueba positiva falló contra el código anterior por rechazo de inline (`logs/tests/tmx-inline-red.log`). La implementación se valida junto con XLIFF y la suite completa; la evidencia se conserva en `logs/tests/tmx-inline-all-validation.log`.

## Pendiente

`sub`, pares superpuestos, UTF-16, cabecera original completa, aprendizaje/edición de unidades con códigos, SDLTM y formatos legados. La salida de TMX sigue siendo un archivo nuevo, no una sobrescritura de la memoria original. El ejecutable que está probando el usuario no se reemplaza.

Fuente normativa y API de referencia: [investigación de memorias Trados](PARIDAD_MEMORIAS_TRADOS_FUENTES.md), sección TMX y referencias RWS/LISA. La evidencia propia de este corte se limita al núcleo y a archivos sintéticos.
