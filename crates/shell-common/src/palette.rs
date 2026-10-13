const PALETTE_PATH: &str = "/widgets/palette.css";

const FALLBACK: &str = r#"
@define-color palette_bg #19120c;
@define-color palette_fg #e8e1dd;
@define-color palette_cursor #ffc079;
@define-color palette_color0 #261e15;
@define-color palette_color1 #ff8a70;
@define-color palette_color2 #a2b275;
@define-color palette_color3 #d6a163;
@define-color palette_color4 #5eb5e1;
@define-color palette_color5 #ca9dc7;
@define-color palette_color6 #79b7a8;
@define-color palette_color7 #ccc5c2;
@define-color palette_color8 #776b5f;
@define-color palette_color9 #ffcbc0;
@define-color palette_color10 #cede9e;
@define-color palette_color11 #ffcf99;
@define-color palette_color12 #a8deff;
@define-color palette_color13 #f9c8f4;
@define-color palette_color14 #a4e4d3;
@define-color palette_color15 #fcf5f4;
"#;

fn palette_file() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut path = std::path::PathBuf::from(home);
    path.push(".config");
    path.push(PALETTE_PATH.trim_start_matches('/'));
    Some(path)
}

pub fn palette_path() -> String {
    palette_file()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/widgets/palette.css".to_string())
}

pub fn palette_css() -> String {
    match palette_file() {
        Some(path) if path.exists() => std::fs::read_to_string(&path)
            .unwrap_or_else(|_| FALLBACK.to_string()),
        _ => FALLBACK.to_string(),
    }
}

pub fn apply_palette() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&palette_css());
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn apply_stylesheet(css: &str) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}