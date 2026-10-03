//! Adaptadores conservadores: el envelope original se conserva y solo cambia el target.
pub mod docx;

use crate::model::*;
use quick_xml::{Reader, Writer, events::Event};
use std::{
    fs,
    io::{BufRead, Write},
    path::Path,
};

const MAX_DOCUMENT: u64 = 256 * 1024 * 1024;
const MAX_TEXT: usize = 4 * 1024 * 1024;
const MAX_TU: usize = 8 * 1024 * 1024;
fn invalid(message: impl ToString) -> CatError {
    CatError::Format(message.to_string())
}
fn local(name: &[u8]) -> &[u8] {
    name.rsplit(|b| *b == b':').next().unwrap_or(name)
}
fn attr(start: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Result<Option<String>> {
    let mut found = None;
    for a in start.attributes() {
        let a = a.map_err(invalid)?;
        let value = a.unescape_value().map_err(invalid)?;
        if value.chars().any(|c| !xml_char(c)) {
            return Err(invalid("carácter XML inválido en atributo"));
        }
        if a.key.as_ref() == name {
            found = Some(value.into_owned());
        }
    }
    Ok(found)
}
fn escaped(text: &str) -> Result<String> {
    if text.chars().any(|c| !xml_char(c)) {
        return Err(invalid("carácter no permitido por XML 1.0"));
    }
    // Las referencias preservan CR incluso con la normalización XML de fin de línea.
    Ok(quick_xml::escape::escape(text).replace('\r', "&#13;"))
}
fn xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
        || matches!(c, '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}
fn text_event(event: &Event<'_>) -> Result<Option<String>> {
    let text = match event {
        Event::Text(t) => Some(t.xml_content().map_err(invalid)?.into_owned()),
        Event::CData(t) => Some(t.xml_content().map_err(invalid)?.into_owned()),
        Event::GeneralRef(r) => {
            let s = if let Some(c) = r.resolve_char_ref().map_err(invalid)? {
                c.to_string()
            } else {
                quick_xml::escape::unescape(&format!("&{};", r.decode().map_err(invalid)?))
                    .map_err(invalid)?
                    .into_owned()
            };
            Some(s)
        }
        _ => None,
    };
    if text
        .as_ref()
        .is_some_and(|s| s.chars().any(|c| !xml_char(c)))
    {
        return Err(invalid("carácter XML inválido"));
    }
    Ok(text)
}
fn check_decl(event: &Event<'_>) -> Result<()> {
    match event {
        Event::DocType(_) => return Err(invalid("DTD y entidades externas no permitidos")),
        Event::Decl(d) => {
            if d.version().map_err(invalid)?.as_ref() != b"1.0" {
                return Err(invalid("solo XML 1.0 está soportado"));
            }
            if let Some(enc) = d.encoding() {
                let enc = enc.map_err(invalid)?;
                if !enc.eq_ignore_ascii_case(b"utf-8") {
                    return Err(invalid("solo XML UTF-8 está soportado"));
                }
            }
        }
        _ => (),
    }
    Ok(())
}

pub fn import_document(
    path: &Path,
    source_lang: &str,
    target_lang: &str,
    cancel: &Cancellation,
) -> Result<ImportedDocument> {
    cancel.check()?;
    ProjectSettings {
        source_lang: source_lang.into(),
        target_lang: target_lang.into(),
    }
    .validate()?;
    if fs::metadata(path)?.len() > MAX_DOCUMENT {
        return Err(invalid("documento supera límite de 256 MiB"));
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "docx" {
        return docx::import_docx(path, source_lang, target_lang, cancel);
    }
    let original = fs::read(path)?;
    let text = std::str::from_utf8(&original).map_err(invalid)?;
    let (format, segments) = match extension.as_str() {
        "txt" => (
            DocumentFormat::Txt,
            txt_lines(text)
                .into_iter()
                .enumerate()
                .map(|(i, (s, _))| ImportedSegment {
                    external_id: i.to_string(),
                    source: s.to_owned(),
                    target: String::new(),
                    state: SegmentState::Draft,
                    locked: false,
                })
                .collect(),
        ),
        "xlf" | "xliff" => (
            DocumentFormat::Xliff12,
            xliff_with_languages(text, cancel, Some((source_lang, target_lang)))?
                .into_iter()
                .map(|u| u.segment)
                .collect(),
        ),
        _ => return Err(invalid("use TXT UTF-8, XLIFF 1.2 textual o DOCX")),
    };
    cancel.check()?;
    Ok(ImportedDocument {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        original_path: Some(fs::canonicalize(path)?),
        format,
        original,
        source_lang: source_lang.into(),
        target_lang: target_lang.into(),
        segments,
    })
}

fn txt_lines(text: &str) -> Vec<(&str, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut result = Vec::new();
    let mut start = 0;
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if matches!(b[i], b'\r' | b'\n') {
            let end = i;
            i += if b[i] == b'\r' && b.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            result.push((&text[start..end], &text[end..i]));
            start = i;
        } else {
            i += 1;
        }
    }
    if start < text.len() {
        result.push((&text[start..], ""));
    }
    result
}

struct XUnit {
    segment: ImportedSegment,
    opening: (usize, usize),
    approved: bool,
    target: Option<(usize, usize)>,
    insertion: usize,
    target_name: String,
}
fn xliff(text: &str, cancel: &Cancellation) -> Result<Vec<XUnit>> {
    xliff_with_languages(text, cancel, None)
}
fn xliff_with_languages(
    text: &str,
    cancel: &Cancellation,
    languages: Option<(&str, &str)>,
) -> Result<Vec<XUnit>> {
    if text.chars().any(|c| !xml_char(c)) {
        return Err(invalid("carácter XML inválido"));
    }
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut units = Vec::new();
    let mut active: Option<XUnit> = None;
    let mut field: Option<(bool, usize)> = None;
    let mut target_start = 0;
    let mut source_seen = false;
    let mut target_seen = false;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut file_ids = std::collections::HashSet::new();
    loop {
        cancel.check()?;
        let before = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(invalid)?;
        check_decl(&event)?;
        match &event {
            Event::Start(s) | Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                let lname = local(&name);
                // Validate all attributes, including duplicate attributes.
                let _ = attr(s, b"__none")?;
                if stack.is_empty() {
                    if root_seen
                        || root_closed
                        || lname != b"xliff"
                        || attr(s, b"version")?.as_deref() != Some("1.2")
                    {
                        return Err(invalid("se requiere un único xliff version 1.2"));
                    }
                    root_seen = true;
                    let prefix = name.strip_suffix(b"xliff").unwrap_or_default();
                    let namespace_attr = if prefix.is_empty() {
                        "xmlns".to_owned()
                    } else {
                        format!(
                            "xmlns:{}",
                            String::from_utf8_lossy(&prefix[..prefix.len() - 1])
                        )
                    };
                    if attr(s, namespace_attr.as_bytes())?
                        .as_deref()
                        .is_some_and(|ns| ns != "urn:oasis:names:tc:xliff:document:1.2")
                    {
                        return Err(invalid("namespace XLIFF 1.2 incorrecto"));
                    }
                }
                if stack.len() >= 128 {
                    return Err(invalid("XML demasiado profundo"));
                }
                if field.is_some() {
                    return Err(invalid(
                        "inline codes en source/target no soportados todavía",
                    ));
                }
                if lname == b"seg-source" {
                    return Err(invalid(
                        "XLIFF segmentado con seg-source no soportado todavía",
                    ));
                }
                if lname == b"file" {
                    if let Some((source, target)) = languages {
                        for (attribute, configured) in [
                            (b"source-language".as_slice(), source),
                            (b"target-language".as_slice(), target),
                        ] {
                            if let Some(declared) = attr(s, attribute)?
                                && !declared.eq_ignore_ascii_case(configured)
                            {
                                return Err(invalid(format!(
                                    "Conflicto de idioma XLIFF: {} declara {declared}, configurado {configured}. Elige el par del archivo y vuelve a importar; no se relabelan datos.",
                                    String::from_utf8_lossy(attribute)
                                )));
                            }
                        }
                    }
                    if stack.iter().any(|n| local(n) == b"file") {
                        return Err(invalid("file XLIFF anidado"));
                    }
                    file_ids.clear();
                }
                if lname == b"trans-unit" {
                    if active.is_some() {
                        return Err(invalid("trans-unit anidado"));
                    }
                    if !matches!(event, Event::Start(_)) {
                        return Err(invalid("trans-unit vacío"));
                    }
                    let id = attr(s, b"id")?.ok_or_else(|| invalid("trans-unit sin id"))?;
                    if !file_ids.insert(id.clone()) {
                        return Err(invalid("id de trans-unit duplicado dentro de file"));
                    }
                    let approved = attr(s, b"approved")?.as_deref() == Some("yes");
                    let prefix = name.strip_suffix(b"trans-unit").unwrap_or_default();
                    active = Some(XUnit {
                        segment: ImportedSegment {
                            external_id: id,
                            source: String::new(),
                            target: String::new(),
                            state: if approved {
                                SegmentState::Confirmed
                            } else {
                                SegmentState::Draft
                            },
                            locked: attr(s, b"translate")?.as_deref() == Some("no"),
                        },
                        opening: (before, reader.buffer_position() as usize),
                        approved,
                        target: None,
                        insertion: 0,
                        target_name: format!("{}target", String::from_utf8_lossy(prefix)),
                    });
                    source_seen = false;
                    target_seen = false;
                } else if active.is_some()
                    && stack.last().is_some_and(|n| local(n) == b"trans-unit")
                    && matches!(lname, b"source" | b"target")
                {
                    let target = lname == b"target";
                    if if target { target_seen } else { source_seen } {
                        return Err(invalid("source/target duplicado"));
                    }
                    if target {
                        target_seen = true;
                        target_start = before;
                    } else {
                        source_seen = true;
                    }
                    if let Some(u) = active.as_mut() {
                        if target {
                            if matches!(
                                attr(s, b"state")?.as_deref(),
                                Some("translated" | "final" | "signed-off")
                            ) {
                                u.segment.state = SegmentState::Confirmed;
                            }
                            u.target_name = String::from_utf8_lossy(&name).into_owned();
                            if matches!(event, Event::Empty(_)) {
                                u.target = Some((before, reader.buffer_position() as usize));
                            }
                        }
                        if matches!(event, Event::Empty(_)) && !target {
                            u.insertion = reader.buffer_position() as usize;
                        }
                    }
                    if matches!(event, Event::Start(_)) {
                        field = Some((target, stack.len() + 1));
                    }
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(name);
                }
            }
            Event::End(e) => {
                if stack.pop().as_deref() != Some(e.name().as_ref()) {
                    return Err(invalid("XML sin balance"));
                }
                if let Some((target, depth)) = field
                    && stack.len() + 1 == depth
                {
                    if let Some(u) = active.as_mut() {
                        if target {
                            u.target = Some((target_start, reader.buffer_position() as usize));
                        } else {
                            u.insertion = reader.buffer_position() as usize;
                        }
                    }
                    field = None;
                }
                if local(e.name().as_ref()) == b"trans-unit" {
                    if !source_seen {
                        return Err(invalid("trans-unit sin source"));
                    }
                    if let Some(u) = active.take() {
                        units.push(u);
                    }
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Eof => break,
            _ => {
                if let Some(s) = text_event(&event)? {
                    if let Some((target, _)) = field {
                        if let Some(u) = active.as_mut() {
                            let value = if target {
                                &mut u.segment.target
                            } else {
                                &mut u.segment.source
                            };
                            value.push_str(&s);
                            if value.len() > MAX_TEXT {
                                return Err(invalid("segmento supera 4 MiB"));
                            }
                        }
                    } else if stack.is_empty() && !s.trim().is_empty() {
                        return Err(invalid("texto fuera del XML"));
                    }
                }
            }
        }
    }
    if !root_seen || !root_closed || !stack.is_empty() {
        return Err(invalid("XML incompleto"));
    }
    Ok(units)
}

pub fn serialize_document(
    document: &ImportedDocument,
    targets: &[String],
    cancel: &Cancellation,
) -> Result<Vec<u8>> {
    cancel.check()?;
    if targets.len() != document.segments.len() {
        return Err(CatError::Invalid("cantidad de targets no coincide".into()));
    }
    if document.format == DocumentFormat::Docx {
        return docx::serialize_docx(document, targets, cancel);
    }
    let original = std::str::from_utf8(&document.original).map_err(invalid)?;
    let mut output = String::new();
    match document.format {
        DocumentFormat::Txt => {
            let lines = txt_lines(original);
            if lines.len() != targets.len() {
                return Err(invalid("skeleton TXT no coincide"));
            }
            if original.starts_with('\u{feff}') {
                output.push('\u{feff}');
            }
            for ((_, ending), target) in lines.into_iter().zip(targets) {
                cancel.check()?;
                if target.contains(['\r', '\n']) {
                    return Err(invalid("target TXT no puede agregar líneas estructurales"));
                }
                output.push_str(target);
                output.push_str(ending);
            }
        }
        DocumentFormat::Xliff12 => {
            let units = xliff(original, cancel)?;
            if units.len() != targets.len() {
                return Err(invalid("skeleton XLIFF no coincide"));
            }
            let mut cursor = 0;
            for ((u, segment), target) in units.iter().zip(&document.segments).zip(targets) {
                cancel.check()?;
                if u.segment.external_id != segment.external_id
                    || u.segment.source != segment.source
                {
                    return Err(invalid("identidad/source del skeleton no coincide"));
                }
                let (start, end) = u.target.unwrap_or((u.insertion, u.insertion));
                let changed = target != &u.segment.target;
                let mut unit_attributes = Vec::new();
                if u.approved && (changed || segment.state == SegmentState::Draft) {
                    unit_attributes.push(("approved", "no"));
                }
                if segment.locked != u.segment.locked {
                    unit_attributes.push(("translate", if segment.locked { "no" } else { "yes" }));
                }
                if !unit_attributes.is_empty() {
                    output.push_str(&original[cursor..u.opening.0]);
                    output.push_str(&updated_opening(
                        &original[u.opening.0..u.opening.1],
                        &unit_attributes,
                    )?);
                    cursor = u.opening.1;
                }
                output.push_str(&original[cursor..start]);
                let state = match segment.state {
                    SegmentState::Confirmed => "translated",
                    SegmentState::Draft => "needs-review-translation",
                };
                let state_changed = changed || segment.state != u.segment.state;
                let state_attributes = [("state", state)];
                if u.target.is_some() && !state_changed {
                    output.push_str(&original[start..end]);
                    cursor = end;
                    continue;
                }
                // Preserve original target attributes while replacing only content.
                if u.target.is_some() {
                    let raw = &original[start..end];
                    let opening_end = xml_open_end(raw)?;
                    output.push_str(&updated_opening(
                        &raw[..opening_end],
                        if state_changed {
                            &state_attributes
                        } else {
                            &[]
                        },
                    )?);
                } else {
                    output.push_str(&format!("<{} state=\"{}\">", u.target_name, state));
                }
                output.push_str(&escaped(target)?);
                output.push_str(&format!("</{}>", u.target_name));
                cursor = end;
            }
            output.push_str(&original[cursor..]);
            let validated = xliff(&output, cancel)?;
            if validated
                .iter()
                .zip(targets)
                .any(|(u, t)| &u.segment.target != t)
            {
                return Err(invalid("validación del target exportado falló"));
            }
        }
        DocumentFormat::Docx => unreachable!(),
    }
    cancel.check()?;
    Ok(output.into_bytes())
}
fn xml_open_end(raw: &str) -> Result<usize> {
    let mut quote = None;
    for (i, c) in raw.char_indices() {
        match c {
            '\'' | '"' if quote == Some(c) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(c),
            '>' if quote.is_none() => return Ok(i + 1),
            _ => (),
        }
    }
    Err(invalid("tag XML incompleto"))
}

fn updated_opening(raw: &str, replacements: &[(&str, &str)]) -> Result<String> {
    if replacements.is_empty() {
        return Ok(if raw.ends_with("/>") {
            format!("{}>", raw.trim_end_matches("/>").trim_end())
        } else {
            raw.into()
        });
    }
    let mut reader = Reader::from_str(raw);
    let event = reader.read_event().map_err(invalid)?;
    let start = match event {
        Event::Start(s) | Event::Empty(s) => s,
        _ => return Err(invalid("target inválido")),
    };
    let mut updated = quick_xml::events::BytesStart::new(
        std::str::from_utf8(start.name().as_ref())
            .map_err(invalid)?
            .to_owned(),
    );
    for attribute in start.attributes() {
        let attribute = attribute.map_err(invalid)?;
        if !replacements
            .iter()
            .any(|(key, _)| attribute.key.as_ref() == key.as_bytes())
        {
            updated.push_attribute(attribute);
        }
    }
    for &(key, value) in replacements {
        updated.push_attribute((key, value));
    }
    let mut writer = Writer::new(Vec::new());
    writer.write_event(Event::Start(updated)).map_err(invalid)?;
    String::from_utf8(writer.into_inner()).map_err(invalid)
}

/// Limits individual XML events without capping the total size of a large TM.
struct EventInput<R> {
    inner: R,
    remaining: usize,
}
impl<R: BufRead> std::io::Read for EventInput<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(bytes.len());
        bytes[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}
impl<R: BufRead> BufRead for EventInput<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "evento XML supera 8 MiB",
            ));
        }
        let available = self.inner.fill_buf()?;
        Ok(&available[..available.len().min(self.remaining)])
    }
    fn consume(&mut self, count: usize) {
        self.remaining = self.remaining.saturating_sub(count);
        self.inner.consume(count);
    }
}

