use super::{
    attr, check_decl, escaped, inline::Fragment, invalid, local, text_event, updated_opening,
};
use crate::model::{Cancellation, ImportedSegment, Result};
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};
use std::collections::{HashMap, HashSet};

pub(super) struct Unit {
    template: String,
    pub(super) from_source: bool,
    markers: Vec<(usize, Marker)>,
    fragments: Vec<(String, Fragment, Fragment)>,
}

struct Marker {
    key: String,
    name: String,
    span: (usize, usize),
    content: (usize, usize),
}

impl Unit {
    pub fn parse(
        source: &str,
        segmented: &str,
        target: &str,
        cancel: &Cancellation,
    ) -> Result<Self> {
        let (sources, unmarked) = markers(segmented, cancel)?;
        if sources.is_empty() {
            return Err(invalid("seg-source sin marcadores de segmento"));
        }
        if canonical(source, cancel)? != canonical(&unmarked, cancel)? {
            return Err(invalid("seg-source no conserva el contenido de source"));
        }
        Fragment::parse(&unmarked, None, cancel)?;
        let from_source = target
            .chars()
            .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'));
        let (targets, target_unmarked) = markers(target, cancel)?;
        if !from_source {
            Fragment::parse(&target_unmarked, None, cancel)?;
            let source_ids: HashSet<_> = sources.iter().map(|m| &m.key).collect();
            let target_ids: HashSet<_> = targets.iter().map(|m| &m.key).collect();
            if source_ids != target_ids {
                return Err(invalid(
                    "target segmentado debe identificar los mismos segmentos que seg-source; no se adivinan límites",
                ));
            }
        }
        let target_by_key: HashMap<_, _> = targets.iter().map(|m| (m.key.as_str(), m)).collect();
        let mut fragments = Vec::new();
        for marker in &sources {
            cancel.check()?;
            let source =
                Fragment::parse(&segmented[marker.content.0..marker.content.1], None, cancel)?;
            let target = if let Some(marker) = target_by_key.get(marker.key.as_str()) {
                Fragment::parse(
                    &target[marker.content.0..marker.content.1],
                    Some(&source),
                    cancel,
                )?
            } else {
                Fragment::default()
            };
            fragments.push((marker.key.clone(), source, target));
        }
        let source_by_key: HashMap<_, _> = sources
            .iter()
            .enumerate()
            .map(|(i, m)| (m.key.as_str(), i))
            .collect();
        let target_indices = targets
            .iter()
            .map(|m| source_by_key[m.key.as_str()])
            .collect::<Vec<_>>();
        let markers = if from_source {
            sources.into_iter().enumerate().collect()
        } else {
            target_indices.into_iter().zip(targets).collect()
        };
        Ok(Self {
            template: if from_source { segmented } else { target }.into(),
            from_source,
            markers,
            fragments,
        })
    }

