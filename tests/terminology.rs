use lumencat::{model::*, storage::ProjectStore, terminology};

fn expression(language: &str, text: &str, status: TermStatus) -> TermExpression {
    TermExpression {
        language: language.into(),
        text: text.into(),
        status,
        case_sensitive: false,
    }
}

fn concept(base_id: i64) -> NewTermConcept {
    NewTermConcept {
        base_id,
        domain: "Legal".into(),
        notes: "Aviso al usuario".into(),
        provenance: "Cliente A".into(),
        expressions: vec![
            expression("en", "privacy notice", TermStatus::Preferred),
            expression("es", "aviso de privacidad", TermStatus::Preferred),
            expression("es", "aviso de protección de datos", TermStatus::Allowed),
            expression("es", "nota privada", TermStatus::Forbidden),
            expression("fr", "avis de confidentialité", TermStatus::Preferred),
        ],
    }
}

#[test]
fn concepts_variants_bases_and_language_filters_survive_restart() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("terms.lcat");
    let mut store = ProjectStore::open(&path)?;
    let base = store.create_term_base("Cliente A")?;
    let other = store.create_term_base("Cliente B")?;
    let id = store.add_term_concept(&concept(base), &Cancellation::default())?;
    let mut regional = concept(other);
    regional.expressions[1].language = "es-MX".into();
    regional.expressions.retain(|e| e.language != "es");
    store.add_term_concept(&regional, &Cancellation::default())?;
    assert_eq!(store.term_concepts("en", "es")?.len(), 1);
    assert_eq!(store.term_concepts("en", "es-MX")?.len(), 1);
    store.set_term_base_enabled(base, false)?;
    assert!(store.term_concepts("en", "es")?.is_empty());
    store.set_term_base_enabled(base, true)?;
    store.close()?;
    drop(store);
    let mut reopened = ProjectStore::open_existing(&path)?;
    let terms = reopened.term_concepts("EN", "ES")?;
    assert_eq!(terms[0].id, id);
    assert_eq!(terms[0].expressions.len(), 5);
    assert_eq!(terms[0].notes, "Aviso al usuario");
    assert_eq!(terms[0].provenance, "Cliente A");
    assert_eq!(reopened.term_bases()?.len(), 2);
    let cancelled = Cancellation::default();
    cancelled.cancel();
    assert!(matches!(
        reopened.add_term_concept(&concept(base), &cancelled),
        Err(CatError::Cancelled)
    ));
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER reject_term BEFORE INSERT ON term_expressions WHEN NEW.language='fr' BEGIN SELECT RAISE(ABORT,'induced'); END;")?;
    assert!(
        reopened
            .add_term_concept(&concept(base), &Cancellation::default())
            .is_err()
    );
    assert_eq!(reopened.term_concepts("en", "es")?.len(), 1);
    assert_eq!(
        connection.query_row("SELECT count(*) FROM term_concepts", [], |r| r
            .get::<_, i64>(0))?,
        2
    );
    reopened.close()?;
    Ok(())
}

