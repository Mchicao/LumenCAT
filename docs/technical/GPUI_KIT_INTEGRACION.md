# GPUI Kit en LumenCAT

La interfaz usa GPUI Kit 0.7.1 para botones de acciones/historial, iconos Lucide y grupos de selección de apariencia. Se conservan la cinta, el editor bilingüe, los proyectos y las seis combinaciones de apariencia. La biblioteca no incorpora adaptadores de documentos ni un sistema de plugins.

## Dependencia y arranque

- `Cargo.toml` importa `gpui-kit =0.7.1` con el nombre `gpui`, para conservar las referencias y macros existentes. Kit fija su familia de motor `gpui-pre =0.3.8`; el lock no mantiene en paralelo el checkout GPUI anterior.
- `runtime.rs` registra los iconos incluidos en Kit, inicializa los componentes y abre la ventana mediante `gpui::open_window`. Su Root proporciona la infraestructura para foco y tooltips.
- `components.rs` devuelve `gpui::component::button::Button`; desaparecieron el dibujado propio de botones y flechas de historial. Kit controla la activación por ratón/teclado y el deshabilitado; no se añaden callbacks de teclado duplicados.
- Configuración usa `Radio` y `RadioGroup`. Una selección guarda primero el JSON: si falla, conserva la selección y paleta anterior. `Theme::sync_components` proyecta el modo/acento sobre los componentes mediante la API oficial `Theme::update`, que sincroniza los tokens y el Root.
- La entrada de texto, los códigos protegidos y la virtualización del editor siguen siendo propios. No se instaló Ely ni se modificó el núcleo de documentos.

## Comprobaciones

Comprobado: `cargo check --locked --all-targets` y árbol de dependencias con un solo `gpui-pre`. Registro: `logs/tests/gpui-kit-check.log` y `logs/tests/gpui-kit-dependencies.log`.

CORE PASS: formato, Clippy all-targets con `-D warnings`, 47 pruebas sin fallos y build release. Registro: `logs/tests/gpui-kit-validation.log`. El contraste comprobado incluye los colores usados por los botones de Kit en las seis combinaciones de apariencia.

La primera release se abrió mediante el controlador en `output/verification/gpui-kit-20261005/` y se comprobó visualmente la cinta, Archivo y sus iconos. SHA-256: `82A8DD55FA41C8FFE675064033C34CC50101B4D574DBF9D4DA12C72D11730756`. Se dejó abierta al usuario; al iniciar la pasada siguiente ya estaba cerrada. No se atribuyen a ese artefacto los recorridos de builds posteriores.

### Archivo y confirmación de cierre

Archivo ahora reemplaza la cinta con navegación azul y acciones a la derecha, inspiradas en la captura de Trados. Muestra Abrir, Documentos del proyecto y Nuevo; no inventa recientes ni funciones de paquetes/nube. Guardar destino como reutiliza la exportación protegida. Salir y la X comparten un diálogo real de Kit con Guardar y salir, Salir sin guardar y Cancelar. El autoguardado se pausa durante la decisión; descartar nunca revierte commits ni operaciones en curso.

La release final `D79F5A282619B2A43621593AD78212D67066AED8A7951B533D204B4D4E2C3186` aprobó formato, Clippy y **48 pruebas**. En `output/verification/file-menu-final-20261005/results.md` constan recorridos GUI: importación XLIFF, Archivo/documentos, ambos accesos al diálogo, cancelación con borrador, fallo de guardado provocado mediante un bloqueo temporal del proyecto desechable, reintento/guardado/reapertura, descarte/reapertura y exportación XLIFF con original intacto. Se inspeccionó Archivo claro/oscuro. Las instancias previas conservan resultados separados; una recibió cambios ajenos al recorrido y se pausó antes de continuar con autorización del usuario.

La build final quedó abierta para el usuario en una instancia aislada. Siguen pendientes el recorrido completo del historial, todos los temas/atajos, IME/RTL y lectores de pantalla con este motor. El EXE diario no se sustituyó. Logs: `logs/tests/file-menu-exit-final-{validation,release}.log`.

Para repetir las comprobaciones del código:

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin lumencat
```

Para GUI, sigue `.cursor/skills/verify-lumencat/SKILL.md` y su receta `features/ribbon.md`; lanza únicamente mediante `scripts/utils/control_lumencat.ps1` con `-Configuration release -WaitSeconds 900` y un RunId nuevo. Comprueba acciones habilitadas/deshabilitadas, edición/confirmación, historial, temas, reapertura y exportación sin alterar el original. Usa preferencias propias del RunId.

## Fuentes y límites

- [Instalación oficial](https://gpui-kit.com/docs/installation).
- [GPUI Kit publicado](https://crates.io/crates/gpui-kit/0.7.1), licencia Apache-2.0.
- [Versiones del motor fijadas por Kit](https://github.com/longbridge/gpui-kit/blob/v0.7.1/Cargo.toml).
- La actualización del motor exige verificación nativa: compilar no acredita IME/RTL, lectores de pantalla ni comportamiento físico de todos los atajos.
- El respaldo de los archivos locales previos vive en `.cache/gpui-kit-baseline-20261005/`. No se limpió la caché Rust ni se sustituyó el EXE de uso diario.
