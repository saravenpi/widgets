//! The right-hand control row: one pill per control whose backend is present on
//! this machine, each leading with its icon and opening its own popup on click.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation};

use crate::icons;
use crate::popup::Popup;
use crate::{battery, brightness, logout, network, volume};

/// The assembled row, plus the labels that keep ticking after it is built.
///
/// Only battery and network are ticked: volume and brightness update when they
/// are clicked, because for those the popup is the source of truth.
pub struct Row {
    pub container: Box,
    pub battery_label: Option<Label>,
    pub network_label: Option<Label>,
}

/// Build the row. A control whose backend is missing is simply not placed, so
/// the bar reads correctly on a machine with no battery or no audio.
///
/// `painted` collects the glyphs on the pills so the palette can reach them
/// later; the caller keeps the same set. See [`icons::follow_palette`].
pub fn build(popup: &Popup, painted: &Rc<RefCell<icons::Painted>>) -> Row {
    let mut row = Row::new();

    if volume::available() {
        row.push(popup, painted, icons::Icon::Volume, volume::show, volume::refresh);
    }

    if brightness::available() {
        row.push(
            popup,
            painted,
            icons::Icon::Brightness,
            brightness::show,
            brightness::refresh,
        );
    }

    if battery::available() {
        let label = row.push(popup, painted, icons::Icon::Battery, battery::show, battery::refresh);
        row.battery_label = Some(label);
    }

    if network::available() {
        let label = row.push(popup, painted, icons::Icon::Network, network::show, network::refresh);
        row.network_label = Some(label);
    }

    if let Some(button) = logout::pill(painted) {
        row.container.append(&button);
    }

    row
}

impl Row {
    fn new() -> Self {
        const BAR_GAP: i32 = 4;
        let container = Box::new(Orientation::Horizontal, BAR_GAP);
        container.add_css_class("controls");
        container.set_hexpand(false);
        container.set_halign(gtk4::Align::End);
        Self {
            container,
            battery_label: None,
            network_label: None,
        }
    }

    /// Place one control's pill and hand back the label its ticker will refresh.
    fn push(
        &mut self,
        popup: &Popup,
        painted: &Rc<RefCell<icons::Painted>>,
        icon: icons::Icon,
        show: impl Fn(&Popup) + 'static,
        refresh: impl Fn(&Label) + 'static,
    ) -> Label {
        let label = Label::new(None);
        self.container
            .append(&pill(icon, &label, painted, popup, show, refresh));
        label
    }
}

