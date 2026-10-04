use crate::{editing, model::*};
use std::{collections::HashMap, ops::Range};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

struct Token {
    text: String,
    folded: String,
    range: Range<usize>,
}

fn tokens(text: &str, coded: bool) -> Vec<Token> {
    let parts = if coded {
        editing::parts(text)
    } else {
        vec![(text, false)]
    };
    let mut visible = String::new();
    let mut spans = Vec::new();
    let mut raw = 0;
    for (part, code) in parts {
        if !code {
            let start = visible.len();
            visible.push_str(part);
            if !part.is_empty() {
                spans.push((start..visible.len(), raw));
            }
        } else if part.starts_with("<x") {
            visible.push(' ');
        }
        raw += part.len();
    }
    let mut result = Vec::new();
    let mut start = None;
    for (index, ch) in visible
        .char_indices()
        .chain(std::iter::once((visible.len(), ' ')))
    {
        if ch.is_alphanumeric() || is_combining_mark(ch) {
            start.get_or_insert(index);
        } else if let Some(first) = start.take() {
            let first_span = &spans[spans.partition_point(|(range, _)| range.end <= first)];
            let last_span = &spans[spans.partition_point(|(range, _)| range.end < index)];
            let word: String = visible[first..index].nfc().collect();
            result.push(Token {
                folded: word.to_lowercase().nfc().collect(),
                text: word,
                range: first_span.1 + first - first_span.0.start
                    ..last_span.1 + index - last_span.0.start,
            });
        }
    }
    result
}

fn occurrences(text: &[Token], expression: &TermExpression) -> Vec<Range<usize>> {
    let phrase = tokens(&expression.text, false);
    if phrase.is_empty() || phrase.len() > text.len() {
        return Vec::new();
    }
    text.windows(phrase.len())
        .filter(|window| {
            window.iter().zip(&phrase).all(|(a, b)| {
                if expression.case_sensitive {
                    a.text == b.text
                } else {
                    a.folded == b.folded
                }
            })
        })
        .map(|window| window[0].range.start..window[window.len() - 1].range.end)
        .take(513)
        .collect()
}

pub fn check(
    concepts: &[TermConcept],
    source: &str,
    target: &str,
    sl: &str,
    tl: &str,
    format: DocumentFormat,
    cancel: &Cancellation,
) -> Result<TerminologyResult> {
    cancel.check()?;
    if source.len() > 1_048_576 || target.len() > 1_048_576 {
        return Err(CatError::Invalid(
            "Consulta terminológica supera 1 MiB por texto".into(),
        ));
    }
    let source = tokens(source, format == DocumentFormat::Docx);
    let target = tokens(target, format == DocumentFormat::Docx);
    let mut result = TerminologyResult::default();
    // ponytail: escaneo por expresión, acotado a 10.000 por proyecto; indexar frases antes de ampliar ese límite.
    for concept in concepts {
        cancel.check()?;
        let targets: Vec<_> = concept
            .expressions
            .iter()
            .filter(|e| e.language.eq_ignore_ascii_case(tl))
            .cloned()
            .collect();
        for expression in concept
            .expressions
            .iter()
            .filter(|e| e.language.eq_ignore_ascii_case(sl))
        {
            for range in occurrences(&source, expression) {
                if result.matches.len() >= 512 {
                    return Err(CatError::Invalid("Reconocimiento no evaluado: supera 512 coincidencias; reduce el texto de consulta".into()));
                }
                result.matches.push(TermMatch {
                    concept_id: concept.id,
                    base_name: concept.base_name.clone(),
                    source: expression.text.clone(),
                    source_range: range,
                    targets: targets.clone(),
                    domain: concept.domain.clone(),
                    notes: concept.notes.clone(),
                    provenance: concept.provenance.clone(),
                });
            }
        }
    }
    result
        .matches
        .sort_by_key(|m| (m.source_range.start, m.source_range.end, m.concept_id));
    result
        .matches
        .dedup_by_key(|m| (m.source_range.start, m.source_range.end, m.concept_id));
    let mut senses = HashMap::new();
    for found in &result.matches {
        *senses
            .entry((found.source_range.start, found.source_range.end))
            .or_insert(0) += 1;
    }
    for found in &result.matches {
        cancel.check()?;
        let mut add = |code, message, target_range| {
            result.issues.push(TermIssue {
                code,
                message,
                concept_id: found.concept_id,
                source_range: found.source_range.clone(),
                target_range,
            })
        };
        if senses[&(found.source_range.start, found.source_range.end)] > 1 {
            add(
                "term-ambiguous",
                format!(
                    "{}: varios conceptos; elige el sentido antes de evaluar equivalencias",
                    found.source
                ),
                None,
            );
            continue;
        }
        let allowed: Vec<_> = found
            .targets
            .iter()
            .filter(|e| e.status != TermStatus::Forbidden)
            .collect();
        if !allowed.is_empty() && !allowed.iter().any(|e| !occurrences(&target, e).is_empty()) {
            add(
                "term-missing",
                format!(
                    "{}: falta una equivalencia permitida de {}",
                    found.source, found.base_name
                ),
                None,
            );
        }
        for expression in found
            .targets
            .iter()
            .filter(|e| e.status == TermStatus::Forbidden)
        {
            for range in occurrences(&target, expression) {
                add(
                    "term-forbidden",
                    format!(
                        "{}: forma prohibida en {}",
                        expression.text, found.base_name
                    ),
                    Some(range),
                );
            }
        }
        if result.issues.len() > 512 {
            return Err(CatError::Invalid(
                "Terminología no evaluada: supera 512 incidencias".into(),
            ));
        }
    }
    cancel.check()?;
    Ok(result)
}
