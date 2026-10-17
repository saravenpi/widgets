//! UI layout and navigation for the settings widget.

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, Label, Orientation, Stack};

use crate::displays;
use crate::pages;

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

fn nav_button(text: &str) -> Button {
    let btn = Button::new();
    btn.add_css_class("nav-tab");
    btn.set_hexpand(true);
    btn.set_halign(gtk4::Align::Fill);
    let label = Label::new(Some(text));
    label.set_halign(gtk4::Align::Start);
    btn.set_child(Some(&label));
    btn
}

fn build_body() -> Box {
    let body = Box::new(Orientation::Horizontal, 0);
    body.set_vexpand(true);

    let nav = Box::new(Orientation::Vertical, 0);
    nav.add_css_class("sidebar-nav");

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let tab_displays = nav_button("Displays");
    tab_displays.add_css_class("active");

    let tab_appearance = nav_button("Appearance");
    let tab_about = nav_button("About");

    wire_tabs(&stack, &tab_displays, &tab_appearance, &tab_about);

    nav.append(&tab_displays);
    nav.append(&tab_appearance);
    nav.append(&tab_about);
    body.append(&nav);

    stack.add_named(&displays::build_page(), Some("displays"));
    stack.add_named(&pages::build_appearance_page(), Some("appearance"));
    stack.add_named(&pages::build_about_page(), Some("about"));
    body.append(&stack);

    body
}

fn wire_tabs(stack: &Stack, td: &Button, ta: &Button, tb: &Button) {
    let s1 = stack.clone();
    let td1 = td.clone();
    let ta1 = ta.clone();
    let tb1 = tb.clone();
    td.connect_clicked(move |_| {
        s1.set_visible_child_name("displays");
        td1.add_css_class("active");
        ta1.remove_css_class("active");
        tb1.remove_css_class("active");
    });

    let s2 = stack.clone();
    let td2 = td.clone();
    let ta2 = ta.clone();
    let tb2 = tb.clone();
    ta.connect_clicked(move |_| {
        s2.set_visible_child_name("appearance");
        td2.remove_css_class("active");
        ta2.add_css_class("active");
        tb2.remove_css_class("active");
    });

    let s3 = stack.clone();
    let td3 = td.clone();
    let ta3 = ta.clone();
    let tb3 = tb.clone();
    tb.connect_clicked(move |_| {
        s3.set_visible_child_name("about");
        td3.remove_css_class("active");
        ta3.remove_css_class("active");
        tb3.add_css_class("active");
    });
}
