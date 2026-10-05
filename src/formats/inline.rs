use super::{attr, check_decl, escaped, invalid, local, text_event};
use crate::model::{Cancellation, Result};
use quick_xml::{Reader, events::Event};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Fragment {
    pub view: String,
    codes: Vec<Code>,
}

struct Code {
    id: usize,
    key: String,
    opening: String,
    closing: Option<String>,
    pair: Option<(bool, String)>,
}

impl Fragment {
    pub fn parse(raw: &str, reference: Option<&Self>, cancel: &Cancellation) -> Result<Self> {
        let mut reader = Reader::from_str(raw);
        let mut fragment = Self::default();
        let mut groups = Vec::new();
        let mut occurrences = HashMap::new();
        loop {
            cancel.check()?;
            let before = reader.buffer_position() as usize;
            let event = reader.read_event().map_err(invalid)?;
            check_decl(&event)?;
            match &event {
                Event::Start(start) | Event::Empty(start) => {
                    let name = start.name();
                    let name = local(name.as_ref());
                    let group = matches!(name, b"g" | b"mrk");
                    if !group
                        && !matches!(name, b"x" | b"ph" | b"bpt" | b"ept" | b"it" | b"bx" | b"ex")
                    {
                        return Err(invalid(
                            "elemento inline XLIFF no soportado; sub y extensiones requieren un adaptador específico",
                        ));
                    }
                    let identity = if name == b"mrk" {
                        format!(
                            "{}:{}",
                            attr(start, b"mtype")?.unwrap_or_default(),
                            attr(start, b"mid")?.unwrap_or_default()
                        )
                    } else {
                        attr(start, b"id")?
                            .filter(|id| !id.is_empty())
                            .ok_or_else(|| invalid("código inline XLIFF sin id"))?
                    };
                    let key = format!("{}:{identity}", String::from_utf8_lossy(name));
                    let occurrence = occurrences.entry(key.clone()).or_insert(0);
                    if *occurrence > 0 && name != b"mrk" {
                        return Err(invalid("id de código inline XLIFF duplicado"));
                    }
                    *occurrence += 1;
                    let key = format!("{key}:{}", *occurrence);
                    let id = if let Some(reference) = reference {
                        reference
                            .codes
                            .iter()
                            .find(|code| code.key == key)
                            .ok_or_else(|| {
                                invalid("el destino contiene códigos inline ausentes del origen")
                            })?
                            .id
                    } else {
                        fragment.codes.len() + 1
                    };
                    let opening_end = reader.buffer_position() as usize;
                    let closing = if group && matches!(event, Event::Start(_)) {
                        groups.push((start.name().as_ref().to_vec(), id));
                        fragment.view.push_str(&format!("<g id=\"{id}\">"));
                        Some(String::new())
                    } else {
                        if matches!(event, Event::Start(_)) {
                            loop {
                                cancel.check()?;
                                let inner = reader.read_event().map_err(invalid)?;
                                check_decl(&inner)?;
                                match &inner {
                                    Event::End(end) if end.name() == start.name() => break,
                                    Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) => {
                                        text_event(&inner)?;
                                    }
                                    Event::Comment(_) => (),
                                    _ => {
                                        return Err(invalid(
                                            "contenido XML anidado dentro de un código nativo XLIFF no soportado",
                                        ));
                                    }
                                }
                            }
                        }
                        fragment.view.push_str(&format!("<x id=\"{id}\"/>"));
                        None
                    };
                    let pair = match name {
                        b"bpt" | b"ept" | b"bx" | b"ex" => Some((
                            matches!(name, b"bpt" | b"bx"),
                            attr(start, b"rid")?.unwrap_or(identity),
                        )),
                        _ => None,
                    };
                    fragment.codes.push(Code {
                        id,
                        key,
                        opening: raw[before..if closing.is_some() {
                            opening_end
                        } else {
                            reader.buffer_position() as usize
                        }]
                            .into(),
                        closing,
                        pair,
                    });
                }
                Event::End(end) => {
                    let (name, id) = groups
                        .pop()
                        .ok_or_else(|| invalid("cierre inline sin apertura"))?;
                    if name != end.name().as_ref() {
                        return Err(invalid("códigos inline sin balance"));
                    }
                    let code = fragment
                        .codes
                        .iter_mut()
                        .find(|code| code.id == id)
                        .ok_or_else(|| invalid("grupo inline perdido"))?;
                    code.closing = Some(raw[before..reader.buffer_position() as usize].into());
                    fragment.view.push_str("</g>");
                }
                Event::Eof => break,
                _ => {
                    if let Some(text) = text_event(&event)? {
                        if crate::editing::parts(&text).iter().any(|(_, tag)| *tag) {
                            return Err(invalid(
                                "texto literal ambiguo con los códigos protegidos <g>/<x>",
                            ));
                        }
                        fragment.view.push_str(&text);
                    } else {
                        return Err(invalid("evento XML inline no soportado"));
                    }
                }
            }
            if fragment.view.len() > super::MAX_TEXT || groups.len() > 128 {
                return Err(invalid(
                    "fragmento inline supera el límite de tamaño o profundidad",
                ));
            }
        }
        if !groups.is_empty() {
            return Err(invalid("grupo inline incompleto"));
        }
        if reference.is_none() {
            fragment.render(&fragment.view, &fragment)?;
        }
        Ok(fragment)
    }

    pub fn render(&self, target: &str, existing: &Self) -> Result<String> {
        validate_target(&self.view, target)?;
        let mut output = String::new();
        let mut groups = Vec::new();
        let mut pairs = Vec::new();
        for (piece, tag) in crate::editing::parts(target) {
            if !tag {
                output.push_str(&escaped(piece)?);
                continue;
            }
            if piece == "</g>" {
                let code: &Code = groups
                    .pop()
                    .ok_or_else(|| invalid("cierre inline sin apertura"))?;
                output.push_str(
                    code.closing
                        .as_deref()
                        .ok_or_else(|| invalid("cierre inline perdido"))?,
                );
                continue;
            }
            let id = code_id(piece)?;
            let code = existing
                .codes
                .iter()
                .chain(&self.codes)
                .find(|code| code.id == id)
                .ok_or_else(|| invalid("código inline desconocido"))?;
            output.push_str(&code.opening);
            if code.closing.is_some() {
                groups.push(code);
            }
            if let Some((opening, pair)) = &code.pair {
                if *opening {
                    pairs.push(pair);
                } else if pairs.pop() != Some(pair) {
                    return Err(invalid(
                        "orden o anidación de códigos nativos XLIFF incorrectos",
                    ));
                }
            }
        }
        if !pairs.is_empty() {
            return Err(invalid("faltan cierres de códigos nativos XLIFF"));
        }
        Ok(output)
    }
}

