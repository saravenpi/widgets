//! UI layout and components for the settings widget.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, DropDown, Label, Orientation, Stack, StringList,
};

use crate::sway;

pub const WIDTH: i32 = 680;
pub const HEIGHT: i32 = 460;

pub fn build_surface(window: &ApplicationWindow) -> Box {
    let surface = Box::new(Orientation::Vertical, 0);
    surface.add_css_class("surface");
    surface.add_css_class("settings-dialog");
    surface.set_width_request(WIDTH);
    surface.set_height_request(HEIGHT);

    surface.append(&build_header(window));
    surface.append(&build_body());
    surface
}

fn build_header(window: &ApplicationWindow) -> Box {
    let header = Box::new(Orientation::Horizontal, 0);
    header.add_css_class("dialog-header");

    let title_box = Box::new(Orientation::Vertical, 0);
    let title = Label::new(Some("Settings"));
    title.add_css_class("title");
    title.set_halign(gtk4::Align::Start);

    let subtitle = Label::new(Some("System configuration and display management"));
    subtitle.add_css_class("subtitle");
    subtitle.set_halign(gtk4::Align::Start);

    title_box.append(&title);
    title_box.append(&subtitle);
    title_box.set_hexpand(true);
    header.append(&title_box);

    let close_btn = Button::with_label("Close");
    close_btn.add_css_class("action");
    close_btn.add_css_class("cancel");
    let close_win = window.clone();
    close_btn.connect_clicked(move |_| close_win.close());
    header.append(&close_btn);

    header
}

fn build_body() -> Box {
    let body = Box::new(Orientation::Horizontal, 0);
    body.set_vexpand(true);

    let nav = Box::new(Orientation::Vertical, 0);
    nav.add_css_class("sidebar-nav");

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let tab_displays = Button::with_label("Displays");
    tab_displays.add_css_class("nav-tab");
    tab_displays.add_css_class("active");

    let tab_appearance = Button::with_label("Appearance");
    tab_appearance.add_css_class("nav-tab");

    let tab_about = Button::with_label("About");
    tab_about.add_css_class("nav-tab");

    wire_tabs(
        &stack,
        &tab_displays,
        &tab_appearance,
        &tab_about,
    );

    nav.append(&tab_displays);
    nav.append(&tab_appearance);
    nav.append(&tab_about);
    body.append(&nav);

    stack.add_named(&build_displays_page(), Some("displays"));
    stack.add_named(&build_appearance_page(), Some("appearance"));
    stack.add_named(&build_about_page(), Some("about"));
    body.append(&stack);

    body
}

fn wire_tabs(
    stack: &Stack,
    tab_displays: &Button,
    tab_appearance: &Button,
    tab_about: &Button,
) {
    let s1 = stack.clone();
    let td1 = tab_displays.clone();
    let ta1 = tab_appearance.clone();
    let tb1 = tab_about.clone();
    tab_displays.connect_clicked(move |_| {
        s1.set_visible_child_name("displays");
        td1.add_css_class("active");
        ta1.remove_css_class("active");
        tb1.remove_css_class("active");
    });

    let s2 = stack.clone();
    let td2 = tab_displays.clone();
    let ta2 = tab_appearance.clone();
    let tb2 = tab_about.clone();
    tab_appearance.connect_clicked(move |_| {
        s2.set_visible_child_name("appearance");
        td2.remove_css_class("active");
        ta2.add_css_class("active");
        tb2.remove_css_class("active");
    });

    let s3 = stack.clone();
    let td3 = tab_displays.clone();
    let ta3 = tab_appearance.clone();
    let tb3 = tab_about.clone();
    tab_about.connect_clicked(move |_| {
        s3.set_visible_child_name("about");
        td3.remove_css_class("active");
        ta3.remove_css_class("active");
        tb3.add_css_class("active");
    });
}

fn build_displays_page() -> Box {
    let page = Box::new(Orientation::Vertical, 12);
    page.add_css_class("content-pane");

    let heading = Label::new(Some("Display Settings"));
    heading.add_css_class("section-title");
    heading.set_halign(gtk4::Align::Start);
    page.append(&heading);

    match sway::get_outputs() {
        Ok(outputs) if !outputs.is_empty() => {
            for output in outputs {
                page.append(&build_output_card(&output));
            }
        }
        Ok(_) => {
            let empty = Label::new(Some("No active outputs detected in Sway."));
            empty.add_css_class("dim");
            page.append(&empty);
        }
        Err(err) => {
            let err_lbl = Label::new(Some(&format!("Error connecting to Sway: {err}")));
            err_lbl.add_css_class("dim");
            page.append(&err_lbl);
        }
    }

    page
}

