use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

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
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn()?;
    Ok(())
}

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

fn split_exec(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut chars = value.chars();
    while let Some(current_char) = chars.next() {
        match current_char {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                    started = true;
                }
            }
            ' ' | '\t' | '\n' | '\r' if !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                current.push(current_char);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_codes_are_stripped() {
        assert_eq!(strip_field_codes("kitty %u"), "kitty ");
        assert_eq!(strip_field_codes("foo%%bar"), "foobar");
        assert_eq!(strip_field_codes("%f %F %u %U %i %c %k %%"), "       ");
    }

    #[test]
    fn quotes_and_escapes_are_respected() {
        assert_eq!(
            split_exec("sh -c \"echo hi > /tmp/out\""),
            vec!["sh", "-c", "echo hi > /tmp/out"]
        );
        assert_eq!(split_exec("env X=\"a b\" prog"), vec!["env", "X=a b", "prog"]);
        assert_eq!(split_exec("prog a\\ b"), vec!["prog", "a b"]);
        assert_eq!(split_exec("kitty"), vec!["kitty"]);
        assert!(split_exec("").is_empty());
    }

    #[test]
    fn launch_spawns_detached() {
        assert!(launch("/bin/echo ok").is_ok());
        assert!(launch("").is_err());
    }
}
