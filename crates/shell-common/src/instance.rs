//! One copy of a widget, at most.
//!
//! Widgets meant to live for a whole session — the statusbar, the sidebar
//! daemon — claim a pid file under `$XDG_RUNTIME_DIR` before they do any work.
//! A second copy, started from a compositor config or a key binding, finds the
//! file and steps aside instead of adding a second bar to the screen.
//!
//! A pid alone is not enough to trust the file: pids are recycled, and a stale
//! entry pointing at whatever inherited the number would block the widget, or
//! worse, make it signal an unrelated process. [`running`] therefore also
//! checks `/proc/<pid>/comm`, and reads a file whose process is gone — or is no
//! longer a copy of the widget — as stale and takes it over.

use std::path::PathBuf;

/// The pid file a widget claims, e.g. `$XDG_RUNTIME_DIR/widgets-statusbar.pid`.
pub fn pidfile(name: &str) -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!("widgets-{name}.pid"))
}

/// The pid of the running copy of `name`, if the pid file names a live one.
///
/// This process never counts as a copy of itself, so [`claim`] can call it.
pub fn running(name: &str) -> Option<i32> {
    let pid = std::fs::read_to_string(pidfile(name))
        .ok()?
        .trim()
        .parse::<i32>()
        .ok()?;
    let ours = std::process::id() as i32;
    (pid > 0 && pid != ours && is_widget(name, pid)).then_some(pid)
}

/// Take ownership of `name`, writing this process's pid.
///
/// Returns the pid of the copy that is already running, and leaves the file
/// untouched in that case; otherwise returns `None` and this process owns it
/// until [`release`].
pub fn claim(name: &str) -> Option<i32> {
    if let Some(pid) = running(name) {
        return Some(pid);
    }
    let path = pidfile(name);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, std::process::id().to_string());
    None
}

/// Remove the pid file, but only while it still describes this process.
pub fn release(name: &str) {
    let ours = std::process::id().to_string();
    if std::fs::read_to_string(pidfile(name)).ok().as_deref() == Some(ours.as_str()) {
        let _ = std::fs::remove_file(pidfile(name));
    }
}

/// Whether `pid` is currently a process called `name`.
///
/// On a system without `/proc` this is always false, which errs towards letting
/// the widget start rather than locking the user out of it.
fn is_widget(name: &str, pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|comm| comm.trim() == name)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pid file path is derived from the environment on every call, so this
    /// test owns `XDG_RUNTIME_DIR` for its duration. Nothing else reads it.
    #[test]
    fn a_widget_owns_its_pid_file() {
        let dir = std::env::temp_dir().join("widgets-instance-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp runtime dir");
        std::env::set_var("XDG_RUNTIME_DIR", &dir);

        // A free name is claimed, and our own entry never reads back as a copy.
        assert_eq!(claim("statusbar"), None);
        assert_eq!(
            std::fs::read_to_string(pidfile("statusbar")).expect("pid file"),
            std::process::id().to_string()
        );
        assert_eq!(running("statusbar"), None);

        // Releasing removes it; a second release is a no-op.
        release("statusbar");
        assert!(!pidfile("statusbar").exists());
        release("statusbar");

        // A pid file pointing at a process that is gone is stale, and the
        // widget takes it over instead of refusing to start.
        std::fs::write(pidfile("statusbar"), "2147483646").expect("stale pid file");
        assert_eq!(running("statusbar"), None);
        assert_eq!(claim("statusbar"), None);

        // So is a pid file pointing at a live process that is not the widget.
        std::fs::write(pidfile("statusbar"), "1").expect("foreign pid file");
        assert_eq!(running("statusbar"), None);

        release("statusbar");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