fn build_output_card(output: &sway::Output) -> Box {
    let card = Box::new(Orientation::Vertical, 10);
    card.add_css_class("card");

    let name_label = Label::new(Some(&format!("{} ({})", output.name, output.model)));
    name_label.add_css_class("card-title");
    name_label.set_halign(gtk4::Align::Start);
    card.append(&name_label);

    let cur_desc = match output.current_mode {
        Some(m) => format!(
            "Current: {}×{} @ {:.1} Hz (Scale {:.2}x)",
            m.width,
            m.height,
            m.refresh_hz(),
            output.scale
        ),
        None => "Current: Unknown".to_string(),
    };
    let status_label = Label::new(Some(&cur_desc));
    status_label.add_css_class("card-desc");
    status_label.set_halign(gtk4::Align::Start);
    card.append(&status_label);

    let mut unique_modes: Vec<(i32, i32)> = Vec::new();
    for m in &output.modes {
        if !unique_modes.iter().any(|(w, h)| *w == m.width && *h == m.height) {
            unique_modes.push((m.width, m.height));
        }
    }
    unique_modes.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));

    let mode_strings: Vec<String> = unique_modes.iter().map(|(w, h)| format!("{w}×{h}")).collect();
    let string_list = StringList::new(
        &mode_strings.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
    );
    let dropdown = DropDown::new(Some(string_list), None::<gtk4::Expression>);
    dropdown.set_hexpand(true);

    if let Some(cur) = output.current_mode {
        if let Some(idx) = unique_modes.iter().position(|(w, h)| *w == cur.width && *h == cur.height) {
            dropdown.set_selected(idx as u32);
        }
    }

    let row_mode = Box::new(Orientation::Horizontal, 10);
    let lbl_res = Label::new(Some("Resolution:"));
    lbl_res.add_css_class("dim");
    row_mode.append(&lbl_res);
    row_mode.append(&dropdown);
    card.append(&row_mode);

    let (scale_row, selected_scale) = build_scale_selector(output.scale);
    card.append(&scale_row);

    card.append(&build_apply_row(
        &output.name,
        unique_modes,
        dropdown,
        status_label,
        selected_scale,
    ));

    card
}

fn build_scale_selector(current_scale: f64) -> (Box, Rc<RefCell<f64>>) {
    let scale_row = Box::new(Orientation::Horizontal, 6);
    let lbl_scale = Label::new(Some("Scale:"));
    lbl_scale.add_css_class("dim");
    scale_row.append(&lbl_scale);

    let scales = [1.0, 1.25, 1.5, 2.0];
    let selected_scale = Rc::new(RefCell::new(current_scale));
    let scale_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    for &s in &scales {
        let btn = Button::with_label(&format!("{:.2}x", s));
        btn.add_css_class("scale-pill");
        if (current_scale - s).abs() < 0.05 {
            btn.add_css_class("active");
        }
        let sel = selected_scale.clone();
        let s_val = s;
        let btns = scale_buttons.clone();
        btn.connect_clicked(move |clicked_btn| {
            *sel.borrow_mut() = s_val;
            for b in btns.borrow().iter() {
                b.remove_css_class("active");
            }
            clicked_btn.add_css_class("active");
        });
        scale_buttons.borrow_mut().push(btn.clone());
        scale_row.append(&btn);
    }

    (scale_row, selected_scale)
}

fn build_apply_row(
    out_name: &str,
    modes_list: Vec<(i32, i32)>,
    dropdown: DropDown,
    status_label: Label,
    selected_scale: Rc<RefCell<f64>>,
) -> Box {
    let actions_row = Box::new(Orientation::Horizontal, 10);
    actions_row.set_margin_top(8);

    let apply_btn = Button::with_label("Apply Resolution");
    apply_btn.add_css_class("action");
    apply_btn.add_css_class("primary");

    let feedback = Label::new(None);
    feedback.add_css_class("dim");
    feedback.set_halign(gtk4::Align::Start);

    let name = out_name.to_string();
    let fb = feedback.clone();
    apply_btn.connect_clicked(move |_| {
        let sel_idx = dropdown.selected() as usize;
        if let Some(&(w, h)) = modes_list.get(sel_idx) {
            let scale = *selected_scale.borrow();
            match sway::set_mode(&name, w, h, scale) {
                Ok(()) => {
                    fb.set_text("Applied successfully");
                    status_label.set_text(&format!("Current: {w}×{h} (Scale {scale:.2}x)"));
                }
                Err(e) => {
                    fb.set_text(&format!("Failed: {e}"));
                }
            }
        }
    });

    actions_row.append(&apply_btn);
    actions_row.append(&feedback);
    actions_row
}

fn build_appearance_page() -> Box {
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

    let card_desc = Label::new(Some(
        "Widgets automatically follow the active palette generated from wallpaper or colorscheme.\nClick Sync Palette to trigger an immediate reload across all applications."
    ));
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

fn build_about_page() -> Box {
    let page = Box::new(Orientation::Vertical, 12);
    page.add_css_class("content-pane");

    let heading = Label::new(Some("About System"));
    heading.add_css_class("section-title");
    heading.set_halign(gtk4::Align::Start);
    page.append(&heading);

    let card = Box::new(Orientation::Vertical, 8);
    card.add_css_class("card");

    let row1 = info_row("Operating System:", "Alpine Linux v3.24");
    let row2 = info_row("Compositor:", "Sway (Wayland)");
    let row3 = info_row("Widgets Version:", env!("CARGO_PKG_VERSION"));
    let row4 = info_row("Configuration:", "~/.config/widgets/config.yml");

    card.append(&row1);
    card.append(&row2);
    card.append(&row3);
    card.append(&row4);

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
