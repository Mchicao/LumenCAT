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

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
    Xliff,
    Tmx,
}

impl Fragment {
    pub fn parse(raw: &str, reference: Option<&Self>, cancel: &Cancellation) -> Result<Self> {
        Self::parse_with(raw, reference, Dialect::Xliff, cancel)
    }

    pub fn parse_tmx(raw: &str, reference: Option<&Self>, cancel: &Cancellation) -> Result<Self> {
        Self::parse_with(raw, reference, Dialect::Tmx, cancel)
    }

    fn parse_with(
        raw: &str,
        reference: Option<&Self>,
        dialect: Dialect,
        cancel: &Cancellation,
    ) -> Result<Self> {
        let mut reader = Reader::from_str(raw);
        let mut fragment = Self::default();
        let mut groups = Vec::new();
        let mut occurrences = HashMap::new();
        let reference_ids: HashMap<_, _> = reference
            .into_iter()
            .flat_map(|fragment| &fragment.codes)
            .map(|code| (code.key.as_str(), code.id))
            .collect();
        loop {
            cancel.check()?;
            let before = reader.buffer_position() as usize;
            let event = reader.read_event().map_err(invalid)?;
            check_decl(&event)?;
            match &event {
                Event::Start(start) | Event::Empty(start) => {
                    let name = start.name();
                    let name = local(name.as_ref());
                    let group = if dialect == Dialect::Tmx {
                        name == b"hi"
                    } else {
                        matches!(name, b"g" | b"mrk")
                    };
                    let supported = if dialect == Dialect::Tmx {
                        matches!(name, b"ph" | b"bpt" | b"ept" | b"it" | b"ut")
                    } else {
                        matches!(name, b"x" | b"ph" | b"bpt" | b"ept" | b"it" | b"bx" | b"ex")
                    };
                    if !group && !supported {
                        return Err(invalid(
                            "elemento inline no soportado; sub y extensiones requieren un adaptador específico",
                        ));
                    }
                    let pair_open = matches!(name, b"bpt" | b"bx");
                    let pair_close = matches!(name, b"ept" | b"ex");
                    let pair = if pair_open || pair_close {
                        let identity = if dialect == Dialect::Tmx {
                            attr(start, b"i")?
                        } else {
                            attr(start, b"rid")?.or(attr(start, b"id")?)
                        };
                        Some(format!(
                            "{}:{}",
                            if matches!(name, b"bpt" | b"ept") {
                                "bpt"
                            } else {
                                "bx"
                            },
                            identity
                                .filter(|id| !id.is_empty())
                                .ok_or_else(|| invalid("par inline sin identificador"))?
                        ))
                    } else {
                        None
                    };
                    if pair_close && dialect == Dialect::Tmx {
                        if matches!(event, Event::Start(_)) {
                            read_native_end(&mut reader, start, cancel)?;
                        }
                        let (expected, index, native): (Vec<u8>, usize, bool) = groups
                            .pop()
                            .ok_or_else(|| invalid("cierre nativo sin apertura"))?;
                        if !native
                            || Some(expected.as_slice()) != pair.as_deref().map(str::as_bytes)
                        {
                            return Err(invalid("orden o identificador de par inline incorrecto"));
                        }
                        fragment.codes[index].closing =
                            Some(raw[before..reader.buffer_position() as usize].into());
                        fragment.view.push_str("</g>");
                        continue;
                    }
                    let identity = if dialect == Dialect::Tmx {
                        attr(start, b"x")?
                            .or(if pair_open { attr(start, b"i")? } else { None })
                            .unwrap_or_default()
                    } else if name == b"mrk" {
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
                    if *occurrence > 0 && name != b"mrk" && dialect != Dialect::Tmx {
                        return Err(invalid("id de código inline XLIFF duplicado"));
                    }
                    *occurrence += 1;
                    let key = format!("{key}:{}", *occurrence);
                    let id = if reference.is_some() {
                        *reference_ids.get(key.as_str()).ok_or_else(|| {
                            invalid("el destino contiene códigos inline ausentes del origen")
                        })?
                    } else {
                        fragment.codes.len() + 1
                    };
                    let container = group && matches!(event, Event::Start(_));
                    let pair_group = pair_open && dialect == Dialect::Tmx;
                    if !container && matches!(event, Event::Start(_)) {
                        read_native_end(&mut reader, start, cancel)?;
                    }
                    let opening_end = reader.buffer_position() as usize;
                    let closing = if container || pair_group {
                        groups.push((
                            pair.as_ref()
                                .map(|value| value.as_bytes().to_vec())
                                .unwrap_or_else(|| start.name().as_ref().to_vec()),
                            fragment.codes.len(),
                            pair_group,
                        ));
                        fragment.view.push_str(&format!("<g id=\"{id}\">"));
                        Some(String::new())
                    } else {
                        fragment.view.push_str(&format!("<x id=\"{id}\"/>"));
                        None
                    };
                    fragment.codes.push(Code {
                        id,
                        key,
                        opening: raw[before..opening_end].into(),
                        closing,
                        pair: if dialect == Dialect::Xliff {
                            pair.map(|key| (pair_open, key))
                        } else {
                            None
                        },
                    });
                }
                Event::End(end) => {
                    let (name, index, native) = groups
                        .pop()
                        .ok_or_else(|| invalid("cierre inline sin apertura"))?;
                    if native || name != end.name().as_ref() {
                        return Err(invalid("códigos inline sin balance"));
                    }
                    fragment.codes[index].closing =
                        Some(raw[before..reader.buffer_position() as usize].into());
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
        let codes: HashMap<_, _> = self
            .codes
            .iter()
            .chain(&existing.codes)
            .map(|code| (code.id, code))
            .collect();
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
            let code = *codes
                .get(&id)
                .ok_or_else(|| invalid("código inline desconocido"))?;
            output.push_str(&code.opening);
            if code.closing.is_some() {
                groups.push(code);
            }
            if let Some((opening, pair)) = &code.pair {
                if *opening {
                    pairs.push(pair);
                } else if pairs.pop() != Some(pair) {
                    return Err(invalid("orden o anidación de códigos nativos incorrectos"));
                }
            }
        }
        if !pairs.is_empty() {
            return Err(invalid("faltan cierres de códigos nativos"));
        }
        Ok(output)
    }
}

fn read_native_end(
    reader: &mut Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    cancel: &Cancellation,
) -> Result<()> {
    loop {
        cancel.check()?;
        let event = reader.read_event().map_err(invalid)?;
        check_decl(&event)?;
        match &event {
            Event::End(end) if end.name() == start.name() => return Ok(()),
            Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) => {
                text_event(&event)?;
            }
            Event::Comment(_) => (),
            _ => {
                return Err(invalid(
                    "contenido XML anidado dentro de un código nativo no soportado",
                ));
            }
        }
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
