use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use serde::Deserialize;

use super::Workspace;

const MAGIC: &[u8; 6] = b"i3-ipc";
const RUN_COMMAND: u32 = 0;
const GET_WORKSPACES: u32 = 1;
const SUBSCRIBE: u32 = 2;
/// i3 IPC marks event frames by setting the high bit of the message type.
const EVENT_BIT: u32 = 0x8000_0000;

#[derive(Deserialize)]
struct SwayWorkspace {
    #[serde(default)]
    num: Option<i32>,
    name: String,
    focused: bool,
    urgent: bool,
}

pub fn run(tx: async_channel::Sender<Vec<Workspace>>) {
    let Some(path) = std::env::var_os("SWAYSOCK") else {
        eprintln!("statusbar: SWAYSOCK is not set, workspace list disabled");
        return;
    };
    let mut stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("statusbar: cannot connect to sway IPC socket: {err}");
            return;
        }
    };
    if let Err(err) = send(&mut stream, SUBSCRIBE, br#"["workspace","output","mode"]"#) {
        eprintln!("statusbar: sway subscribe failed: {err}");
        return;
    }
    if let Err(err) = reply(&mut stream) {
        eprintln!("statusbar: sway subscribe reply failed: {err}");
        return;
    }

    // The state up front, then one round trip per event.
    match workspaces(&mut stream) {
        Ok(state) => {
            if tx.send_blocking(state).is_err() {
                return;
            }
        }
        Err(err) => eprintln!("statusbar: sway workspace query failed: {err}"),
    }
    loop {
        if let Err(err) = next_event(&mut stream) {
            eprintln!("statusbar: sway event stream closed: {err}");
            return;
        }
        match workspaces(&mut stream) {
            Ok(state) => {
                if tx.send_blocking(state).is_err() {
                    return;
                }
            }
            Err(err) => {
                eprintln!("statusbar: sway workspace query failed: {err}");
                return;
            }
        }
    }
}

pub fn focus(key: &str) {
    let Some(path) = std::env::var_os("SWAYSOCK") else {
        return;
    };
    let Ok(mut stream) = UnixStream::connect(path) else {
        return;
    };
    let payload = format!("workspace {key}");
    if send(&mut stream, RUN_COMMAND, payload.as_bytes()).is_err() {
        return;
    }
    let _ = reply(&mut stream);
}

fn workspaces(stream: &mut UnixStream) -> std::io::Result<Vec<Workspace>> {
    send(stream, GET_WORKSPACES, b"")?;
    let (_, payload) = reply(stream)?;
    parse(&payload)
}

/// Turn a `GET_WORKSPACES` payload into the bar's own list.
///
/// A workspace with no number is named rather than numbered (`num` is `-1`),
/// so it sorts after the numbered ones and is focused by name.
fn parse(payload: &[u8]) -> std::io::Result<Vec<Workspace>> {
    let mut list: Vec<SwayWorkspace> = serde_json::from_slice(payload)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    list.sort_by_key(|workspace| {
        workspace
            .num
            .filter(|num| *num >= 0)
            .unwrap_or(i32::MAX)
    });
    Ok(list.into_iter().map(to_workspace).collect())
}

fn to_workspace(workspace: SwayWorkspace) -> Workspace {
    let key = match workspace.num {
        Some(num) if num >= 0 => num.to_string(),
        _ => workspace.name,
    };
    Workspace {
        label: key.clone(),
        key,
        focused: workspace.focused,
        urgent: workspace.urgent,
    }
}

/// Read the next frame that answers one of our own requests.
///
/// sway multiplexes replies and subscribed events on a single socket, so a
/// frame read at the wrong moment desynchronises the stream — a reply parsed as
/// a workspace list fails, and the workspace list would stop updating. Events
/// seen while a reply is expected are dropped; the reply that follows them
/// already reflects them.
fn reply(stream: &mut UnixStream) -> std::io::Result<(u32, Vec<u8>)> {
    loop {
        let frame = recv(stream)?;
        if frame.0 & EVENT_BIT == 0 {
            return Ok(frame);
        }
    }
}

/// Wait for the next subscribed event, ignoring a reply still in flight.
fn next_event(stream: &mut UnixStream) -> std::io::Result<()> {
    loop {
        if recv(stream)?.0 & EVENT_BIT != 0 {
            return Ok(());
        }
    }
}

fn send(stream: &mut UnixStream, message_type: u32, payload: &[u8]) -> std::io::Result<()> {
    let mut header = [0u8; 14];
    header[..6].copy_from_slice(MAGIC);
    header[6..10].copy_from_slice(&(payload.len() as u32).to_ne_bytes());
    header[10..14].copy_from_slice(&message_type.to_ne_bytes());
    stream.write_all(&header)?;
    stream.write_all(payload)
}

fn recv(stream: &mut UnixStream) -> std::io::Result<(u32, Vec<u8>)> {
    let mut header = [0u8; 14];
    stream.read_exact(&mut header)?;
    if &header[..6] != MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad i3 IPC frame header",
        ));
    }
    let length = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload)?;
    let message_type = u32::from_ne_bytes(header[10..14].try_into().unwrap());
    Ok((message_type, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let labels: Vec<&str> = list.iter().map(|workspace| workspace.label.as_str()).collect();
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
            compositor.write_all(&frame(GET_WORKSPACES, WORKSPACES)).unwrap();
        });

        let (message_type, payload) = reply(&mut bar).expect("reply");
        assert_eq!(message_type, GET_WORKSPACES);
        assert_eq!(parse(&payload).expect("workspace list").len(), 3);
    }

    #[test]
    fn an_event_is_read_past_the_reply_in_flight() {
        let (mut bar, mut compositor) = UnixStream::pair().expect("socket pair");
        std::thread::spawn(move || {
            compositor.write_all(&frame(GET_WORKSPACES, WORKSPACES)).unwrap();
            compositor
                .write_all(&frame(2 | EVENT_BIT, br#"{"change":"focus"}"#))
                .unwrap();
        });

        assert!(next_event(&mut bar).is_ok());
    }
}
