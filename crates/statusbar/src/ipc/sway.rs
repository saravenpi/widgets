use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use serde::Deserialize;

use super::Workspace;

const MAGIC: &[u8; 6] = b"i3-ipc";
const RUN_COMMAND: u32 = 0;
const GET_WORKSPACES: u32 = 1;
const SUBSCRIBE: u32 = 2;

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
    if let Err(err) = recv(&mut stream) {
        eprintln!("statusbar: sway subscribe reply failed: {err}");
        return;
    }
    loop {
        match refresh(&mut stream) {
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
        if let Err(err) = recv(&mut stream) {
            eprintln!("statusbar: sway event stream closed: {err}");
            return;
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
    let _ = recv(&mut stream);
}

fn refresh(stream: &mut UnixStream) -> std::io::Result<Vec<Workspace>> {
    send(stream, GET_WORKSPACES, b"")?;
    let (_, payload) = recv(stream)?;
    let mut list: Vec<SwayWorkspace> = serde_json::from_slice(&payload)
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
