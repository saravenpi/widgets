//! The i3 IPC wire format as sway speaks it: a fixed 14-byte header, then a
//! payload. Nothing here knows what a workspace is.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub(super) const MAGIC: &[u8; 6] = b"i3-ipc";
pub(super) const RUN_COMMAND: u32 = 0;
pub(super) const GET_WORKSPACES: u32 = 1;
pub(super) const SUBSCRIBE: u32 = 2;
/// i3 IPC marks event frames by setting the high bit of the message type.
pub(super) const EVENT_BIT: u32 = 0x8000_0000;

pub(super) fn send(
    stream: &mut UnixStream,
    message_type: u32,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut header = [0u8; 14];
    header[..6].copy_from_slice(MAGIC);
    header[6..10].copy_from_slice(&(payload.len() as u32).to_ne_bytes());
    header[10..14].copy_from_slice(&message_type.to_ne_bytes());
    stream.write_all(&header)?;
    stream.write_all(payload)
}

pub(super) fn recv(stream: &mut UnixStream) -> std::io::Result<(u32, Vec<u8>)> {
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

/// Read the next frame that answers one of our own requests.
///
/// sway multiplexes replies and subscribed events on a single socket, so a
/// frame read at the wrong moment desynchronises the stream — a reply parsed as
/// a workspace list fails, and the workspace list would stop updating. Events
/// seen while a reply is expected are dropped; the reply that follows them
/// already reflects them.
pub(super) fn reply(stream: &mut UnixStream) -> std::io::Result<(u32, Vec<u8>)> {
    loop {
        let frame = recv(stream)?;
        if frame.0 & EVENT_BIT == 0 {
            return Ok(frame);
        }
    }
}

/// Wait for the next subscribed event, ignoring a reply still in flight.
pub(super) fn next_event(stream: &mut UnixStream) -> std::io::Result<()> {
    loop {
        if recv(stream)?.0 & EVENT_BIT != 0 {
            return Ok(());
        }
    }
}