    pub fn segments(&self, base: &ImportedSegment) -> Vec<ImportedSegment> {
        self.fragments
            .iter()
            .map(|(key, source, target)| ImportedSegment {
                external_id: format!("seg:{}:{}:{key}", base.external_id.len(), base.external_id),
                source: source.view.clone(),
                target: target.view.clone(),
                state: base.state,
                locked: base.locked,
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn changed(&self, targets: &[String]) -> bool {
        self.fragments
            .iter()
            .zip(targets)
            .any(|((_, _, existing), target)| &existing.view != target)
    }

    pub fn serialize(&self, targets: &[String], cancel: &Cancellation) -> Result<String> {
        if targets.len() != self.len() {
            return Err(invalid("cantidad de segmentos XLIFF no coincide"));
        }
        let mut replacements = Vec::new();
        for (index, marker) in &self.markers {
            cancel.check()?;
            let (_, source, existing) = &self.fragments[*index];
            let target = &targets[*index];
            if !self.from_source && target == &existing.view {
                continue;
            }
            let content = source.render(target, existing)?;
            let opening = updated_opening(&self.template[marker.span.0..marker.content.0], &[])?;
            replacements.push((
                marker.span.0,
                marker.span.1,
                format!("{opening}{content}</{}>", marker.name),
            ));
        }
        let output = rewrite(&self.template, replacements)?;
        let (_, unmarked) = markers(&output, cancel)?;
        Fragment::parse(&unmarked, None, cancel)?;
        Ok(output)
    }
}

fn markers(raw: &str, cancel: &Cancellation) -> Result<(Vec<Marker>, String)> {
    let mut reader = Reader::from_str(raw);
    let mut stack = Vec::new();
    let mut found = Vec::new();
    let mut active: Option<(usize, usize)> = None;
    let mut removals = Vec::new();
    let mut keys = HashSet::new();
    let mut has_mid = None;
    loop {
        cancel.check()?;
        let before = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(invalid)?;
        check_decl(&event)?;
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                let _ = attr(start, b"__none")?;
                if local(start.name().as_ref()) == b"mrk"
                    && attr(start, b"mtype")?.as_deref() == Some("seg")
                {
                    if active.is_some() {
                        return Err(invalid("marcadores de segmento anidados"));
                    }
                    let mid = attr(start, b"mid")?;
                    if mid.as_deref() == Some("")
                        || has_mid.is_some_and(|value| value != mid.is_some())
                    {
                        return Err(invalid("mid vacío o identificación parcial de segmentos"));
                    }
                    has_mid = Some(mid.is_some());
                    let key = mid.map_or_else(
                        || format!("ordinal:{}", found.len()),
                        |id| format!("mid:{id}"),
                    );
                    if !keys.insert(key.clone()) {
                        return Err(invalid("mid de segmento duplicado"));
                    }
                    let after = reader.buffer_position() as usize;
                    found.push(Marker {
                        key,
                        name: std::str::from_utf8(start.name().as_ref())
                            .map_err(invalid)?
                            .into(),
                        span: (before, after),
                        content: (after, after),
                    });
                    removals.push((before, after, String::new()));
                    if matches!(event, Event::Start(_)) {
                        active = Some((found.len() - 1, stack.len() + 1));
                    }
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(start.name().as_ref().to_vec());
                }
                if stack.len() > 128 {
                    return Err(invalid("XML demasiado profundo"));
                }
            }
            Event::End(end) => {
                if stack.pop().as_deref() != Some(end.name().as_ref()) {
                    return Err(invalid("XML segmentado sin balance"));
                }
                if let Some((index, depth)) = active
                    && stack.len() + 1 == depth
                {
                    found[index].span.1 = reader.buffer_position() as usize;
                    found[index].content.1 = before;
                    removals.push((before, reader.buffer_position() as usize, String::new()));
                    active = None;
                }
            }
            Event::Eof => break,
            _ => {
                text_event(&event)?;
            }
        }
    }
    if !stack.is_empty() || active.is_some() {
        return Err(invalid("XML segmentado incompleto"));
    }
    Ok((found, rewrite(raw, removals)?))
}

fn rewrite(raw: &str, mut replacements: Vec<(usize, usize, String)>) -> Result<String> {
    replacements.sort_by_key(|(start, _, _)| *start);
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end, replacement) in replacements {
        if start < cursor || end < start || end > raw.len() {
            return Err(invalid("rangos XLIFF solapados"));
        }
        output.push_str(&raw[cursor..start]);
        output.push_str(&replacement);
        cursor = end;
    }
    output.push_str(&raw[cursor..]);
    Ok(output)
}

fn canonical(raw: &str, cancel: &Cancellation) -> Result<String> {
    let mut reader = Reader::from_str(raw);
    let mut writer = Writer::new(Vec::new());
    loop {
        cancel.check()?;
        let event = reader.read_event().map_err(invalid)?;
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                let mut attributes = start
                    .attributes()
                    .map(|a| {
                        let a = a.map_err(invalid)?;
                        Ok((
                            std::str::from_utf8(a.key.as_ref())
                                .map_err(invalid)?
                                .to_owned(),
                            a.unescape_value().map_err(invalid)?.into_owned(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                attributes.sort();
                let name = std::str::from_utf8(start.name().as_ref())
                    .map_err(invalid)?
                    .to_owned();
                let mut normalized = BytesStart::new(name.clone());
                for (key, value) in &attributes {
                    normalized.push_attribute((key.as_str(), value.as_str()));
                }
                writer
                    .write_event(Event::Start(normalized))
                    .map_err(invalid)?;
                if matches!(event, Event::Empty(_)) {
                    writer
                        .write_event(Event::End(quick_xml::events::BytesEnd::new(name)))
                        .map_err(invalid)?;
                }
            }
            Event::End(_) => {
                writer.write_event(event).map_err(invalid)?;
            }
            Event::Eof => break,
            _ => {
                if let Some(text) = text_event(&event)? {
                    std::io::Write::write_all(writer.get_mut(), escaped(&text)?.as_bytes())?;
                }
            }
        }
    }
    String::from_utf8(writer.into_inner()).map_err(invalid)
}
