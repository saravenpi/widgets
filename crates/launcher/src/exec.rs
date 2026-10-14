//! Running what an `Exec` line names.
//!
//! An `Exec` value is not a shell line: the desktop entry specification defines
//! its own quoting, so this honours double quotes and backslash escapes and
//! nothing else. Field codes (`%u`, `%f`, `%i` and friends) describe arguments
//! the launcher has no way to supply, and are stripped.

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

/// Start the program an `Exec` value names, detached from the launcher.
pub fn launch(exec: &str) -> io::Result<()> {
    let parts = split_exec(&strip_field_codes(exec));
    let (program, args) = parts
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty Exec line"))?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    command.spawn()?;
    Ok(())
}

/// Put the child in its own session, so closing the launcher does not take the
/// application down with it. The hook runs between fork and exec, where only
/// async-signal-safe calls belong, and `setsid` is one.
fn detach(command: &mut Command) {
    unsafe {
        command.pre_exec(|| match libc::setsid() {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
}

/// Drop the field codes: `%` plus one character, the escape for a literal `%`
/// included.
fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars();
    while let Some(current) = chars.next() {
        if current == '%' {
            chars.next();
            continue;
        }
        out.push(current);
    }
    out
}

/// Split an `Exec` value into the program and its arguments. A backslash
/// escapes the character after it; one at the end of the value is dropped.
fn split_exec(value: &str) -> Vec<String> {
    let mut split = Splitter::default();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(escaped) = chars.next() {
                split.plain(escaped);
            }
        } else {
            split.push(character);
        }
    }
    split.finish()
}

/// The arguments collected so far, and the token being built.
#[derive(Default)]
struct Splitter {
    args: Vec<String>,
    token: String,
    started: bool,
    quoted: bool,
}

impl Splitter {
    /// Take one character, deciding whether it continues the token, ends it, or
    /// only toggles the quoting. Whitespace inside quotes is just a character.
    fn push(&mut self, character: char) {
        match character {
            '"' => {
                self.quoted = !self.quoted;
                self.started = true;
            }
            ' ' | '\t' | '\n' | '\r' if !self.quoted => self.separate(),
            plain => self.plain(plain),
        }
    }

    /// Append `character` to the token under construction.
    fn plain(&mut self, character: char) {
        self.token.push(character);
        self.started = true;
    }

    /// Close the token. Only a token that was started is closed, so a run of
    /// separators collapses into one break.
    fn separate(&mut self) {
        if self.started {
            self.args.push(std::mem::take(&mut self.token));
            self.started = false;
        }
    }

    /// The arguments, with the last token closed.
    fn finish(mut self) -> Vec<String> {
        self.separate();
        self.args
    }
}

#[cfg(test)]
mod tests;
