//! Appearance and About pages for the settings widget.

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};

pub fn build_appearance_page() -> Box {
    let page = Box::new(Orientation::Vertical, 12);
    page.add_css_class("content-pane");

    let heading = Label::new(Some("Appearance & Theme"));
    heading.add_css_class("section-title");
    heading.set_halign(gtk4::Align::Start);
    page.append(&heading);

    let card = Box::new(Orientation::Vertical, 10);
    card.add_css_class("card");

    let card_title = Label::new(Some("Palette Synchronization"));
    card_title.add_css_class("card-title");
    card_title.set_halign(gtk4::Align::Start);
    card.append(&card_title);

    let desc_text = "Widgets automatically follow the active palette generated from wallpaper or \
colorscheme.\nClick Sync Palette to trigger an immediate reload across all applications.";
    let card_desc = Label::new(Some(desc_text));
    card_desc.add_css_class("card-desc");
    card_desc.set_halign(gtk4::Align::Start);
    card.append(&card_desc);

    let sync_btn = Button::with_label("Sync Palette");
    sync_btn.add_css_class("action");
    sync_btn.add_css_class("primary");
    let feedback = Label::new(None);
    feedback.add_css_class("dim");

    let fb = feedback.clone();
    sync_btn.connect_clicked(move |_| {
        match std::process::Command::new("palette").arg("sync").spawn() {
            Ok(_) => fb.set_text("Palette sync triggered"),
            Err(e) => fb.set_text(&format!("Sync failed: {e}")),
        }
    });

    let row = Box::new(Orientation::Horizontal, 10);
    row.append(&sync_btn);
    row.append(&feedback);
    card.append(&row);

    page.append(&card);
    page
}

pub fn build_about_page() -> Box {
    let page = Box::new(Orientation::Vertical, 12);
    page.add_css_class("content-pane");

    let heading = Label::new(Some("About System"));
    heading.add_css_class("section-title");
    heading.set_halign(gtk4::Align::Start);
    page.append(&heading);

    let card = Box::new(Orientation::Vertical, 8);
    card.add_css_class("card");

    card.append(&info_row("Operating System:", "Alpine Linux v3.24"));
    card.append(&info_row("Compositor:", "Sway (Wayland)"));
    card.append(&info_row("Widgets Version:", env!("CARGO_PKG_VERSION")));
    card.append(&info_row("Configuration:", "~/.config/widgets/config.yml"));

    page.append(&card);
    page
}

fn info_row(label_text: &str, value_text: &str) -> Box {
    let row = Box::new(Orientation::Horizontal, 8);
    let l = Label::new(Some(label_text));
    l.add_css_class("dim");
    l.set_halign(gtk4::Align::Start);
    l.set_width_request(140);
    let v = Label::new(Some(value_text));
    v.add_css_class("badge");
    v.set_halign(gtk4::Align::Start);
    row.append(&l);
    row.append(&v);
    row
}
