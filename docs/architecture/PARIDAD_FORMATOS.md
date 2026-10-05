# Mapa de ejecución: paridad documental y de memorias con Trados

## Destino

LumenCAT debe importar, traducir y generar los tipos documentales de la matriz Trados, y leer/actualizar los tipos de memoria de archivo correspondientes. La aceptación se mide por operación y variante, no por extensión. **El objetivo completo sigue abierto.**

Encargo del usuario: implementar lo necesario, trabajar principalmente en `main` local y publicar cortes verificados periódicamente en GitHub `main`. Este mapa acompaña la ejecución; no reemplaza la implementación por una investigación ni reduce silenciosamente el alcance.

## Decisiones tomadas

- [Inventario documental de Trados](../technical/PARIDAD_FORMATOS_TRADOS_FUENTES.md): base Studio 2026/Studio19 `19.0.0.3043`, perfiles/versiones, plugins separados y criterios de aceptación por familia. Manifiestos y documentación no acreditan licencia activa ni ejecución.
- [Inventario de memorias Trados](../technical/PARIDAD_MEMORIAS_TRADOS_FUENTES.md): SDLTM no se actualiza mediante SQL propio; los índices/tokenización requieren un motor que los mantenga.
- [XLIFF con códigos](../technical/XLIFF_INLINE_20261005.md): conservar etiquetas nativas sobre el original y reutilizar los códigos protegidos del editor.
- [TMX con códigos](../technical/TMX_INLINE_20261005.md): conservar unidades, propiedades y variantes; permitir formato específico de cada idioma sin inventar etiquetas en otro.
- [SDLTM mediante SDK](../technical/SDLTM_SDK_20261005.md): puente opcional, explícito, sobre instantáneas/copia nueva; requiere instalación de Trados y no redistribuye sus DLL. No sustituye al futuro lector sin SDK.

## Entregas y estado

| Entrega | Estado probado | Falta para cerrar su alcance |
|---|---|---|
| GPUI Kit, cinta, temas, Archivo/cierre | Publicado `efb5c85`; núcleo y GUI de su build documentados | No implica nuevos formatos; la instancia del usuario conserva esa build |
| XLIFF 1.2 con inline | Publicado `04b10f7`; núcleo/artefactos sintéticos | GUI/consumidor externo de nuevos códigos; segmentación y extensiones |
| TMX con códigos/metadatos TU | Publicado `f5fbb7d`; núcleo/artefactos, ampliado con fixture del SDK | UTF-16, `sub`, cabecera original y aprendizaje de códigos |
| Importar/actualizar copia SDLTM | CORE+SDK PASS; 43→44 TUs, corrección, índices exact/fuzzy y originales intactos | GUI, más fixtures/versiones y funcionamiento sin SDK |

No combinar pruebas de builds distintas como si acreditasen una única build final. Tests del núcleo no prueban GUI; consumir una fixture mediante SDK no prueba todos los filtros de Trados.

## Frontera de trabajo

1. **Bilingües:** completar XLIFF 1.2 segmentado (`seg-source`, `mrk mtype="seg"`), `sub`, pares superpuestos; XLIFF 2.0; SDLXLIFF y otros dialectos con sus estados/skeleton, separando edición de generación nativa.
2. **Memorias:** lectura SDLTM sin SDK; importación/edición/aprendizaje de códigos; actualización de variantes y fidelidad TMX completa; legados TMW/MDB/Workbench con motores/conversión verificables.
3. **Oficina:** ampliar DOCX (historias, campos, enlaces, revisiones, controles y variantes); XLSX/PPTX/ODF/Visio con conservación por partes y consumidor independiente.
4. **Texto/web/software:** JSON, CSV/TAB, XML/perfiles/RESX, PO/properties, SRT/VTT y otros subtítulos; HTML, Markdown, YAML, correo y recursos .NET según los contratos del inventario.
5. **Maquetación/conversión:** IDML/ICML/INX/MIF/Quark/PSD y Office binario/PDF, con motores maduros cuando reduzcan complejidad. PDF de Trados entrega Word, no round-trip PDF.

La matriz documental vinculada contiene el detalle exhaustivo de familias/extensiones; esta lista es el índice, no una sustitución de esa matriz. Los plugins investigados permanecen separados del producto base.

## Decisiones aún abiertas

- Motor externo abierto para ampliar filtros (Okapi/Tikal), despliegue Java, versiones/skeleton y conservación. Evaluarlo contra corpus antes de integrar; no prometer cobertura por catálogo.
- Conversión de binarios con LibreOffice/Word u otros motores: declarar cambio de formato y pérdidas; no llamar conservación sin pérdidas a una conversión.
- Licencias/consumidores disponibles para Adobe, formatos legados y plugins. No comprar, activar, redistribuir runtimes comerciales ni conectar servidores externos como efecto lateral del encargo.
- Corpus por perfil, límites de tamaños y ajustes de extracción equivalentes a Trados. La matriz de fuentes no sustituye esas pruebas.

## Puerta de entrega

Cada corte debe incluir código, un check observable, documentación de límites y validación de calidad. Probar importación→edición→persistencia→exportación→consumo independiente, con positivos, negativos y original intacto. Usar proyectos/instancias desechables en `output/verification/`, controlador en cola y ninguna toma del foreground.

La instancia `file-menu-final-20261005` se dejó al usuario; posteriormente su PID ya no estaba activo y se observó otra desde `output/lumencat.exe`. No conducir ninguna instancia del usuario: comprobar proceso/cola frescos y coordinar antes de una nueva sesión GUI. Los avances publicados no sustituyen automáticamente el ejecutable diario.
