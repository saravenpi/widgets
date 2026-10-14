//! The `Exec` line checks: field codes, quoting, and a real detached spawn.

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
