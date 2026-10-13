use std::io::BufRead;
use std::process::{Command, Stdio};

use serde::Deserialize;

use super::Workspace;

#[derive(Deserialize)]
struct NiriWorkspace {
    id: u64,
    idx: u8,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    is_urgent: bool,
    #[serde(default)]
    is_focused: bool,
}

#[derive(Deserialize)]
enum Event {
    WorkspacesChanged {
        workspaces: Vec<NiriWorkspace>,
    },
    WorkspaceUrgencyChanged {
        id: u64,
        urgent: bool,
    },
    WorkspaceActivated {
        id: u64,
        focused: bool,
    },
}

pub fn run(tx: async_channel::Sender<Vec<Workspace>>) {
    let Some(mut state) = workspaces() else {
        eprintln!("statusbar: initial niri workspace query failed");
        return;
    };
    if tx.send_blocking(project(&state)).is_err() {
        return;
    }
    let child = Command::new("niri")
        .args(["msg", "--json", "event-stream"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        eprintln!("statusbar: cannot start niri msg event-stream");
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        return;
    };
    for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
        let Ok(event) = serde_json::from_str::<Event>(&line) else {
            continue;
        };
        if apply(&mut state, event) && tx.send_blocking(project(&state)).is_err() {
            return;
        }
    }
}

pub fn focus(key: &str) {
    let result = Command::new("niri")
        .args(["msg", "action", "focus-workspace", "--", key])
        .status();
    if let Err(err) = result {
        eprintln!("statusbar: niri focus-workspace failed: {err}");
    }
}

fn workspaces() -> Option<Vec<NiriWorkspace>> {
    let output = Command::new("niri")
        .args(["msg", "--json", "workspaces"])
        .output()
        .ok()?;
    serde_json::from_slice(&output.stdout).ok()
}

/// Fold one event into the tracked state, returning whether the bar has to be
/// redrawn. niri sends the whole workspace list on `WorkspacesChanged`, so the
/// two focused/urgent events are applied on top of it rather than re-queried.
fn apply(state: &mut Vec<NiriWorkspace>, event: Event) -> bool {
    match event {
        Event::WorkspacesChanged { workspaces } => {
            *state = workspaces;
            true
        }
        // Only the workspace being entered is a redraw; the one being left
        // reports `focused: false` and is already in the state.
        Event::WorkspaceActivated { id, focused } => focused && set_focused(state, id),
        Event::WorkspaceUrgencyChanged { id, urgent } => set_urgent(state, id, urgent),
    }
}

fn set_focused(state: &mut [NiriWorkspace], id: u64) -> bool {
    let mut changed = false;
    for workspace in state.iter_mut() {
        let focused = workspace.id == id;
        changed |= workspace.is_focused != focused;
        workspace.is_focused = focused;
    }
    changed
}

fn set_urgent(state: &mut [NiriWorkspace], id: u64, urgent: bool) -> bool {
    let Some(workspace) = state.iter_mut().find(|workspace| workspace.id == id) else {
        return false;
    };
    let changed = workspace.is_urgent != urgent;
    workspace.is_urgent = urgent;
    changed
}

fn project(state: &[NiriWorkspace]) -> Vec<Workspace> {
    state
        .iter()
        .map(|workspace| Workspace {
            key: workspace.idx.to_string(),
            label: workspace
                .name
                .clone()
                .unwrap_or_else(|| workspace.idx.to_string()),
            focused: workspace.is_focused,
            urgent: workspace.is_urgent,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(id: u64, idx: u8, focused: bool) -> NiriWorkspace {
        NiriWorkspace {
            id,
            idx,
            name: None,
            is_urgent: false,
            is_focused: focused,
        }
    }

    fn event(line: &str) -> Event {
        serde_json::from_str(line).expect("niri event")
    }

    #[test]
    fn events_move_the_focus_and_the_urgency() {
        let mut state = vec![workspace(1, 1, true), workspace(2, 2, false)];

        assert!(apply(
            &mut state,
            event(r#"{"WorkspaceActivated":{"id":2,"focused":true}}"#),
        ));
        assert!(!state[0].is_focused);
        assert!(state[1].is_focused);

        // Leaving a workspace is not a redraw: its `focused: false` describes
        // the state the bar is already showing.
        assert!(!apply(
            &mut state,
            event(r#"{"WorkspaceActivated":{"id":1,"focused":false}}"#),
        ));
        assert!(state[1].is_focused);

        assert!(apply(
            &mut state,
            event(r#"{"WorkspaceUrgencyChanged":{"id":1,"urgent":true}}"#),
        ));
        assert!(state[0].is_urgent);
        assert!(!apply(
            &mut state,
            event(r#"{"WorkspaceUrgencyChanged":{"id":1,"urgent":true}}"#),
        ));

        // An urgency report for a workspace that is gone changes nothing.
        assert!(!apply(
            &mut state,
            event(r#"{"WorkspaceUrgencyChanged":{"id":99,"urgent":true}}"#),
        ));
    }

    #[test]
    fn a_full_state_replaces_what_we_tracked() {
        let mut state = vec![workspace(1, 1, true)];
        let full = event(
            r#"{"WorkspacesChanged":{"workspaces":[{"id":5,"idx":5,"name":null,"is_urgent":true,"is_focused":true}]}}"#,
        );
        assert!(apply(&mut state, full));
        assert_eq!(state.len(), 1);
        assert_eq!(state[0].id, 5);
        assert!(state[0].is_urgent && state[0].is_focused);
    }

    #[test]
    fn the_index_is_the_key_and_the_name_is_the_label() {
        let mut state = vec![workspace(7, 3, true)];
        state[0].name = Some("web".into());
        let projected = project(&state);
        assert_eq!(projected[0].key, "3");
        assert_eq!(projected[0].label, "web");
        assert!(projected[0].focused);

        // Without a name, the label falls back to the index.
        let projected = project(&[workspace(8, 4, false)]);
        assert_eq!(projected[0].label, "4");
    }
}
