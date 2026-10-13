use std::io::BufRead;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::Workspace;

#[derive(Deserialize)]
struct HyprWorkspace {
    id: i64,
    name: String,
    #[serde(default)]
    urgent: bool,
}

#[derive(Deserialize)]
struct ActiveWorkspace {
    id: i64,
}

pub fn run(tx: async_channel::Sender<Vec<Workspace>>) {
    match state() {
        Some(initial) => {
            if tx.send_blocking(initial).is_err() {
                return;
            }
        }
        None => eprintln!("statusbar: initial hyprctl workspace query failed"),
    }
    let Some(path) = event_socket() else {
        eprintln!("statusbar: hyprland event socket path incomplete, workspace list will not update");
        return;
    };
    let stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("statusbar: cannot connect to hyprland event socket: {err}");
            return;
        }
    };
    for line in std::io::BufReader::new(stream).lines().map_while(Result::ok) {
        if !relevant(&line) {
            continue;
        }
        match state() {
            Some(next) => {
                if tx.send_blocking(next).is_err() {
                    return;
                }
            }
            None => eprintln!("statusbar: hyprctl workspace query failed"),
        }
    }
}

pub fn focus(key: &str) {
    let result = Command::new("hyprctl")
        .args(["dispatch", "workspace", key])
        .status();
    if let Err(err) = result {
        eprintln!("statusbar: hyprctl dispatch workspace failed: {err}");
    }
}

fn state() -> Option<Vec<Workspace>> {
    let mut list = query::<Vec<HyprWorkspace>>(["-j", "workspaces"])?;
    let active = query::<ActiveWorkspace>(["-j", "activeworkspace"])?;
    list.sort_by_key(|workspace| workspace.name.parse::<i32>().unwrap_or(i32::MAX));
    Some(
        list.into_iter()
            .map(|workspace| Workspace {
                key: workspace.id.to_string(),
                label: workspace.name.clone(),
                focused: workspace.id == active.id,
                urgent: workspace.urgent,
            })
            .collect(),
    )
}

fn query<T: serde::de::DeserializeOwned>(args: [&str; 2]) -> Option<T> {
    let output = Command::new("hyprctl").args(args).output().ok()?;
    serde_json::from_slice(&output.stdout).ok()
}

fn event_socket() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(
        PathBuf::from(runtime)
            .join("hypr")
            .join(signature)
            .join(".socket2.sock"),
    )
}

fn relevant(line: &str) -> bool {
    let event = line.split(">>").next().unwrap_or("");
    matches!(
        event,
        "WORKSPACE"
            | "WORKSPACEV2"
            | "CREATEWORKSPACE"
            | "CREATEWORKSPACEV2"
            | "DESTROYWORKSPACE"
            | "DESTROYWORKSPACEV2"
    )
}
