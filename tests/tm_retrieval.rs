use lumencat::{model::*, storage::ProjectStore};

#[test]
fn retrieval_keeps_rare_reference_among_many_common_tokens() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let mut store = ProjectStore::open(&directory.path().join("tm.db"))?;
    store.insert_tm_units(
        (0..5000).map(|id| TmUnit {
            source: format!("Order {id} ready for delivery"),
            target: format!("Pedido {id} listo para entrega"),
            source_lang: "en".into(),
            target_lang: "es".into(),
            raw_xml: String::new(),
        }),
        &Cancellation::default(),
    )?;
    let matches = store.matches("Order 2500 ready for dispatch", "en", "es")?;
    assert!(
        matches
            .iter()
            .any(|m| m.source == "Order 2500 ready for delivery"),
        "candidato específico perdido entre tokens comunes"
    );
    assert_eq!(store.concordance("2500", "en", "es")?.len(), 1);
    let duplicate = TmUnit {
        source: "Order 2500 ready for delivery".into(),
        target: "Pedido 2500 listo para entrega".into(),
        source_lang: "en".into(),
        target_lang: "es".into(),
        raw_xml: String::new(),
    };
    assert_eq!(
        store.insert_tm_units([duplicate.clone(), duplicate], &Cancellation::default())?,
        0
    );
    let cancelled = Cancellation::default();
    cancelled.cancel();
    assert!(matches!(
        store.search_page(1, 0, 128, "missing", &cancelled),
        Err(CatError::Cancelled)
    ));
    store.close()?;
    Ok(())
}
