//! The toast test surface.
//!
//! The panel's test drives the stack through these. They are separate functions
//! rather than methods on [`Popups`] so the type the widget ships carries no
//! test-only API, and being a child of `popup` they still reach its private
//! state.

use std::rc::Rc;

use super::{Popups, Toast};

/// How many toasts are on screen.
pub(crate) fn visible(popups: &Popups) -> usize {
    popups.toasts.borrow().len()
}

/// The window carrying the stack.
pub(crate) fn surface(popups: &Popups) -> &gtk4::ApplicationWindow {
    &popups.window
}

/// Push every toast past its deadline so the next `expire` drops it. Real
/// toasts wait out `TOAST_SECONDS` instead.
pub(crate) fn force_expiry(popups: &Popups) {
    for toast in popups.toasts.borrow_mut().iter_mut() {
        toast.deadline = 0;
    }
}

/// The ids currently on screen, oldest first. Used to prove the stack really
/// holds what the count says it does.
pub(crate) fn ids(popups: &Rc<Popups>) -> Vec<u32> {
    popups
        .toasts
        .borrow()
        .iter()
        .map(|toast: &Toast| toast.notification.id)
        .collect()
}
