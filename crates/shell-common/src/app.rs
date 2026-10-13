//! Starting a widget's `GApplication` without handing it our command line.

use gtk4::glib;
use gtk4::prelude::*;

/// Run `application` with nothing but the program name in its argv.
///
/// `Application::run` forwards the whole process command line to
/// `g_application_run`, and GApplication reads a bare word as a *file to open*.
/// A widget that does not handle opening files aborts before `activate` is
/// reached - `GLib-GIO-CRITICAL: This application can not open files`, exit
/// code 1 - so `sidebar toggle` never built a panel at all.
///
/// Every widget parses its own arguments before this point, so the application
/// is given the program name and nothing else.
pub fn run(application: &gtk4::Application, program: &str) -> glib::ExitCode {
    application.run_with_args(&[program])
}
