# Verificación de LumenCAT

- Para E2E, bugs de interfaz y validación de features, lee `.cursor/skills/verify-lumencat/SKILL.md` y su mapa `features/README.md`.
- GPUI es la superficie principal; `--legacy-egui` tiene una receta independiente. Usa instancias y proyectos desechables bajo `output/verification/`.
- La app es un recurso en cola (`output/verification/.app-lock.json`): usa el controlador con `-WaitSeconds` y nunca lances el EXE por tu cuenta.
- Distingue guías disponibles de recorridos ejecutados. Las pruebas del núcleo no acreditan interacción GUI ni compatibilidad con Word/Trados.
- Conserva evidencia y limita cleanup a procesos creados por la ejecución. No tomes el foreground sin autorización.
