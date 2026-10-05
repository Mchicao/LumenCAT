use lumencat::gpui_app::theme::{Accent, Appearance, ColorMode, Theme};

#[test]
fn appearance_persists_all_choices_and_rejects_invalid_settings()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("settings/appearance.json");
    assert_eq!(Appearance::load(&path)?, Appearance::default());
    for mode in [ColorMode::Light, ColorMode::Dark] {
        for accent in [Accent::Blue, Accent::Teal, Accent::Violet] {
            let appearance = Appearance { mode, accent };
            appearance.save(&path)?;
            assert_eq!(Appearance::load(&path)?, appearance);
            Theme::apply(appearance);
            let controls = Theme::component_colors();
            let luminance = |color: gpui::Rgba| {
                let linear = |value: f32| {
                    if value <= 0.04045 {
                        value / 12.92
                    } else {
                        ((value + 0.055) / 1.055).powf(2.4)
                    }
                };
                0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
            };
            for (foreground, background) in [
                (Theme::text_primary(), Theme::bg_app()),
                (Theme::text_muted(), Theme::bg_surface()),
                (Theme::text_on_accent(), Theme::sky()),
                (Theme::text_on_accent(), Theme::emerald()),
                (controls.foreground.into(), controls.background.into()),
                (controls.muted_foreground.into(), controls.background.into()),
                (
                    controls.button_primary_foreground.into(),
                    controls.button_primary.into(),
                ),
                (
                    controls.button_secondary_foreground.into(),
                    controls.button_secondary.into(),
                ),
                (
                    controls.button_success_foreground.into(),
                    controls.button_success.into(),
                ),
                (
                    controls.button_danger_foreground.into(),
                    controls.button_danger.into(),
                ),
            ] {
                let a = luminance(foreground);
                let b = luminance(background);
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                    "Contraste insuficiente: {appearance:?}"
                );
            }
        }
    }
    std::fs::write(&path, r#"{"mode":"unknown","accent":"blue"}"#)?;
    let invalid = std::fs::read(&path)?;
    assert!(Appearance::load(&path).is_err());
    assert_eq!(std::fs::read(&path)?, invalid);
    let blocker = directory.path().join("not-a-directory");
    std::fs::write(&blocker, "keep")?;
    assert!(
        Appearance::default()
            .save(&blocker.join("appearance.json"))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(blocker)?, "keep");
    Ok(())
}
