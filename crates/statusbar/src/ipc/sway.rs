//! Following **sway**'s workspaces over its i3 IPC socket.
//!
//! The socket multiplexes replies and subscribed events, so the framing and the
//! reply-versus-event classification live in [`wire`], and the JSON in [`parse`].

mod parse;
mod wire;

#[cfg(test)]
mod tests;

use std::os::unix::net::UnixStream;

use super::Workspace;
use wire::{GET_WORKSPACES, RUN_COMMAND, SUBSCRIBE, next_event, reply, send};

const SUBSCRIBE_PAYLOAD: &[u8] = br#"["workspace","output","mode"]"#;

pub fn run(tx: async_channel::Sender<Vec<Workspace>>) {
    let Some(mut stream) = subscribe() else {
        return;
    };
    // The state up front, then one round trip per event.
    if !report(&mut stream, &tx, false) {
        return;
    }
    loop {
        if let Err(err) = next_event(&mut stream) {
            eprintln!("statusbar: sway event stream closed: {err}");
            return;
        }
        if !report(&mut stream, &tx, true) {
            return;
        }
    }
}

/// Connect to sway and subscribe, or say why not and hand back `None`.
fn subscribe() -> Option<UnixStream> {
    let Some(path) = std::env::var_os("SWAYSOCK") else {
        eprintln!("statusbar: SWAYSOCK is not set, workspace list disabled");
        return None;
    };
    let mut stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("statusbar: cannot connect to sway IPC socket: {err}");
            return None;
        }
    };
    if let Err(err) = send(&mut stream, SUBSCRIBE, SUBSCRIBE_PAYLOAD) {
        eprintln!("statusbar: sway subscribe failed: {err}");
        return None;
    }
    if let Err(err) = reply(&mut stream) {
        eprintln!("statusbar: sway subscribe reply failed: {err}");
        return None;
    }
    Some(stream)
}

/// Query the workspaces and hand them to the bar.
///
/// `false` means the reader should stop. A failed send always stops it, because
/// the bar is gone. A failed query stops it only once the stream is running
/// (`fatal_on_error`): the first query is worth reporting and carrying on from,
/// since the subscribe reply has already proved the socket works.
fn report(
    stream: &mut UnixStream,
    tx: &async_channel::Sender<Vec<Workspace>>,
    fatal_on_error: bool,
) -> bool {
    match workspaces(stream) {
        Ok(state) => tx.send_blocking(state).is_ok(),
        Err(err) => {
            eprintln!("statusbar: sway workspace query failed: {err}");
            !fatal_on_error
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
    parse::parse(&payload)
}
