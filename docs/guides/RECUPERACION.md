# Respaldo y recuperación de proyectos

La versión de esquema 5 conserva documentos, memorias e historial de proyectos v1–v4 y añade bases/conceptos/expresiones terminológicas. Mantiene los idiomas, colecciones y contribuciones aprendidas de v4. Antes de migrar un proyecto existente, LumenCAT crea un respaldo SQLite coherente junto al original, con nombre `<archivo>.backup-v<versión>-<identificador>.lcat`. Incluye los commits todavía presentes en WAL; no es una copia simple del archivo abierto.

## Qué cambia

- Reemplazar en varios segmentos forma una sola operación de deshacer/rehacer, incluso después de reiniciar.
- El historial v1 conserva su orden y cursor, incluidos los pasos disponibles para rehacer.
- Una falla de migración revierte la transacción y conserva el respaldo. Las bases ajenas y versiones futuras se rechazan.
- Un binario anterior no abre una versión de esquema futura: para volver a él, usa una **copia del respaldo de su versión**, no el proyecto actualizado. No se implementa una migración hacia atrás.

## Recuperar sin sobrescribir

1. Cierra el proyecto y conserva el original, incluido su WAL si el cierre falló.
2. Localiza el respaldo. No lo renombres encima del original ni copies WAL/SHM al respaldo.
3. Abre una copia recuperada con un nombre nuevo. Comprueba documentos, traducciones y último paso del historial.

En este corte, respaldo manual y recuperación están disponibles en el núcleo Rust (`ProjectStore::backup_to`, `recover_copy`, `open_existing`); sus diálogos de interfaz todavía no están implementados. Ambos rechazan destinos existentes y validan integridad. `open_existing` rechaza rutas ausentes sin crear un proyecto vacío.

## Verificar el corte

```powershell
cargo test --locked --lib migration_tests
cargo test --locked --test storage --test replacement --test recovery
```

Las pruebas usan directorios temporales: verifican texto Unicode, memoria, cursor heredado, respaldo con WAL, rollback de migración fallida, conflictos revisionados y grupos de undo/redo tras reiniciar. No abren proyectos del usuario.

La frontera de rollback de esta entrega es `src/storage.rs`, `src/storage/migrations.rs` y sus pruebas. Revertir código no convierte un `.lcat` migrado a v1: conserva y recupera su respaldo anterior.
