//! The stylesheet checks.
//!
//! They live beside `theme.rs` rather than inside it so the module itself stays
//! the shared visual language and nothing else. As a child module they still
//! reach the private `document`, `BASE_CSS` and the palette.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::watch;
use super::*;

/// The lock the stylesheet checks take in turn.
///
/// GTK is not thread safe, and the checks in one binary run in parallel: two of
/// them parsing CSS at the same time report errors that are not there, or worse.
/// Each test holds this for its length.
static GTK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gtk() -> std::sync::MutexGuard<'static, ()> {
    GTK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A sheet that parses, for the tests that only need *something* the widget layer
/// could contain.
const SHIPPED: &str = ".shipped { font-size: 11px; }";

/// What GTK refused to parse. A stylesheet typo never fails a widget at
/// runtime - the rule simply stops applying - so the tests collect these.
///
/// The provider is built through `glib::Object::new` on purpose. The
/// generated `CssProvider::new` asserts that `gtk::init` has run, and that
/// needs a display, while parsing does not. Staying display-free is what
/// lets the check run in CI, where no compositor exists.
fn parse_errors(document: &str) -> Vec<String> {
    let errors = Rc::new(RefCell::new(Vec::new()));
    let sink = errors.clone();
    let provider: gtk4::CssProvider = gtk4::glib::Object::new();
    provider.connect_parsing_error(move |_, _, error| {
        sink.borrow_mut().push(error.to_string());
    });
    provider.load_from_data(document);
    let collected = errors.borrow().clone();
    collected
}

fn widget_sheet(widget: &str) -> String {
    let path = format!("{}/../{}/style.css", env!("CARGO_MANIFEST_DIR"), widget);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

/// Every stylesheet the widgets ship has to parse, on any machine: a rule
/// GTK rejects stops applying silently, so nothing else would notice.
///
/// The toolkit is taken for the length of this check, and every other check that
/// parses CSS takes it too — see [`gtk`].
#[test]
fn stylesheets_parse() {
    let _gtk = gtk();
    let palette_and_base = format!("{}\n{}", palette::palette_css(), BASE_CSS);
    assert_eq!(
        parse_errors(&palette_and_base),
        Vec::<String>::new(),
        "the palette and the shared base sheet must parse"
    );

    for widget in ["statusbar", "sidebar", "launcher"] {
        let document = format!("{}\n{}", palette_and_base, widget_sheet(widget));
        assert_eq!(
            parse_errors(&document),
            Vec::<String>::new(),
            "{widget} style.css does not parse on top of the shared base"
        );
    }

    // The assertions above only prove something while GTK reports what it
    // refuses. Without these, a change that stopped collecting errors would
    // leave the check green and empty instead of failing.
    assert!(!parse_errors("window { colour: red; }").is_empty());
    assert!(!parse_errors("window { background-color: notacolor; }").is_empty());
    assert!(parse_errors("window { background-color: #19120c; }").is_empty());
}

/// The palette, with the foreground moved to `fg`. Every other token is the one
/// the fallback defines, so a test that points `$HOME` at a directory of its own
/// still hands GTK a complete document.
fn palette_with(fg: &str) -> String {
    crate::palette::FALLBACK.replace(
        "@define-color palette_fg #e8e1dd;",
        &format!("@define-color palette_fg {fg};"),
    )
}

/// The files have to reach a *running* widget, not just a reload function: this
/// drives the real `GFileMonitor` over a real `palette.css` and a real user
/// sheet, the way `palette sync` and a hand edit do, and watches the provider
/// take the change.
///
/// A reload that loaded back the document built at start-up would leave the
/// provider exactly as it was — the file was watched and the change was thrown
/// away — which is why the check goes through the monitor rather than calling
/// the reload itself: the wiring from monitor to reload is half of what this is
/// about. Both halves of the document are rewritten, because the palette and the
/// user's own sheet are watched by two separate monitors.
#[test]
fn a_rewritten_palette_reaches_a_running_provider() {
    let _gtk = gtk();
    let home = std::env::temp_dir().join("widgets-theme-monitor-test");
    // Start from nothing: a file a run that failed part way leaves behind would
    // answer for this one.
    let _ = std::fs::remove_dir_all(&home);
    let config = home.join(".config/widgets");
    std::fs::create_dir_all(&config).expect("the test config directory");
    let previous = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    std::fs::write(config.join("palette.css"), palette_with("#ff0000")).expect("the palette");
    std::fs::write(config.join("statusbar.css"), ".probe { font-size: 13px; }").expect("a sheet");

    // Load first, then watch, exactly as `init` does: a provider that started
    // empty would make the first assertion below true for the wrong reason.
    let provider: gtk4::CssProvider = gtk4::glib::Object::new();
    provider.load_from_data(&rebuild("statusbar", SHIPPED));
    watch::watch(provider.clone(), "statusbar", SHIPPED);
    let before = provider.to_string();
    assert!(before.contains("palette_bg"), "the palette layer is loaded");

    std::fs::write(config.join("palette.css"), palette_with("#00ff00")).expect("rewrite it");
    assert!(
        pump_until(|| provider.to_string() != before),
        "a rewritten palette.css has to reach the provider"
    );

    std::fs::write(config.join("statusbar.css"), ".probe { font-size: 17px; }").expect("edit it");
    assert!(
        pump_until(|| provider.to_string().contains("17px")),
        "an edited user sheet has to reach the provider"
    );

    match previous {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// Pump the main context until `check` holds or the deadline passes. The file
/// monitors report there and nothing else in this test runs a loop, so it drives
/// the loop itself.
fn pump_until(check: impl Fn() -> bool) -> bool {
    let context = gtk4::glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        while context.iteration(false) {}
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    check()
}

/// The reload path rebuilds the whole document; the widget layer has to be
/// in there, or a palette change would quietly drop a widget's own rules.
#[test]
fn document_layers_are_all_present() {
    let widget = ".statusbar { color: @palette_fg; }";
    let full = document(widget);
    assert!(full.contains(BASE_CSS), "the shared base is missing");
    assert!(full.ends_with(widget), "the widget sheet is missing");
    assert!(full.contains("palette_bg"), "the palette layer is missing");
}
