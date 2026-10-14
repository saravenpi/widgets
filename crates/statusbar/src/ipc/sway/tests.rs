//! The sway frame-reader and workspace-parsing checks, beside the module they
//! exercise.

use std::io::Write;
use std::os::unix::net::UnixStream;

use super::parse::parse;
use super::wire;
use super::wire::{EVENT_BIT, GET_WORKSPACES, MAGIC, SUBSCRIBE};

fn frame(message_type: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(payload.len() as u32).to_ne_bytes());
    bytes.extend_from_slice(&message_type.to_ne_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

const WORKSPACES: &[u8] =
    br#"[{"num":2,"name":"2","focused":false,"urgent":false},
             {"num":-1,"name":"mail","focused":false,"urgent":true},
             {"num":1,"name":"1","focused":true,"urgent":false}]"#;

#[test]
fn named_workspaces_sort_last_and_carry_focus_and_urgency() {
    let list = parse(WORKSPACES).expect("workspace list");
    let labels: Vec<&str> = list
        .iter()
        .map(|workspace| workspace.label.as_str())
        .collect();
    assert_eq!(labels, ["1", "2", "mail"]);
    assert!(list[0].focused);
    assert!(list[2].urgent);
    assert_eq!(list[2].key, "mail");
}

#[test]
fn a_reply_is_read_past_the_events_in_flight() {
    let (mut bar, mut compositor) = UnixStream::pair().expect("socket pair");
    std::thread::spawn(move || {
        compositor
            .write_all(&frame(SUBSCRIBE | EVENT_BIT, br#"{"change":"focus"}"#))
            .unwrap();
        compositor
            .write_all(&frame(GET_WORKSPACES, WORKSPACES))
            .unwrap();
    });

    let (message_type, payload) = wire::reply(&mut bar).expect("reply");
    assert_eq!(message_type, GET_WORKSPACES);
    assert_eq!(parse(&payload).expect("workspace list").len(), 3);
}

#[test]
fn an_event_is_read_past_the_reply_in_flight() {
    let (mut bar, mut compositor) = UnixStream::pair().expect("socket pair");
    std::thread::spawn(move || {
        compositor
            .write_all(&frame(GET_WORKSPACES, WORKSPACES))
            .unwrap();
        compositor
            .write_all(&frame(2 | EVENT_BIT, br#"{"change":"focus"}"#))
            .unwrap();
    });

    assert!(wire::next_event(&mut bar).is_ok());
}
