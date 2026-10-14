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
    // The state up front, then one re-query per relevant event.
    if !publish(&tx, false) {
        return;
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
        if relevant(&line) && !publish(&tx, true) {
            return;
        }
    }
}

/// Query the workspaces and hand them to the bar.
///
/// `false` means the reader should stop: the bar is gone, or — once the stream is
/// running (`fatal_on_error`) — the query itself failed, and a list that can no
/// longer be trusted is worse than a reader that stops. The first query is only
/// reported and carried on from, because the event socket is what keeps the list
/// alive.
fn publish(tx: &async_channel::Sender<Vec<Workspace>>, fatal_on_error: bool) -> bool {
    match state() {
        Some(workspaces) => tx.send_blocking(workspaces).is_ok(),
        None => {
            eprintln!("statusbar: hyprctl workspace query failed");
            !fatal_on_error
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

/// Whether an event line can change which workspaces exist, or which one is
/// focused.
///
/// Hyprland writes event names in lower case on `.socket2.sock`
/// (`workspace>>2`), and the list below is the workspace-related subset of the
/// event table at https://wiki.hypr.land/IPC/ (checked against v0.41.0). The
/// comparison lowercases the name rather than trusting one spelling.
///
/// Window events are deliberately absent: the bar only draws workspace names,
/// focus and urgency, so a window opening or closing is no reason to go back to
/// `hyprctl`.
fn relevant(line: &str) -> bool {
    let event = line.split(">>").next().unwrap_or("").to_ascii_lowercase();
    matches!(
        event.as_str(),
        "workspace"
            | "workspacev2"
            | "createworkspace"
            | "createworkspacev2"
            | "destroyworkspace"
            | "destroyworkspacev2"
            | "moveworkspace"
            | "moveworkspacev2"
            | "renameworkspace"
            | "activespecial"
            | "focusedmon"
            | "monitoradded"
            | "monitoraddedv2"
            | "monitorremoved"
            | "urgent"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_workspace_events_trigger_a_requery() {
        // Exactly the names Hyprland sends on `.socket2.sock`: lower case.
        for line in [
            "workspace>>3",
            "workspacev2>>3,name",
            "createworkspacev2>>4,4",
            "destroyworkspace>>2",
            "moveworkspace>>2,DP-1",
            "renameworkspace>>2,web",
            "activespecial>>special,DP-1",
            "focusedmon>>DP-1,2",
            "monitorremoved>>DP-2",
            "urgent>>abc123",
        ] {
            assert!(relevant(line), "{line} should trigger a refresh");
        }

        for line in [
            "openwindow>>abc,1,kitty,title",
            "closewindow>>abc",
            "activewindow>>kitty,title",
            "changefloatingmode>>abc,1",
            "",
        ] {
            assert!(!relevant(line), "{line} should be ignored");
        }
    }
}
