//! The pill's own checks: the words it puts beside the level, and the one line
//! the popup adds under it.
//!
//! These are the surface that made the reading worth fixing, so they are pinned
//! here rather than left to the fixture checks next door: a state the reading can
//! now produce has to have something sensible to say on screen.

use super::*;

/// Every state has a label, and the plugged one is the `AC` the bar was missing.
/// A state that fell through to `?` on a machine that was in fact charging is the
/// whole reason this was looked at.
#[test]
fn every_state_has_a_label() {
    assert_eq!(state_label(State::Charging), "Chg");
    assert_eq!(state_label(State::Discharging), "Dis");
    assert_eq!(state_label(State::Full), "Full");
    assert_eq!(state_label(State::Plugged), "AC");
    assert_eq!(state_label(State::Unknown), "?");
}

/// The popup's time line belongs to the two states where the battery's level is
/// moving: charging counts up to full, discharging counts down to empty.
#[test]
fn a_battery_that_is_moving_gets_a_time_line() {
    assert_eq!(
        remaining_text(State::Charging, Some(80)).as_deref(),
        Some("to full in 1h 20m")
    );
    assert_eq!(
        remaining_text(State::Discharging, Some(95)).as_deref(),
        Some("to empty in 1h 35m")
    );
}

/// And no line where there is nothing to count. `Plugged` is the one that made
/// this worth pinning: it is external power with the battery held back, so the
/// battery is not draining and "to empty in" would describe a machine that is not
/// this one. It only became reachable once the adapter's `online` was read, so
/// the wrong line would have shipped with the fix.
#[test]
fn nothing_is_counted_when_nothing_is_moving() {
    assert_eq!(remaining_text(State::Plugged, Some(95)), None);
    assert_eq!(remaining_text(State::Full, Some(95)), None);
    assert_eq!(remaining_text(State::Unknown, Some(95)), None);
    assert_eq!(remaining_text(State::Charging, Some(0)), None);
    assert_eq!(remaining_text(State::Discharging, None), None);
}
