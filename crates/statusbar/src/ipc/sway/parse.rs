//! Turning a sway workspace list into the bar's own.

use serde::Deserialize;

use crate::ipc::Workspace;

#[derive(Deserialize)]
struct SwayWorkspace {
    #[serde(default)]
    num: Option<i32>,
    name: String,
    focused: bool,
    urgent: bool,
}

/// Turn a `GET_WORKSPACES` payload into the bar's own list.
///
/// A workspace with no number is named rather than numbered (`num` is `-1`),
/// so it sorts after the numbered ones and is focused by name.
pub(super) fn parse(payload: &[u8]) -> std::io::Result<Vec<Workspace>> {
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
