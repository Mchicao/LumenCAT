# Comparación real de LumenCAT y Trados — 2026-10-04

LumenCAT y Trados abrieron el mismo DOCX de prueba y mostraron 20 segmentos. Ambos generaron un DOCX con el título traducido y el resto del origen copiado. Word abrió las dos entregas; sus renders de dos páginas resultaron idénticos por SHA-256. Esto acredita esta muestra, no compatibilidad universal ni paridad completa.

## Entorno y alcance

- Windows, AMD Ryzen 5 5600G, 6 cores / 12 logical processors, 17.068.290.048 bytes de RAM instalada.
- Trados instalado: `SDLTradosStudio.exe`, FileVersion `19.0.0.3043`, ProductVersion `Studio19`. No se atribuyen funciones de SR1/2026 a esta build.
- LumenCAT: build release local sobre `e493199` más cambios sin commit. EXE de comparación documental: SHA-256 `8FBDA44CBD88313B378C4A60B9569691DA8BF6A02EA96622902C0BFC108D6D5C`. Build con mejora UI y medición final: `D54ECA620D04C386CA4A8E98D3029D8B4F4E6DACBE9BED38300E791E7098FE77`.
- Fuente: [complex.docx](../../output/verification/trados-bench-20261004/complex.docx), copia intacta de `word-inspection.docx`: 15.499 bytes, SHA-256 `24CB656C0D3FB4A26023F32DDD4988111792C012F47D2A7F4EDBE819D47577E5`. Dos páginas, 33 párrafos Word, dos tablas, celdas combinadas, tres imágenes inline, formato mixto y salto de página por estilo.
- Proyectos desechables de LumenCAT, lanzados por controlador con cola. Trados usó «Traducir documento individual», en-US → Spanish (International), sin TM ni proveedor de traducción automática. LumenCAT usó en → es; la confirmación aprende en su TM local.

## Recorridos y resultados

| Prueba | Resultado | Evidencia |
|---|---|---|
| Importar el DOCX en ambas apps | GUI PASS: 20 segmentos en ambas | [Trados](../../output/verification/trados-bench-20261004/trados-editor.png), [LumenCAT nueva UI](../../output/verification/trados-bench-20261004-ui/20261004-232851-0207-before-snapshot.png) |
| Editar y confirmar título en LumenCAT | GUI PASS: destino persistido, estado confirmado, avance a fila 2 | [Confirmación](../../output/verification/trados-bench-20261004-ui/20261004-232942-5586-before-snapshot.png) |
| Cerrar/reabrir nueva build | GUI PASS: título y estado recuperados; TM aprendida visible | [Reapertura](../../output/verification/trados-bench-20261004-ui/20261004-233149-9882-before-snapshot.png) |
| Editar Trados | GUI PASS mediante paste; entrada por caracteres incompleta con este transporte | [Texto pegado](../../output/verification/trados-bench-20261004/trados-pasted.png) |
| Exportación LumenCAT con destinos vacíos | Rechazo observado, sin DOCX de destino; se completaron los destinos antes de exportar | `output/verification/trados-bench-20261004/lumen-after-export.json` |
| Entrega nativa de ambas apps | WORD PASS: mismo número de páginas/párrafos, geometría de tablas/imágenes/secciones preservada, sin tags filtrados | [Informe LumenCAT](../../output/verification/trados-bench-20261004/word-lumen/word-report.json), [informe Trados](../../output/verification/trados-bench-20261004/word-trados/word-report.json) |
| Render de entregas | PASS: hashes PNG idénticos para página 1 y página 2; revisión visual ejecutada | Carpetas `word-lumen/render/` y `word-trados/render/` dentro de la evidencia |

Entregas: [LumenCAT](../../output/verification/trados-bench-20261004/lumen-target.docx), [Trados](../../output/verification/trados-bench-20261004/trados-target.docx). Es una traducción parcial deliberada para verificar formato. La muestra original presenta una celda con imagen que sobresale a la derecha; ambas entregas reproducen esa geometría. No se corrigió el documento fuente.

## Recursos: ventana final en reposo

Medición simultánea de 30 segundos, 50 muestras por app, 2026-10-05 02:31:51–02:32:20 UTC (2026-10-04 23:31:51–23:32:20 en Santiago). Ambas ventanas se ajustaron a 1280 × 1392 y se comprobó su tamaño. LumenCAT había sido reabierto con el título confirmado y 19 destinos vacíos; Trados conservaba los demás destinos copiados. Son estados cercanos, pero no idénticos.

