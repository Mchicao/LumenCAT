use lumencat::qa::check;

#[test]
fn qa_flags_loss_of_numbers_and_protected_tokens_without_changing_text() {
    let source = "Send 42 items to https://example.org for ${user}.";
    let target = "Envía 24 artículos.";
    let issues = check(source, target, false);
    for expected in ["numbers", "tokens", "unconfirmed"] {
        assert!(issues.iter().any(|issue| issue.code == expected));
    }
    assert!(check("項目 42。", "项目 42。", true).is_empty());
}
