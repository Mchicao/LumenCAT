use crate::model::QaIssue;

/// QA textual local: advertencias informativas; nunca altera ni confirma target.
pub fn check(source: &str, target: &str, confirmed: bool) -> Vec<QaIssue> {
    let mut issues = Vec::new();
    let mut add = |code, message: &str| {
        issues.push(QaIssue {
            code,
            message: message.into(),
        })
    };
    if !source.trim().is_empty() && target.trim().is_empty() {
        add("empty", "Traducción vacía");
    }
    if !source.is_empty() && source == target {
        add(
            "identical",
            "Traducción idéntica a la fuente; comprobar si es intencional",
        );
    }
    if target.contains("  ") {
        add("double-space", "Espacios dobles en traducción");
    }
    if source.starts_with(char::is_whitespace) != target.starts_with(char::is_whitespace)
        || source.ends_with(char::is_whitespace) != target.ends_with(char::is_whitespace)
    {
        add("edge-space", "Espacios iniciales/finales distintos");
    }
    if numbers(source) != numbers(target) {
        add(
            "numbers",
            "Dígitos distintos; comprobar números y formato local",
        );
    }
    if protected_tokens(source) != protected_tokens(target) {
        add("tokens", "URL, email o variable distinta/ausente");
    }
    if let Some(last) = source.trim_end().chars().last()
        && ".!?。！？:;".contains(last)
        && !target.trim_end().ends_with(last)
    {
        add(
            "punctuation",
            "Puntuación final distinta; comprobar convención del idioma",
        );
    }
    if !confirmed {
        add("unconfirmed", "Segmento sin confirmar");
    }
    issues
}

fn numbers(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut number = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_numeric() {
            number.push(character);
        } else if !number.is_empty() {
            result.push(std::mem::take(&mut number));
        }
    }
    result.sort_unstable();
    result
}

fn protected_tokens(text: &str) -> Vec<&str> {
    let mut result: Vec<_> = text
        .split_whitespace()
        .filter(|token| {
            token.contains("://")
                || token.contains('@')
                || token.starts_with("${")
                || token.starts_with('{')
                || token.starts_with('%')
        })
        .map(|token| token.trim_matches(|c: char| ",;.!?()[]".contains(c)))
        .collect();
    result.sort_unstable();
    result
}
