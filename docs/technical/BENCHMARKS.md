# Benchmark reproducible del núcleo

Windows11 Home, Intel i7-8665U, 32GiB RAM, Rust/Cargo1.97, build release thin-LTO/debug0. Entorno exacto en `output/benchmark-environment.json`. Datos sintéticos deterministas, un proceso por tamaño, corridas secuenciales; 30 muestras por lookup/page/commit. p50=índice15, p95=índice28 ordenado (estimador nearest-rank para30 muestras), sin control de caché del SO. La concurrencia de compilación/reviews durante la sesión puede afectar medidas; no comparación de hardware ni benchmark certificado.

| Operación | 10k | 100k | 1M | 3M |
|---|---:|---:|---:|---:|
| Exact TM p95 ms | 0.025 | 0.025 | 0.025 | 0.028 |
| Fuzzy p95 ms | 0.835 | 0.947 | 0.946 | 1.274 |
| Concordancia p95 ms | 0.055 | 0.099 | 0.126 | 0.160 |
| Commit autosave p95 ms | 2.188 | 1.296 | 1.246 | 2.208 |
| Import TMX segundos | 0.727 | 7.098 | 79.200 | 269.358 |
| Export TMX segundos | 0.043 | 0.383 | 5.152 | 12.315 |
| Abrir proyecto warm p95 ms | 21.801 | 28.037 | 103.861 | 57.963 |
| DB MiB tras checkpoint | 5.05 | 51.01 | 457.90 | 1333.65 |

En corrida3M: TXT250k parse+persist577ms, page128 p950.148ms, QA250k138ms. La búsqueda medida encuentra términos abundantes al principio (p950.167ms): NO representa worst-case de texto ausente, que puede escanear documento; ese camino sí tiene cancelación SQLite probada. Concordancia mide token numérico selectivo; no equivale a substring arbitrario.

El corpus comparte palabras comunes con ID numérico, source/target ~30 caracteres. Fuzzy busca sustitución delivery→dispatch; recall declarado: esperado aparece top8 en1 caso por escala. No extrapolar a corpus multilingüe heterogéneo ni concluir recuperación completa. Normalización y falta de segmentación CJK son límites abiertos paraM2.

Datos finales: `output/benchmark-10000-final.csv`, `benchmark-100000-final.csv`, `benchmark-1000000-final.csv`, `benchmark-3000000-final.csv`. Conteos import/export iguales a escala verificados dentro del runner; fixtures/DB temporales se eliminan al terminar. CSV iniciales conservados muestran correcciones: candidate recall0→1; concordancia10k p95750→0.055ms; fuzzy3M226→1.274ms. Las últimas correcciones UI/metadata/cancelación no cambian el dataset ni scoring medido.

## Lo que falta medir

Cold startup, warm ventana primera pintada, RAM idle real, frame/scroll/input con250k, sesiones largas, peor caso search/CJK/fuzzy largo, DOCX real y corpus TMX/XLIFF autorizado. No se automatizó UI nativa ni Word: el usuario restringe computer use sin autorización explícita. No atribuir timings de DB a latencia UI. Apertura de proyecto warm de1M excedió objetivo p95100ms una vez: worker asíncrono no bloquea render, pero se conserva resultado sin ocultarlo.

## Reproducir

`scripts/utils/benchmark.ps1` corre10k/100k/1M; `-Counts 3000000` agrega escala3M. Genera outputs con timestamp sin sobrescribir. Puede necesitar temporalmente ~3GiB para3M entre TMX/DB/WAL/export; al salir normal se retiran fixtures. Asegurar espacio suficiente y no iniciar rebuild del ejecutable benchmark mientras esté corriendo (Windows lo bloquea). La caché de compilación puede reconstruirse desde Cargo.lock.
