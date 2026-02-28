#[test]
#[ignore = "Requires style.bin with pre-built themes - run `dx style build` first"]
fn dx_default_themes_are_loaded() {
    let engine = style::core::AppState::engine();

    let light = engine
        .theme_by_name("dx.light")
        .expect("dx.light theme is present in style.bin");
    let dark = engine.theme_by_name("dx.dark").expect("dx.dark theme is present in style.bin");

    let light_background = light
        .tokens
        .iter()
        .find(|(name, _)| name == "background")
        .map(|(_, value)| value.as_str())
        .expect("dx.light background token");
    let dark_background = dark
        .tokens
        .iter()
        .find(|(name, _)| name == "background")
        .map(|(_, value)| value.as_str())
        .expect("dx.dark background token");

    assert_eq!(light_background, "oklch(0.99 0 0)");
    assert_eq!(dark_background, "oklch(0.13 0 0)");
}
