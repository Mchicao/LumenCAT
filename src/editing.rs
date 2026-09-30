use quick_xml::{Reader, events::Event};

pub fn parts(text: &str) -> Vec<(&str, bool)> {
    let mut result = Vec::new();
    let mut offset = 0;
    let mut plain = 0;
    while let Some(start) = text[offset..].find('<').map(|i| i + offset) {
        let mut reader = Reader::from_str(&text[start..]);
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        let tag = match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => matches!(e.name().as_ref(), b"g" | b"x"),
            Ok(Event::End(e)) => e.name().as_ref() == b"g",
            _ => false,
        };
        let end = start + reader.buffer_position() as usize;
        if tag && end > start {
            if plain < start {
                result.push((&text[plain..start], false));
            }
            result.push((&text[start..end], true));
            plain = end;
        }
        offset = if end > start { end } else { start + 1 };
    }
    if plain < text.len() {
        result.push((&text[plain..], false));
    }
    result
}

pub fn next_tag<'a>(source: &'a str, target: &str) -> Option<&'a str> {
    let source_tags: Vec<_> = parts(source)
        .into_iter()
        .filter(|p| p.1)
        .map(|p| p.0)
        .collect();
    let target_tags: Vec<_> = parts(target)
        .into_iter()
        .filter(|p| p.1)
        .map(|p| p.0)
        .collect();
    source_tags
        .iter()
        .enumerate()
        .find(|(i, tag)| {
            let needed = source_tags[..=*i].iter().filter(|t| *t == *tag).count();
            target_tags.iter().filter(|t| *t == *tag).count() < needed
        })
        .map(|(_, tag)| *tag)
}

pub fn replace_text(text: &str, query: &str, replacement: &str) -> String {
    if query.is_empty() {
        return text.into();
    }
    parts(text)
        .into_iter()
        .map(|(part, tag)| {
            if tag {
                part.into()
            } else {
                part.replace(query, replacement)
            }
        })
        .collect()
}

pub fn words(text: &str) -> usize {
    parts(text)
        .into_iter()
        .filter(|p| !p.1)
        .map(|p| p.0.split_whitespace().count())
        .sum()
}

pub fn utf16_offset(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        if units + ch.len_utf16() > offset {
            return index;
        }
        units += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_tags_and_replacement() {
        assert_eq!(utf16_offset("a😀é", 2), 1);
        assert_eq!(utf16_offset("a😀é", 3), 5);
        let text = "你好 <g id=\"1\">café<x id=\"2\"/></g>";
        assert_eq!(next_tag(text, "<g id=\"1\">"), Some("<x id=\"2\"/>"));
        assert_eq!(replace_text(text, "1", "9"), text);
        assert_eq!(next_tag(text, "<g id=\"1\"><x id=\"2\"/>"), Some("</g>"));
        assert_eq!(parts(text).iter().filter(|p| p.1).count(), 3);
        assert!(replace_text(text, "café", "té").contains("té<x"));
        assert_eq!(
            next_tag("<x id=\"2\"/><x id=\"2\"/>", "<x id=\"2\"/>"),
            Some("<x id=\"2\"/>")
        );
        assert_eq!(parts("a < b café"), vec![("a < b café", false)]);
    }
}
