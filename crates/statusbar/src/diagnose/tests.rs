//! The report's wording: the two sides of a control's line, and what a machine's
//! own identifiers make of it.
//!
//! `report()` itself is not called here: it initialises GTK to ask whether the
//! glyphs paint, and a check that initialised the toolkit on a thread of its own
//! would be a second toolkit in the process. It is exercised by running
//! `statusbar --diagnose`, which is the interface it was written for.

use super::*;

/// A control is reported with the reason that matches whether it is in the row:
/// "placed" is what was found, "absent" is what to install.
#[test]
fn a_control_reads_for_the_side_it_landed_on() {
    let placed = row("volume", true, "pactl answers", "no pactl");
    assert!(placed.contains("volume"), "{placed}");
    assert!(placed.contains("placed"), "{placed}");
    assert!(placed.contains("pactl answers"), "{placed}");
    assert!(!placed.contains("no pactl"), "{placed}");

    let absent = row("battery", false, "a battery is present", "no BAT device");
    assert!(absent.contains("battery"), "{absent}");
    assert!(absent.contains("absent"), "{absent}");
    assert!(absent.contains("no BAT device"), "{absent}");
    assert!(!absent.contains("a battery is present"), "{absent}");
}

/// The report has to name every control it is asked about, so a missing one is
/// visible as a missing control rather than as a missing line.
#[test]
fn every_control_gets_a_line() {
    let text = pills();
    for control in ["volume", "brightness", "battery", "network", "logout"] {
        assert!(text.contains(control), "the report is missing {control}");
    }
}

/// The firmware identifiers are only worth reporting as far as they are readable,
/// and a machine that publishes none is not a virtual one.
#[test]
fn a_machine_without_identifiers_is_not_called_virtual() {
    let name = machine();
    assert!(!name.is_empty(), "the report always has something to print");
    assert!(
        !VIRTUAL_VENDORS.contains(&name.as_str()),
        "a bare vendor name is not a machine: {name}"
    );
}

/// The display and the renderer lines have to answer even when nothing is set,
/// because those two empty answers are the ones a reader is looking for.
#[test]
fn the_environment_lines_answer_when_nothing_is_set() {
    assert!(!renderer().is_empty(), "the renderer line is never blank");
    let display = display();
    assert!(
        display == "wayland" || display == "none" || display.starts_with("x11"),
        "unexpected display answer: {display}"
    );
}
