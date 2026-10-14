//! The niri event checks, beside the module they exercise.

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
