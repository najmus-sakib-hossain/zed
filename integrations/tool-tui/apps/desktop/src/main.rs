mod ai;
mod assets;
mod components;
mod icons;
mod theme;
mod views;

use assets::{AppAssets, DynamicSvgAssets};
use gpui::{px, size, App, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;
use icons::IconDataLoader;
use std::collections::HashSet;
use theme::{Theme, ThemeMode};
use views::zed::ZedView;

fn main() {
    // Detect project root (look for Cargo.toml in ancestors)
    let project_root = detect_project_root()
        .unwrap_or_else(|| std::env::current_dir().expect("Could not determine current directory"));

    // Load all icon data before launching the UI
    let mut loader = IconDataLoader::new(&project_root);
    let _ = loader.load_all(); // Ignore errors silently

    // Create dynamic SVG asset source and register all loaded icons
    let dynamic_assets = DynamicSvgAssets::new();
    let mut flat_names_registered = HashSet::new();

    // Prefer these packs for flat aliases when multiple icons share the same name.
    let preferred_packs = ["lucide", "tabler", "heroicons", "material-symbols"];

    let mut icons_sorted: Vec<_> = loader.icons().iter().collect();
    icons_sorted.sort_by_key(|icon| {
        preferred_packs
            .iter()
            .position(|pack| *pack == icon.pack)
            .unwrap_or(preferred_packs.len())
    });

    for icon in icons_sorted {
        let path = format!("icons/{}/{}.svg", icon.pack, icon.name);
        dynamic_assets.register_svg(path, &icon.svg_body, icon.width, icon.height);

        let flat = format!("icons/{}.svg", icon.name);
        if !flat_names_registered.contains(&flat) {
            dynamic_assets.register_svg(flat.clone(), &icon.svg_body, icon.width, icon.height);
            flat_names_registered.insert(flat);
        }
    }

    // Create application with asset sources
    let app_assets = AppAssets::new(dynamic_assets);

    Application::new().with_assets(app_assets).run(move |cx: &mut App| {
        gpui_component::init(cx);

        // Use dark theme as default
        let theme = Theme::new(ThemeMode::Dark);

        let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| ZedView::new(theme.clone(), cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .unwrap();
    });
}

/// Walk up from the current exe or cwd to find the DX monorepo root
fn detect_project_root() -> Option<std::path::PathBuf> {
    // Try from current directory first
    let mut dir = std::env::current_dir().ok()?;
    loop {
        // Check for the monorepo marker files
        if dir.join("Cargo.toml").exists()
            && dir.join("apps").exists()
            && dir.join("crates").exists()
        {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }

    // Try from executable location
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent()?.to_path_buf();
        loop {
            if dir.join("Cargo.toml").exists()
                && dir.join("apps").exists()
                && dir.join("crates").exists()
            {
                return Some(dir);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    None
}
