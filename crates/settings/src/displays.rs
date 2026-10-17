//! Displays settings page and output controls.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, Label, Orientation, ScrolledWindow, StringList};

use crate::sway;

pub fn build_page() -> Box {
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
    unique_modes.sort_by_key(|a| std::cmp::Reverse(a.0 * a.1));

    let mode_strings: Vec<String> = unique_modes.iter().map(|(w, h)| format!("{w}×{h}")).collect();
    let string_list = StringList::new(
        &mode_strings.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
    );
    let dropdown = DropDown::new(Some(string_list), None::<gtk4::Expression>);
    dropdown.set_hexpand(false);
    dropdown.set_width_request(160);

    let dd = dropdown.clone();
    dropdown.connect_realize(move |_| {
        limit_dropdown_height(&dd);
    });

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

fn limit_dropdown_height(dropdown: &DropDown) {
    let mut child = dropdown.first_child();
    while let Some(w) = child {
        limit_scrolled_windows(&w);
        child = w.next_sibling();
    }
}

fn limit_scrolled_windows(widget: &gtk4::Widget) {
    if let Ok(sw) = widget.clone().downcast::<ScrolledWindow>() {
        sw.set_max_content_height(140);
        sw.set_propagate_natural_height(true);
    }
    let mut child = widget.first_child();
    while let Some(w) = child {
        limit_scrolled_windows(&w);
        child = w.next_sibling();
    }
}