| Métrica | LumenCAT | Trados |
|---|---:|---:|
| Procesos incluidos | 1 | 7 |
| Working set medio, MiB | 57,54 | 1.202,65 |
| Working set máximo, MiB | 57,87 | 1.202,68 |
| Private bytes medios, MiB | 82,44 | 908,79 |
| CPU media / máxima, % de la máquina | 0,00 / 0,00 | 0,00 / 0,00 |

Datos crudos: [resources.csv](../../output/verification/trados-bench-20261004/idle-matched/resources.csv), [summary.json](../../output/verification/trados-bench-20261004/idle-matched/summary.json).

El [sampler](../../scripts/utils/measure_app_resources.ps1) identifica cada raíz por PID y hora de inicio, descubre descendientes y suma working set, private bytes y deltas de CPU. Normaliza CPU por los 12 procesadores lógicos. El primer intervalo no tiene porcentaje CPU. El intervalo nominal es 500 ms más el coste de enumeración; esta pasada observó aproximadamente 600 ms entre muestras. Un proceso de control con carga CPU produjo 13,90% medio, verificando que el contador no está fijado en cero: [control](../../output/verification/trados-bench-20261004/sampler-check/summary.json).

### Límites del benchmark

- Una sesión por producto, sin estadística entre repeticiones. Trados llevaba más tiempo abierto y tenía más subsistemas; Windows puede recortar working sets. Private bytes y working set describen magnitudes distintas.
- La suma de working sets puede contar páginas compartidas varias veces. No equivale a memoria física exclusiva. No se midieron VRAM, GPU, energía ni disco.
- Cero CPU significa que no aumentó el contador observado durante esta ventana; no garantiza consumo nulo en otras fases. Procesos que nacen y mueren entre muestras pueden quedar fuera.
- Las capturas auxiliares `idle-empty`, `import`, `idle-loaded`, `idle-complete` e `idle-final` mezclan estados y automatización distintos. Se conservan como diagnóstico; sus duraciones/picos no se comparan como tiempo de importación o rendimiento de edición.
- Un DOCX pequeño con formato complejo no prueba escalabilidad. No se midieron cold start, latencia de escritura, documentos extensos, QA global ni rendimiento TM equivalente. No se deduce superioridad general ni paridad de features.

### Repetir el contador con PIDs actuales

```powershell
& scripts/utils/measure_app_resources.ps1 -RootProcessIds @(1234,5678) -Names @('Trados','LumenCAT') -OutputDirectory output/verification/otra-medicion -DurationSeconds 30
```

Check reproducible de CPU, con un proceso de prueba que termina solo:

```powershell
$worker = Start-Process pwsh -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-Command','$t=[Diagnostics.Stopwatch]::StartNew(); while($t.Elapsed.TotalSeconds -lt 20){for($i=0;$i -lt 10000;$i++){[void][Math]::Sqrt($i)}}'
& scripts/utils/measure_app_resources.ps1 -RootProcessIds @($worker.Id) -Names @('cpu-check') -OutputDirectory output/verification/control-cpu -DurationSeconds 10
$result = Get-Content -Raw output/verification/control-cpu/summary.json | ConvertFrom-Json
if ($result.CpuMeanPercentMachine -le 0) { throw 'No se detectó CPU en el proceso de control' }
```

## Mejora UI ejecutada

Estado entre origen y destino, orden accesible consistente con el visual, cabeceras «ORIGEN»/«DESTINO»/«DOCUMENTOS» y badges «Borrador»/«Confirmado»/«Bloqueado». Botón visible «Confirmar» más compacto; conserva etiqueta accesible «Confirmar y avanzar» y declara Ctrl+Enter. El flujo y el núcleo no cambiaron. Se actualizaron las recetas de verificación afectadas.

Validación ejecutada: `cargo fmt --check`, test focalizado `cargo test --locked --lib gpui_app` (1 PASS), build release, detector Impeccable sin hallazgos y GUI de importación/edición/confirmación/reapertura. Las pruebas de exportación documental usaron la build anterior; el cambio posterior toca solo presentación/semántica accesible. Las dos instancias propias se cerraron por controlador conservando evidencia; no se cerró la instancia del usuario ni Trados.

Sigue pendiente la densidad equivalente a Trados, visualización de formato inline, QA de archivo y un benchmark con corpus grande y cargas equivalentes. No se implementaron como parte de esta mejora acotada.

Los cambios están locales. El remoto `main` se comprobó en `e493199`; no se ejecutó commit, merge ni push. El checkout ya contenía el refactor GPUI pendiente al comenzar; se conservó.