#[test]
fn recognition_checks_whole_phrases_alternatives_and_forbidden_ranges() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("terms.lcat"))?;
    let base = store.create_term_base("Cliente A")?;
    store.add_term_concept(&concept(base), &Cancellation::default())?;
    let mut art = concept(base);
    art.expressions = vec![
        expression("en", "art", TermStatus::Preferred),
        expression("es", "arte", TermStatus::Preferred),
    ];
    store.add_term_concept(&art, &Cancellation::default())?;
    let terms = store.term_concepts("en", "es")?;
    let check = |source: &str, target: &str| {
        terminology::check(
            &terms,
            source,
            target,
            "en",
            "es",
            DocumentFormat::Txt,
            &Cancellation::default(),
        )
    };
    let result = check(
        "Read the PRIVACY NOTICE.",
        "Lea el aviso de protección de datos.",
    )?;
    assert_eq!(result.matches.len(), 1);
    assert_eq!(
        &"Read the PRIVACY NOTICE."[result.matches[0].source_range.clone()],
        "PRIVACY NOTICE"
    );
    assert!(result.issues.is_empty());
    assert!(check("privacy notices and party", "")?.matches.is_empty());
    assert_eq!(check("privacy notice", "")?.issues[0].code, "term-missing");
    let target = "Lea la nota privada.";
    let forbidden = check("privacy notice", target)?;
    let issue = forbidden
        .issues
        .iter()
        .find(|i| i.code == "term-forbidden")
        .ok_or_else(|| CatError::Invalid("sin issue prohibido".into()))?;
    assert_eq!(
        &target[issue
            .target_range
            .clone()
            .ok_or_else(|| CatError::Invalid("sin rango".into()))?],
        "nota privada"
    );
    let mut ambiguous = concept(base);
    ambiguous.expressions = vec![
        expression("en", "privacy notice", TermStatus::Preferred),
        expression("es", "declaración de privacidad", TermStatus::Preferred),
    ];
    store.add_term_concept(&ambiguous, &Cancellation::default())?;
    let result = terminology::check(
        &store.term_concepts("en", "es")?,
        "privacy notice",
        "aviso de privacidad",
        "en",
        "es",
        DocumentFormat::Txt,
        &Cancellation::default(),
    )?;
    assert_eq!(result.matches.len(), 2);
    assert!(result.issues.iter().all(|i| i.code == "term-ambiguous"));
    store.close()?;
    Ok(())
}

#[test]
fn cancellation_and_excessive_matches_are_not_reported_as_clean() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("terms.lcat"))?;
    let base = store.create_term_base("Cliente")?;
    store.add_term_concept(&concept(base), &Cancellation::default())?;
    let terms = store.term_concepts("en", "es")?;
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(matches!(
        terminology::check(
            &terms,
            "privacy notice",
            "",
            "en",
            "es",
            DocumentFormat::Txt,
            &cancel
        ),
        Err(CatError::Cancelled)
    ));
    assert!(matches!(
        terminology::check(
            &terms,
            &"privacy notice ".repeat(513),
            "",
            "en",
            "es",
            DocumentFormat::Txt,
            &Cancellation::default()
        ),
        Err(CatError::Invalid(_))
    ));
    store.close()?;
    Ok(())
}

#[test]
fn unicode_offsets_case_policy_and_docx_codes_are_not_literal_text() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("terms.lcat"))?;
    let base = store.create_term_base("Unicode")?;
    let mut entry = concept(base);
    entry.expressions = vec![
        expression("fr", "café", TermStatus::Preferred),
        expression("es", "Café", TermStatus::Preferred),
    ];
    entry.expressions[1].case_sensitive = true;
    store.add_term_concept(&entry, &Cancellation::default())?;
    let terms = store.term_concepts("fr", "es")?;
    let source = "🙂 <g id=\"42\">cafe\u{301}</g>";
    let result = terminology::check(
        &terms,
        source,
        "<g id=\"42\">Café</g>",
        "fr",
        "es",
        DocumentFormat::Docx,
        &Cancellation::default(),
    )?;
    assert_eq!(
        &source[result.matches[0].source_range.clone()],
        "cafe\u{301}"
    );
    assert!(result.issues.is_empty());
    let lower = terminology::check(
        &terms,
        source,
        "café",
        "fr",
        "es",
        DocumentFormat::Docx,
        &Cancellation::default(),
    )?;
    assert_eq!(lower.issues[0].code, "term-missing");
    let mut literal = concept(base);
    literal.expressions = vec![
        expression("en", "id", TermStatus::Preferred),
        expression("es", "identificador", TermStatus::Preferred),
    ];
    store.add_term_concept(&literal, &Cancellation::default())?;
    let terms = store.term_concepts("en", "es")?;
    assert!(
        terminology::check(
            &terms,
            source,
            "",
            "en",
            "es",
            DocumentFormat::Docx,
            &Cancellation::default()
        )?
        .matches
        .is_empty()
    );
    assert_eq!(
        terminology::check(
            &terms,
            source,
            "",
            "en",
            "es",
            DocumentFormat::Txt,
            &Cancellation::default()
        )?
        .matches
        .len(),
        1
    );
    store.close()?;
    Ok(())
}