pub fn export_document(
    document: &ImportedDocument,
    targets: &[String],
    destination: &Path,
    cancel: &Cancellation,
) -> Result<()> {
    if destination.exists() {
        return Err(CatError::Invalid(
            "destino ya existe; elija un archivo nuevo".into(),
        ));
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent)?;
    let filename = destination
        .file_name()
        .ok_or_else(|| CatError::Invalid("destino sin nombre".into()))?;
    let resolved = parent.join(filename);
    if document
        .original_path
        .as_ref()
        .is_some_and(|p| p == &resolved)
    {
        return Err(CatError::Invalid(
            "no se puede sobrescribir el original".into(),
        ));
    }
    let bytes = serialize_document(document, targets, cancel)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    cancel.check()?;
    temporary
        .persist_noclobber(&resolved)
        .map_err(|e| CatError::Io(e.error))?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn import_tmx<R: BufRead>(
    reader: R,
    source_lang: &str,
    target_lang: &str,
    cancel: &Cancellation,
    mut sink: impl FnMut(TmUnit) -> Result<()>,
) -> Result<usize> {
    let mut reader = Reader::from_reader(EventInput {
        inner: reader,
        remaining: MAX_TU,
    });
    let mut buffer = Vec::new();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut writer: Option<Writer<Vec<u8>>> = None;
    let mut namespaces: Vec<Vec<(Vec<u8>, Vec<u8>)>> = Vec::new();
    let mut count = 0;
    let mut root_seen = false;
    let mut root_closed = false;
    loop {
        cancel.check()?;
        buffer.clear();
        reader.get_mut().remaining = MAX_TU;
        let event = reader.read_event_into(&mut buffer).map_err(invalid)?;
        check_decl(&event)?;
        let _ = text_event(&event)?;
        match &event {
            Event::Start(s) | Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                let _ = attr(s, b"__none")?;
                let declarations = s
                    .attributes()
                    .filter_map(|a| match a {
                        Ok(a)
                            if a.key.as_ref() == b"xmlns"
                                || a.key.as_ref().starts_with(b"xmlns:") =>
                        {
                            Some(Ok((a.key.as_ref().to_vec(), a.value.into_owned())))
                        }
                        Err(e) => Some(Err(invalid(e))),
                        _ => None,
                    })
                    .collect::<Result<Vec<_>>>()?;
                if stack.is_empty() {
                    if root_seen
                        || root_closed
                        || name != b"tmx"
                        || !matches!(attr(s, b"version")?.as_deref(), Some("1.4" | "1.4b"))
                    {
                        return Err(invalid("se requiere TMX 1.4"));
                    }
                    root_seen = true;
                }
                if stack.len() >= 128 {
                    return Err(invalid("TMX demasiado profundo"));
                }
                if name == b"tu" {
                    if writer.is_some() || stack.last().map(Vec::as_slice) != Some(b"body") {
                        return Err(invalid("tu fuera de body o anidado"));
                    }
                    writer = Some(Writer::new(Vec::new()));
                }
                if let Some(w) = writer.as_mut() {
                    if name == b"tu" {
                        let mut start = s.clone().into_owned();
                        let mut inherited = std::collections::BTreeMap::new();
                        for scope in &namespaces {
                            for (key, value) in scope {
                                inherited.insert(key.clone(), value.clone());
                            }
                        }
                        for (key, _) in &declarations {
                            inherited.remove(key);
                        }
                        for (key, value) in inherited {
                            start.push_attribute((key.as_slice(), value.as_slice()));
                        }
                        w.write_event(Event::Start(start)).map_err(invalid)?;
                    } else {
                        w.write_event(event.clone()).map_err(invalid)?;
                    }
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(name);
                    namespaces.push(declarations);
                } else if s.name().as_ref() == b"tu" {
                    return Err(invalid("TU vacío"));
                }
            }
            Event::End(e) => {
                if stack.pop().as_deref() != Some(e.name().as_ref()) {
                    return Err(invalid("TMX sin balance"));
                }
                namespaces.pop();
                if let Some(w) = writer.as_mut() {
                    w.write_event(event.clone()).map_err(invalid)?;
                }
                if e.name().as_ref() == b"tu"
                    && let Some(w) = writer.take()
                {
                    let raw = String::from_utf8(w.into_inner()).map_err(invalid)?;
                    if let Some(unit) = parse_tu(&raw, source_lang, target_lang, cancel)? {
                        sink(unit)?;
                        count += 1;
                    }
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Eof => break,
            _ => {
                if let Some(w) = writer.as_mut() {
                    w.write_event(event.clone()).map_err(invalid)?;
                } else if stack.is_empty()
                    && text_event(&event)?.is_some_and(|s| !s.trim().is_empty())
                {
                    return Err(invalid("texto fuera de TMX"));
                }
            }
        }
        if writer.as_ref().is_some_and(|w| w.get_ref().len() > MAX_TU) {
            return Err(invalid("TU supera 8 MiB"));
        }
    }
    if !root_seen || !root_closed || !stack.is_empty() {
        return Err(invalid("TMX incompleto"));
    }
    Ok(count)
}

fn parse_tu(
    raw: &str,
    source_lang: &str,
    target_lang: &str,
    cancel: &Cancellation,
) -> Result<Option<TmUnit>> {
    if raw.chars().any(|c| !xml_char(c)) {
        return Err(invalid("carácter XML inválido"));
    }
    let mut reader = Reader::from_str(raw);
    let mut stack = Vec::new();
    let mut root_seen = false;
    let mut language = None;
    let mut languages = std::collections::HashSet::new();
    let mut segment = String::new();
    let mut seg_seen = false;
    let mut source = None;
    let mut target = None;
    loop {
        cancel.check()?;
        let event = reader.read_event().map_err(invalid)?;
        check_decl(&event)?;
        match &event {
            Event::Start(s) | Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                let _ = attr(s, b"__none")?;
                if stack.is_empty() {
                    if root_seen || name != b"tu" || matches!(event, Event::Empty(_)) {
                        return Err(invalid("se requiere un único TU"));
                    }
                    root_seen = true;
                }
                if stack.len() >= 128 {
                    return Err(invalid("TU demasiado profundo"));
                }
                if stack.last().is_some_and(|n: &Vec<u8>| n == b"seg") {
                    return Err(invalid("TMX inline codes no soportados todavía"));
                }
                if name == b"tuv" {
                    let lang = attr(s, b"xml:lang")?
                        .or(attr(s, b"lang")?)
                        .ok_or_else(|| invalid("tuv sin idioma"))?;
                    if !languages.insert(lang.to_ascii_lowercase()) {
                        return Err(invalid("idioma duplicado ambiguo en TU"));
                    }
                    language = Some(lang);
                    segment.clear();
                    seg_seen = false;
                }
                if name == b"seg" {
                    if stack.last().map(Vec::as_slice) != Some(b"tuv") || seg_seen {
                        return Err(invalid("seg fuera de tuv o duplicado"));
                    }
                    seg_seen = true;
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(name);
                }
            }
            Event::End(e) => {
                if stack.pop().as_deref() != Some(e.name().as_ref()) {
                    return Err(invalid("TU sin balance"));
                }
                if e.name().as_ref() == b"tuv" {
                    if !seg_seen {
                        return Err(invalid("tuv sin seg"));
                    }
                    if let Some(lang) = language.take() {
                        if lang.eq_ignore_ascii_case(source_lang) {
                            source = Some(segment.clone());
                        }
                        if lang.eq_ignore_ascii_case(target_lang) {
                            target = Some(segment.clone());
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {
                if let Some(text) = text_event(&event)? {
                    if text.chars().any(|c| !xml_char(c)) {
                        return Err(invalid("carácter XML inválido"));
                    }
                    if stack.last().is_some_and(|n| n == b"seg") {
                        segment.push_str(&text);
                        if segment.len() > MAX_TEXT {
                            return Err(invalid("segmento TMX supera 4 MiB"));
                        }
                    } else if stack.is_empty() && !text.trim().is_empty() {
                        return Err(invalid("texto fuera de TU"));
                    }
                }
            }
        }
    }
    if !root_seen || !stack.is_empty() {
        return Err(invalid("TU incompleto"));
    }
    Ok(source.zip(target).map(|(source, target)| TmUnit {
        source,
        target,
        source_lang: source_lang.into(),
        target_lang: target_lang.into(),
        raw_xml: raw.into(),
    }))
}

pub fn export_tmx<W: Write>(
    mut writer: W,
    units: impl IntoIterator<Item = Result<TmUnit>>,
    cancel: &Cancellation,
) -> Result<usize> {
    writer.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><tmx version=\"1.4\"><header creationtool=\"LumenCAT\" creationtoolversion=\"0.1\" segtype=\"sentence\" o-tmf=\"LumenCAT\" adminlang=\"en\" srclang=\"*all*\" datatype=\"PlainText\"/><body>")?;
    let mut count = 0;
    for unit in units {
        cancel.check()?;
        let unit = unit?;
        let raw = if unit.raw_xml.is_empty() {
            format!(
                "<tu><tuv xml:lang=\"{}\"><seg>{}</seg></tuv><tuv xml:lang=\"{}\"><seg>{}</seg></tuv></tu>",
                escaped(&unit.source_lang)?,
                escaped(&unit.source)?,
                escaped(&unit.target_lang)?,
                escaped(&unit.target)?
            )
        } else {
            unit.raw_xml.clone()
        };
        let parsed = parse_tu(&raw, &unit.source_lang, &unit.target_lang, cancel)?
            .ok_or_else(|| invalid("TU sin par de idiomas"))?;
        if parsed.source != unit.source || parsed.target != unit.target {
            return Err(invalid(
                "metadata raw TU no coincide con traducción; export rechazado",
            ));
        }
        writer.write_all(raw.as_bytes())?;
        count += 1;
    }
    cancel.check()?;
    writer.write_all(b"</body></tmx>")?;
    writer.flush()?;
    Ok(count)
}