fn code_id(piece: &str) -> Result<usize> {
    piece
        .strip_prefix("<g id=\"")
        .and_then(|value| value.strip_suffix("\">"))
        .or_else(|| {
            piece
                .strip_prefix("<x id=\"")
                .and_then(|value| value.strip_suffix("\"/>"))
        })
        .ok_or_else(|| invalid("código protegido modificado"))?
        .parse()
        .map_err(|_| invalid("id de código protegido inválido"))
}

pub(crate) fn validate_target(source: &str, target: &str) -> Result<()> {
    let expected: HashSet<_> = crate::editing::parts(source)
        .into_iter()
        .filter(|(piece, tag)| *tag && *piece != "</g>")
        .map(|(piece, _)| piece)
        .collect();
    let mut seen = HashSet::new();
    let mut groups = Vec::new();
    for (piece, tag) in crate::editing::parts(target) {
        if !tag {
            escaped(piece)?;
        } else if piece == "</g>" {
            if groups.pop().is_none() {
                return Err(invalid("cierre inline sin apertura"));
            }
        } else {
            code_id(piece)?;
            if !expected.contains(piece) || !seen.insert(piece) {
                return Err(invalid(
                    "código protegido desconocido, modificado o duplicado",
                ));
            }
            if piece.starts_with("<g ") {
                groups.push(piece);
            }
        }
    }
    if !groups.is_empty() || seen != expected {
        return Err(invalid("faltan códigos protegidos o sus cierres"));
    }
    Ok(())
}
