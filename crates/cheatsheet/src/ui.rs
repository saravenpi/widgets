//! UI layout, search filtering, and presentation for the cheatsheet widget.

use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, Entry, Label, Orientation, PolicyType, ScrolledWindow};

use crate::model::{Keybind, KeybindConfig};
use crate::sync;

pub const WIDTH: i32 = 720;
pub const HEIGHT: i32 = 540;

pub fn build_surface(window: &ApplicationWindow, config: KeybindConfig) -> (Box, Entry) {
    let surface = Box::new(Orientation::Vertical, 0);
    surface.add_css_class("surface");
    surface.add_css_class("cheatsheet-dialog");
    surface.set_width_request(WIDTH);
    surface.set_height_request(HEIGHT);

    let (header, subtitle) = build_header(window, &config);
    surface.append(&header);

    let search_box = Box::new(Orientation::Vertical, 0);
    search_box.add_css_class("search-box");

    let search_entry = Entry::new();
    search_entry.add_css_class("search-entry");
    search_entry.set_placeholder_text(Some("Type to filter shortcuts... (e.g. terminal, workspace, audio)"));
    search_entry.set_hexpand(true);
    search_box.append(&search_entry);
    surface.append(&search_box);

    let (scroll, rows_by_category) = build_content(&config);
    surface.append(&scroll);

    // Wire live search filtering
    let rows_ref = Rc::new(rows_by_category);
    let rows_clone = Rc::clone(&rows_ref);
    let sub_clone = subtitle.clone();
    search_entry.connect_changed(move |entry| {
        let q = entry.text().trim().to_lowercase();
        let total_visible = filter_categories(&rows_clone, &q);
        update_search_subtitle(&sub_clone, &q, total_visible);
    });

    (surface, search_entry)
}

fn filter_categories(categories: &[(Box, Vec<(Box, String)>)], query: &str) -> usize {
    let mut total = 0;
    for (cat_box, rows) in categories {
        let visible = filter_rows(rows, query);
        cat_box.set_visible(visible > 0);
        total += visible;
    }
    total
}

fn filter_rows(rows: &[(Box, String)], query: &str) -> usize {
    let mut visible = 0;
    for (row_box, searchable) in rows {
        let matches = query.is_empty() || searchable.contains(query);
        row_box.set_visible(matches);
        if matches {
            visible += 1;
        }
    }
    visible
}

fn update_search_subtitle(subtitle: &Label, query: &str, total_visible: usize) {
    if query.is_empty() {
        subtitle.set_text("Universal keyboard shortcuts for Sway, Hyprland, and Niri");
    } else {
        subtitle.set_text(&format!("{total_visible} matching shortcuts found"));
    }
}

fn build_header(window: &ApplicationWindow, config: &KeybindConfig) -> (Box, Label) {
    let header = Box::new(Orientation::Horizontal, 0);
    header.add_css_class("dialog-header");

    let title_box = Box::new(Orientation::Vertical, 0);
    let title = Label::new(Some("Keyboard Shortcuts"));
    title.add_css_class("title");
    title.set_halign(gtk4::Align::Start);

    let subtitle = Label::new(Some("Universal keyboard shortcuts for Sway, Hyprland, and Niri"));
    subtitle.add_css_class("subtitle");
    subtitle.set_halign(gtk4::Align::Start);

    title_box.append(&title);
    title_box.append(&subtitle);
    title_box.set_hexpand(true);
    header.append(&title_box);

    let actions = Box::new(Orientation::Horizontal, 8);

    let sync_btn = Button::with_label("Sync");
    sync_btn.add_css_class("action");
    let cfg = config.clone();
    let sub_target = subtitle.clone();
    sync_btn.connect_clicked(move |btn| {
        match sync::sync_all(&cfg) {
            Ok(paths) => {
                btn.set_label("Synced!");
                sub_target.set_text(&format!("Synchronized {} configs (Sway, Hyprland, Niri)", paths.len()));
            }
            Err(e) => {
                btn.set_label("Error");
                sub_target.set_text(&format!("Sync failed: {e}"));
            }
        }
    });
    actions.append(&sync_btn);

    let close_btn = Button::with_label("Close");
    close_btn.add_css_class("action");
    close_btn.add_css_class("cancel");
    let close_win = window.clone();
    close_btn.connect_clicked(move |_| close_win.close());
    actions.append(&close_btn);

    header.append(&actions);
    (header, subtitle)
}

type CategoryRows = Vec<(Box, Vec<(Box, String)>)>;

fn build_content(config: &KeybindConfig) -> (ScrolledWindow, CategoryRows) {
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .build();
    scroll.add_css_class("content-scroll");

    let container = Box::new(Orientation::Vertical, 0);
    let mut category_records = Vec::new();

    for cat in &config.categories {
        let cat_box = Box::new(Orientation::Vertical, 0);
        cat_box.add_css_class("category-group");

        let title = Label::new(Some(&cat.name));
        title.add_css_class("category-title");
        title.set_halign(gtk4::Align::Start);
        cat_box.append(&title);

        let mut row_records = Vec::new();

        for bind in &cat.binds {
            let row = build_shortcut_row(bind, &config.mod_key);
            let searchable = format!(
                "{} {} {} {}",
                cat.name,
                bind.key,
                format_key_display(&bind.key, &config.mod_key),
                bind.desc
            ).to_lowercase();

            cat_box.append(&row);
            row_records.push((row, searchable));
        }

        container.append(&cat_box);
        category_records.push((cat_box, row_records));
    }

    scroll.set_child(Some(&container));
    (scroll, category_records)
}

fn build_shortcut_row(bind: &Keybind, mod_key: &str) -> Box {
    let row = Box::new(Orientation::Horizontal, 12);
    row.add_css_class("shortcut-row");

    let desc = Label::new(Some(&bind.desc));
    desc.add_css_class("shortcut-desc");
    desc.set_halign(gtk4::Align::Start);
    desc.set_hexpand(true);
    row.append(&desc);

    let display_key = format_key_display(&bind.key, mod_key);
    let pill = Label::new(Some(&display_key));
    pill.add_css_class("key-pill");
    pill.set_halign(gtk4::Align::End);
    row.append(&pill);

    row
}

fn format_key_display(key: &str, mod_key: &str) -> String {
    let mod_name = match mod_key {
        "Mod4" => "Super",
        "Mod1" => "Alt",
        other => other,
    };

    if key.starts_with("XF86") {
        match key {
            "XF86AudioRaiseVolume" => "Volume Up".to_string(),
            "XF86AudioLowerVolume" => "Volume Down".to_string(),
            "XF86AudioMute" => "Mute Audio".to_string(),
            "XF86AudioMicMute" => "Mute Mic".to_string(),
            "XF86MonBrightnessUp" => "Brightness Up".to_string(),
            "XF86MonBrightnessDown" => "Brightness Down".to_string(),
            other => other.to_string(),
        }
    } else {
        let parts: Vec<&str> = key.split('+').collect();
        let mut display_parts = vec![mod_name];
        for part in parts {
            let p = match part {
                "space" => "Space",
                "Return" => "Return",
                "question" => "?",
                "comma" => ",",
                other => other,
            };
            display_parts.push(p);
        }
        display_parts.join(" + ")
    }
}
