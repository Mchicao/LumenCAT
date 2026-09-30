//! DOCX Transitional conservador: reúne texto entre runs con formato idéntico;
//! el paquete original esqueleto evita volver a serializar partes ajenas.
use crate::model::{
    Cancellation, CatError, DocumentFormat, ImportedDocument, ImportedSegment, Result, SegmentState,
};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::{
    collections::HashSet,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const MAIN: &str = "word/document.xml";
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const MAX_PART: u64 = 32 * 1024 * 1024;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;

fn invalid(message: impl Into<String>) -> CatError {
    CatError::Format(message.into())
}
fn zip_error(error: zip::result::ZipError) -> CatError {
    invalid(format!("Paquete DOCX: {error}"))
}
fn attr(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|e| invalid(e.to_string()))?;
        if attribute.key.as_ref() == key {
            return Ok(Some(
                attribute
                    .unescape_value()
                    .map_err(|e| invalid(e.to_string()))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

/// Valida XML sin DTD, entidades externas ni codificaciones implícitas.
fn validate_xml(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("DOCX requiere XML UTF-8"))?;
    if text.chars().any(|c| !super::xml_char(c)) {
        return Err(invalid("Carácter XML DOCX inválido"));
    }
    let mut reader = Reader::from_str(text);
    let mut depth = 0_usize;
    let mut roots = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| invalid(format!("XML DOCX: {e}")))?;
        match &event {
            Event::DocType(_) => return Err(invalid("DTD no permitido en DOCX")),
            Event::Decl(decl) => {
                if let Some(encoding) = decl.encoding() {
                    let encoding = encoding.map_err(|e| invalid(e.to_string()))?;
                    if !encoding.eq_ignore_ascii_case(b"utf-8") {
                        return Err(invalid("Codificación DOCX no soportada"));
                    }
                }
            }
            Event::Start(element) | Event::Empty(element) => {
                // Leer todos los atributos también detecta duplicados y escapes inválidos.
                for attribute in element.attributes() {
                    attribute
                        .map_err(|e| invalid(e.to_string()))?
                        .unescape_value()
                        .map_err(|e| invalid(e.to_string()))?;
                }
                if attr(element, b"ContentType")?.is_some_and(|v| {
                    let v = v.to_ascii_lowercase();
                    v.contains("macroenabled") || v.contains("vbaproject")
                }) {
                    return Err(invalid("Contenido de macros Word no soportado"));
                }
                if depth == 0 {
                    roots += 1;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
                if depth > 128 {
                    return Err(invalid("XML DOCX demasiado anidado"));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(character) = reference
                    .resolve_char_ref()
                    .map_err(|e| invalid(e.to_string()))?
                {
                    if !super::xml_char(character) {
                        return Err(invalid("Referencia a carácter inválido en XML DOCX"));
                    }
                } else {
                    let entity = reference.decode().map_err(|e| invalid(e.to_string()))?;
                    quick_xml::escape::unescape(&format!("&{entity};"))
                        .map_err(|e| invalid(e.to_string()))?;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Cierre XML inesperado"))?;
            }
            Event::Text(t) => {
                if depth == 0
                    && !t
                        .decode()
                        .map_err(|e| invalid(e.to_string()))?
                        .trim()
                        .is_empty()
                {
                    return Err(invalid("Texto fuera de raíz XML"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 || roots != 1 {
        return Err(invalid("XML DOCX incompleto o múltiples raíces"));
    }
    Ok(())
}

struct TextSlot {
    start: usize,
    end: usize,
}

struct ParagraphSlot {
    texts: Vec<TextSlot>,
    text: String,
}

fn validate_opc_namespace(text: &str, relationships: bool) -> Result<()> {
    let (root, children, namespace): (&[u8], &[&[u8]], &str) = if relationships {
        (
            b"Relationships",
            &[b"Relationship"],
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )
    } else {
        (
            b"Types",
            &[b"Default", b"Override"],
            "http://schemas.openxmlformats.org/package/2006/content-types",
        )
    };
    let mut reader = Reader::from_str(text);
    let mut seen_root = false;
    loop {
        match reader
            .read_event()
            .map_err(|error| invalid(error.to_string()))?
        {
            Event::Start(e) | Event::Empty(e) => {
                if !seen_root {
                    if e.name().as_ref() != root
                        || attr(&e, b"xmlns")?.as_deref() != Some(namespace)
                    {
                        return Err(invalid("Namespace OPC no soportado o inválido"));
                    }
                    seen_root = true;
                } else if !children.contains(&e.name().as_ref())
                    || attr(&e, b"xmlns")?.is_some_and(|value| value != namespace)
                {
                    return Err(invalid(
                        "Elemento o redefinición de namespace OPC no soportado",
                    ));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn validate_relationship_targets(name: &str, text: &str, parts: &HashSet<String>) -> Result<()> {
    let base = if name == "_rels/.rels" {
        ""
    } else {
        name.rsplit_once("/_rels/")
            .map(|(base, _)| base)
            .ok_or_else(|| invalid("Ubicación de relaciones OPC no soportada"))?
    };
    let mut reader = Reader::from_str(text);
    let mut ids = HashSet::new();
    loop {
        match reader
            .read_event()
            .map_err(|error| invalid(error.to_string()))?
        {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"Relationship" => {
                let id = attr(&e, b"Id")?
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| invalid("Relación OPC sin ID"))?;
                if !ids.insert(id) {
                    return Err(invalid("ID de relación OPC duplicado"));
                }
                if !matches!(attr(&e, b"TargetMode")?.as_deref(), None | Some("Internal")) {
                    return Err(invalid("TargetMode OPC no soportado"));
                }
                let relationship_type = attr(&e, b"Type")?
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| invalid("Relación OPC sin Type"))?;
                if relationship_type
                    .to_ascii_lowercase()
                    .contains("vbaproject")
                {
                    return Err(invalid("Contenido de macros Word no soportado"));
                }
                let target = attr(&e, b"Target")?
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| invalid("Relación OPC sin destino"))?;
                if target.contains(['%', '#', '?', '\\', ':']) {
                    return Err(invalid("URI de relación OPC codificada o no soportada"));
                }
                let mut path: Vec<&str> = if target.starts_with('/') {
                    Vec::new()
                } else {
                    base.split('/').filter(|p| !p.is_empty()).collect()
                };
                for component in target.split('/') {
                    match component {
                        "" | "." => {}
                        ".." => {
                            if path.pop().is_none() {
                                return Err(invalid("Relación OPC fuera del paquete"));
                            }
                        }
                        component => path.push(component),
                    }
                }
                if !parts.contains(&path.join("/").to_ascii_lowercase()) {
                    return Err(invalid("Relación OPC apunta a una parte inexistente"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}
fn text_slots(xml: &[u8], cancel: &Cancellation) -> Result<Vec<ParagraphSlot>> {
    let text = std::str::from_utf8(xml).map_err(|e| invalid(e.to_string()))?;
    let mut reader = Reader::from_str(text);
    let mut slots = Vec::new();
    let mut paragraph = false;
    let mut paragraph_text = String::new();
    let mut paragraph_texts = Vec::new();
    let mut paragraph_run_style: Option<Vec<u8>> = None;
    let mut run_style: Option<Vec<u8>> = None;
    let mut run_style_start = None;
    let mut run_has_style = false;
    let mut run_has_text = false;
    let mut active: Option<(usize, String)> = None;
    let mut root = false;
    let mut stack: Vec<Vec<u8>> = Vec::new();
    loop {
        cancel.check()?;
        let before = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(|e| invalid(e.to_string()))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let name = element.name();
                let name = name.as_ref();
                if !root {
                    if name != b"w:document" || attr(element, b"xmlns:w")?.as_deref() != Some(W_NS)
                    {
                        return Err(invalid(
                            "Solo DOCX Transitional con namespace w estándar está soportado",
                        ));
                    }
                    root = true;
                }
                if !name.starts_with(b"w:")
                    || attr(element, b"xmlns:w")?.is_some_and(|namespace| namespace != W_NS)
                {
                    return Err(invalid(
                        "Namespaces o contenido Word alternativo no soportados",
                    ));
                }
                let local = name.split(|&b| b == b':').next_back().unwrap_or(name);
                if matches!(
                    local,
                    b"hyperlink"
                        | b"fldSimple"
                        | b"fldChar"
                        | b"instrText"
                        | b"ins"
                        | b"del"
                        | b"moveFrom"
                        | b"moveTo"
                        | b"sdt"
                        | b"txbxContent"
                        | b"bookmarkStart"
                        | b"bookmarkEnd"
                        | b"drawing"
                        | b"pict"
                        | b"object"
                        | b"altChunk"
                        | b"tab"
                        | b"br"
                        | b"cr"
                        | b"sym"
                        | b"footnoteReference"
                        | b"endnoteReference"
                        | b"commentReference"
                        | b"commentRangeStart"
                        | b"commentRangeEnd"
                ) {
                    return Err(invalid(format!(
                        "Contenido Word no soportado: {}. El original se conserva.",
                        String::from_utf8_lossy(local)
                    )));
                }
                if active.is_some() {
                    return Err(invalid("Marcado anidado dentro de texto Word"));
                }
                if stack.last().map(Vec::as_slice) == Some(b"w:r")
                    && !matches!(name, b"w:rPr" | b"w:t")
                {
                    return Err(invalid(format!(
                        "Contenido inline Word no traducible: {}",
                        String::from_utf8_lossy(local)
                    )));
                }
                if name == b"w:p" {
                    if paragraph
                        || !matches!(stack.last().map(Vec::as_slice), Some(b"w:body" | b"w:tc"))
                    {
                        return Err(invalid("Párrafos Word anidados"));
                    }
                    paragraph = true;
                    paragraph_text.clear();
                    paragraph_texts.clear();
                    paragraph_run_style = None;
                } else if name == b"w:r" {
                    if !paragraph || stack.last().map(Vec::as_slice) != Some(b"w:p") {
                        return Err(invalid("Run Word fuera de un párrafo simple"));
                    }
                    run_style = None;
                    run_has_style = false;
                    run_has_text = false;
                } else if name == b"w:rPr" && stack.last().map(Vec::as_slice) == Some(b"w:pPr") {
                    // Paragraph-mark formatting does not affect the text runs.
                } else if name == b"w:rPr" {
                    if stack.last().map(Vec::as_slice) != Some(b"w:r")
                        || run_has_style
                        || run_has_text
                    {
                        return Err(invalid(
                            "Propiedades de run Word duplicadas o fuera de lugar",
                        ));
                    }
                    run_has_style = true;
                    if matches!(event, Event::Empty(_)) {
                        run_style = Some(
                            text.as_bytes()[before..reader.buffer_position() as usize].to_vec(),
                        );
                    } else {
                        run_style_start = Some(before);
                    }
                } else if name == b"w:t" {
                    if !paragraph
                        || stack.last().map(Vec::as_slice) != Some(b"w:r")
                        || matches!(event, Event::Empty(_))
                    {
                        return Err(invalid(
                            "Texto Word requiere un w:t no vacío dentro de un run",
                        ));
                    }
                    run_has_text = true;
                    for attribute in element.attributes() {
                        let attribute = attribute.map_err(|e| invalid(e.to_string()))?;
                        if attribute.key.as_ref() != b"xml:space" {
                            return Err(invalid("Atributo w:t no soportado"));
                        }
                    }
                    active = Some((before, String::new()));
                }
                if matches!(event, Event::Empty(_)) && name == b"w:p" {
                    paragraph = false;
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(name.to_vec());
                }
            }
            Event::End(element) => {
                let name = element.name();
                if name.as_ref() == b"w:rPr" {
                    if let Some(start) = run_style_start.take() {
                        run_style = Some(
                            text.as_bytes()[start..reader.buffer_position() as usize].to_vec(),
                        );
                    }
                } else if name.as_ref() == b"w:r" {
                    let style = run_style.take().unwrap_or_default();
                    if paragraph_run_style
                        .as_ref()
                        .is_some_and(|expected| expected != &style)
                    {
                        return Err(invalid(
                            "Párrafo con formato mixto entre runs: tags protegidos aún no soportados",
                        ));
                    }
                    paragraph_run_style.get_or_insert(style);
                } else if name.as_ref() == b"w:t" {
                    let (start, value) = active
                        .take()
                        .ok_or_else(|| invalid("Texto Word sin apertura"))?;
                    if value.is_empty() || value.chars().any(|c| !super::xml_char(c)) {
                        return Err(invalid(
                            "w:t vacío o con caracteres XML inválidos no soportado",
                        ));
                    }
                    if value.contains(['\r', '\n', '\t']) {
                        return Err(invalid(
                            "Texto DOCX con saltos de línea o tabulaciones no soportado; requieren tags Word protegidos",
                        ));
                    }
                    paragraph_text.push_str(&value);
                    paragraph_texts.push(TextSlot {
                        start,
                        end: reader.buffer_position() as usize,
                    });
                }
                stack.pop();
                if name.as_ref() == b"w:p" {
                    paragraph = false;
                    if !paragraph_texts.is_empty() {
                        slots.push(ParagraphSlot {
                            texts: std::mem::take(&mut paragraph_texts),
                            text: std::mem::take(&mut paragraph_text),
                        });
                    }
                }
            }
            Event::CData(_) => return Err(invalid("CDATA Word no soportado")),
            Event::Comment(_) | Event::PI(_) if active.is_some() => {
                return Err(invalid("Marcado dentro de texto Word no soportado"));
            }
            Event::Text(t) => {
                if let Some((_, value)) = active.as_mut() {
                    if let Some(decoded) = super::text_event(&Event::Text(t))? {
                        value.push_str(&decoded);
                    }
                } else if !t
                    .decode()
                    .map_err(|e| invalid(e.to_string()))?
                    .trim()
                    .is_empty()
                {
                    return Err(invalid("Texto Word fuera de w:t"));
                }
            }
            Event::GeneralRef(reference) if active.is_some() => {
                if let Some(decoded) = super::text_event(&Event::GeneralRef(reference))?
                    && let Some((_, value)) = active.as_mut()
                {
                    value.push_str(&decoded);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if slots.is_empty() {
        return Err(invalid("DOCX sin párrafos traducibles"));
    }
    Ok(slots)
}

fn inspect(bytes: &[u8], cancel: &Cancellation) -> Result<Vec<ParagraphSlot>> {
    let eocd = bytes
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .ok_or_else(|| invalid("ZIP sin directorio final"))?;
    let tail = bytes
        .get(eocd..)
        .filter(|t| t.len() >= 22)
        .ok_or_else(|| invalid("ZIP truncado"))?;
    let count = u16::from_le_bytes([tail[10], tail[11]]) as usize;
    let comment = u16::from_le_bytes([tail[20], tail[21]]) as usize;
    if tail.len() != 22 + comment
        || count > 2048
        || tail[4..8] != [0, 0, 0, 0]
        || tail[8..10] != tail[10..12]
    {
        return Err(invalid("ZIP multipart, ZIP64 o directorio inválido"));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;
    // zip indexa por nombre; el contador original detecta nombres duplicados descartados por su mapa.
    if archive.len() != count {
        return Err(invalid("Entradas ZIP duplicadas"));
    }
    let mut names = HashSet::new();
    let parts: HashSet<String> = archive.file_names().map(str::to_ascii_lowercase).collect();
    let mut spans = Vec::new();
    let mut total = 0_u64;
    let mut main = None;
    let mut content_type = false;
    let mut office_relationship = false;
    for i in 0..archive.len() {
        cancel.check()?;
        let mut part = archive.by_index(i).map_err(zip_error)?;
        let name = part.name().to_owned();
        let lower_name = name.to_ascii_lowercase();
        if !names.insert(lower_name.clone())
            || name.contains('\\')
            || name.starts_with('/')
            || name.split('/').any(|p| p == "..")
        {
            return Err(invalid("Nombre de parte DOCX duplicado o inseguro"));
        }
        if lower_name.contains("vba") || lower_name.starts_with("word/embeddings/") {
            return Err(invalid("Macros u objetos embebidos no soportados"));
        }
        total = total
            .checked_add(part.size())
            .ok_or_else(|| invalid("Tamaño DOCX desbordado"))?;
        if part.size() > MAX_PART
            || total > MAX_TOTAL
            || part.size()
                > part
                    .compressed_size()
                    .saturating_mul(1000)
                    .saturating_add(1024)
        {
            return Err(invalid("DOCX supera límites de expansión seguros"));
        }
        let start = part.data_start();
        let end = start
            .checked_add(part.compressed_size())
            .ok_or_else(|| invalid("Parte ZIP inválida"))?;
        if end > bytes.len() as u64 || spans.iter().any(|&(a, b)| start < b && a < end) {
            return Err(invalid("Partes ZIP solapadas o truncadas"));
        }
        spans.push((start, end));
        let mut data = Vec::new();
        part.by_ref().take(MAX_PART + 1).read_to_end(&mut data)?;
        if data.len() as u64 != part.size() {
            return Err(invalid("Tamaño real ZIP inválido"));
        }
        if lower_name.ends_with(".xml") || lower_name.ends_with(".rels") {
            validate_xml(&data)?;
            let text = std::str::from_utf8(&data).map_err(|e| invalid(e.to_string()))?;
            if lower_name.ends_with(".rels") || name == "[Content_Types].xml" {
                validate_opc_namespace(text, lower_name.ends_with(".rels"))?;
            }
            if lower_name.ends_with(".rels") {
                validate_relationship_targets(&name, text, &parts)?;
            }
            if name == "[Content_Types].xml" || name == "_rels/.rels" {
                let mut reader = Reader::from_str(text);
                loop {
                    match reader.read_event().map_err(|e| invalid(e.to_string()))? {
                        Event::Start(e) | Event::Empty(e) => {
                            if name == "[Content_Types].xml"
                                && e.local_name().as_ref() == b"Override"
                                && attr(&e, b"PartName")?.as_deref() == Some("/word/document.xml")
                                && attr(&e, b"ContentType")?.as_deref()
                                    == Some(
                                        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
                                    )
                            {
                                content_type = true;
                            }
                            if name == "_rels/.rels"
                                && e.local_name().as_ref() == b"Relationship"
                                && attr(&e, b"Type")?.as_deref()
                                    == Some(
                                        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
                                    )
                                && attr(&e, b"Target")?.as_deref() == Some(MAIN)
                            {
                                office_relationship = true;
                            }
                        }
                        Event::Eof => break,
                        _ => {}
                    }
                }
            }
            if name != MAIN {
                let mut reader = Reader::from_str(text);
                loop {
                    match reader.read_event().map_err(|e| invalid(e.to_string()))? {
                        Event::Start(e) | Event::Empty(e) => {
                            if e.local_name().as_ref() == b"t" {
                                return Err(invalid(
                                    "Texto en otra parte DOCX (encabezados, notas, glosario u otra historia) todavía no soportado",
                                ));
                            }
                            if e.local_name().as_ref() == b"lvlText" {
                                for attribute in e.attributes() {
                                    let attribute =
                                        attribute.map_err(|error| invalid(error.to_string()))?;
                                    if attribute.key.local_name().as_ref() == b"val"
                                        && attribute
                                            .unescape_value()
                                            .map_err(|error| invalid(error.to_string()))?
                                            .chars()
                                            .any(char::is_alphabetic)
                                    {
                                        return Err(invalid(
                                            "Etiqueta textual de numeración Word no soportada",
                                        ));
                                    }
                                }
                            }
                        }
                        Event::Eof => break,
                        _ => {}
                    }
                }
            }
        }
        if name == MAIN {
            main = Some(data);
        }
    }
    if !content_type || !office_relationship {
        return Err(invalid("DOCX carece de contrato OPC Transitional válido"));
    }
    text_slots(
        &main.ok_or_else(|| invalid("DOCX sin word/document.xml"))?,
        cancel,
    )
}

pub fn import_docx(
    path: &Path,
    sl: &str,
    tl: &str,
    cancel: &Cancellation,
) -> Result<ImportedDocument> {
    if std::fs::metadata(path)?.len() > MAX_TOTAL {
        return Err(invalid("Archivo DOCX demasiado grande"));
    }
    let mut original = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_TOTAL + 1)
        .read_to_end(&mut original)?;
    if original.len() as u64 > MAX_TOTAL {
        return Err(invalid("Archivo DOCX demasiado grande"));
    }
    let slots = inspect(&original, cancel)?;
    Ok(ImportedDocument {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        format: DocumentFormat::Docx,
        original,
        original_path: Some(std::fs::canonicalize(path)?),
        source_lang: sl.into(),
        target_lang: tl.into(),
        segments: slots
            .into_iter()
            .enumerate()
            .map(|(i, s)| ImportedSegment {
                external_id: format!("word-text-{i}"),
                source: s.text,
                target: String::new(),
                state: SegmentState::Draft,
                locked: false,
            })
            .collect(),
    })
}

pub fn serialize_docx(
    document: &ImportedDocument,
    targets: &[String],
    cancel: &Cancellation,
) -> Result<Vec<u8>> {
    let slots = inspect(&document.original, cancel)?;
    if targets.len() != slots.len() || slots.len() != document.segments.len() {
        return Err(invalid("Cantidad de traducciones DOCX no coincide"));
    }
    if slots
        .iter()
        .zip(&document.segments)
        .any(|(slot, segment)| slot.text != segment.source)
    {
        return Err(invalid(
            "El source DOCX no coincide con el esqueleto original",
        ));
    }
    if targets
        .iter()
        .any(|t| t.is_empty() || t.chars().any(|c| !super::xml_char(c)))
    {
        return Err(invalid(
            "Traducción DOCX vacía o con caracteres XML inválidos",
        ));
    }
    if targets.iter().any(|t| t.contains(['\r', '\n', '\t'])) {
        return Err(invalid(
            "Traducción DOCX multilínea o con tabulaciones no válida: este formato requiere w:br o w:tab protegidos, todavía no soportados",
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(&document.original)).map_err(zip_error)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        cancel.check()?;
        let mut part = archive.by_index(i).map_err(zip_error)?;
        if part.name() != MAIN {
            writer.raw_copy_file(part).map_err(zip_error)?;
            continue;
        }
        let mut xml = String::new();
        part.read_to_string(&mut xml)?;
        let mut output = String::with_capacity(xml.len());
        let mut offset = 0;
        for (paragraph, target) in slots.iter().zip(targets) {
            for (index, slot) in paragraph.texts.iter().enumerate() {
                cancel.check()?;
                output.push_str(&xml[offset..slot.start]);
                if index == 0 {
                    output.push_str("<w:t xml:space=\"preserve\">");
                    output.push_str(&super::escaped(target)?);
                    output.push_str("</w:t>");
                }
                offset = slot.end;
            }
        }
        output.push_str(&xml[offset..]);
        writer
            .start_file(
                MAIN,
                SimpleFileOptions::default().compression_method(part.compression()),
            )
            .map_err(zip_error)?;
        writer.write_all(output.as_bytes())?;
    }
    let output = writer.finish().map_err(zip_error)?.into_inner();
    let roundtrip = inspect(&output, cancel)?;
    if roundtrip.iter().map(|s| &s.text).ne(targets.iter()) {
        return Err(invalid("Verificación round-trip DOCX falló; no se exportó"));
    }
    Ok(output)
}