/// One pill: the icon beside `label`, and a click that opens the control's own
/// popup and then pulls the label back up to date.
///
/// The label is made by the caller and handed in, so the label the button paints
/// is the one the tickers refresh — a pill that built its own internal label
/// would leave a ticker updating a widget nobody can see. `show` and `refresh`
/// are the control module's own functions, so the pill carries no per-control
/// logic of its own.
fn pill(
    icon: icons::Icon,
    label: &Label,
    painted: &Rc<RefCell<icons::Painted>>,
    popup: &Popup,
    show: impl Fn(&Popup) + 'static,
    refresh: impl Fn(&Label) + 'static,
) -> gtk4::Button {
    refresh(label);

    let child = Box::new(Orientation::Horizontal, 4);
    child.append(&painted.borrow_mut().image(icon));
    child.append(label);

    let button = gtk4::Button::new();
    button.set_child(Some(&child));
    button.add_css_class("pill");
    button.set_valign(gtk4::Align::Center);

    let popup = popup.clone();
    let label = label.clone();
    let anchor = button.clone();
    button.connect_clicked(move |_| {
        if popup.is_open_for(&anchor) {
            popup.hide();
            return;
        }
        popup.position_under(&anchor);
        show(&popup);
        refresh(&label);
    });
    button
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gtk4::gio;

    use super::*;

    /// A pill wired to a real popup, with counters standing in for a control's
    /// `show` and `refresh` so the test can tell how often each ran.
    ///
    /// `application_id` has to differ between the tests that call this: an
    /// application is exported on the bus by id, so two of them claiming the same
    /// one in a single run makes the second refuse to register.
    fn wired_pill(application_id: &str) -> (gtk4::Button, Label, Popup, Rc<Cell<u32>>) {
        let application = gtk4::Application::builder()
            .application_id(application_id)
            .build();
        application
            .register(None::<&gio::Cancellable>)
            .expect("register the test application");

        let popup = Popup::new(&application);
        let label = Label::new(None);
        let painted = Rc::new(RefCell::new(icons::Painted::default()));
        let refreshes = Rc::new(Cell::new(0u32));
        let counter = refreshes.clone();
        let button = pill(
            icons::Icon::Battery,
            &label,
            &painted,
            &popup,
            |popup| crate::popup::message(popup, "Battery", "50"),
            move |label| {
                let refreshes = counter.get() + 1;
                counter.set(refreshes);
                label.set_text(&format!("refresh {refreshes}"));
            },
        );
        (button, label, popup, refreshes)
    }

    /// Clicking a pill has to open the control's popup and pull its label back up
    /// to date. Both once went missing: the handler dropped the popup instead of
    /// showing it, so every control lost its popup, and the label the tickers
    /// refreshed was a copy that was never painted on the bar.
    ///
    /// GTK needs a display, so this stands down where there is none rather than
    /// fail on a machine that cannot show a window — run the suite against a
    /// display to actually exercise it. The body runs on the toolkit thread for
    /// the reason [`crate::testing`] gives.
    #[test]
    fn clicking_a_pill_shows_the_popup_and_refreshes_the_label() {
        let ran = crate::testing::on_gtk_thread(|| {
            let (button, label, popup, refreshes) = wired_pill("dev.saravenpi.ControlPillTest");

            assert_eq!(refreshes.get(), 1, "the pill refreshes itself once, when built");
            assert_eq!(label.text().as_str(), "refresh 1", "and paints what it read");
            assert!(!popup.is_visible(), "the popup starts off screen");

            button.emit_clicked();

            assert!(popup.is_visible(), "clicking the pill puts its popup on screen");
            assert_eq!(refreshes.get(), 2, "clicking the pill asks for a fresh reading");
            assert_eq!(label.text().as_str(), "refresh 2", "and paints that one");
        });
        if !ran {
            eprintln!("statusbar: no display, skipping the control pill test");
        }
    }

    /// A pill has to carry its glyph on screen, next to the label it ticks.
    ///
    /// This is the check the icons needed: they were rasterized from SVG, doing
    /// that failed silently on a machine without an SVG loader, and the only
    /// thing that changed was that the picture was never appended — a pill with
    /// a label and no icon looks exactly like a bar whose pills have no icons.
    ///
    /// Like the click check this needs a display and stands down without one.
    #[test]
    fn a_pill_carries_its_glyph_beside_its_label() {
        let ran = crate::testing::on_gtk_thread(|| {
            let (button, label, ..) = wired_pill("dev.saravenpi.ControlGlyphTest");
            let row = button
                .child()
                .and_then(|child| child.downcast::<Box>().ok())
                .expect("the pill holds a row");
            let glyph = row
                .first_child()
                .and_then(|child| child.downcast::<gtk4::Image>().ok())
                .expect("the row leads with the pill's icon");
            let texture = glyph
                .paintable()
                .and_then(|paintable| paintable.downcast::<gtk4::gdk::Texture>().ok())
                .expect("the icon has a texture to paint");

            assert_eq!(
                (texture.width(), texture.height()),
                (24, 24),
                "the glyph is drawn at the icon's own size"
            );
            let widget = label.clone().upcast_ref::<gtk4::Widget>().as_ptr();
            assert!(
                glyph
                    .next_sibling()
                    .is_some_and(|next| next.as_ptr() == widget),
                "and the label it ticks comes after it"
            );
        });
        if !ran {
            eprintln!("statusbar: no display, skipping the pill glyph test");
        }
    }
}
