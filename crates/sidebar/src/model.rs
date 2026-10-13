use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const MAX_NOTIFICATIONS: usize = 50;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub ts: i64,
    pub urgent: bool,
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub fn relative_time(ts: i64, now: i64) -> String {
    let age = (now - ts).max(0);
    if age < 60 {
        "just now".to_string()
    } else if age < 3_600 {
        format!("{}m", age / 60)
    } else if age < 86_400 {
        format!("{}h", age / 3_600)
    } else {
        format!("{}d", age / 86_400)
    }
}

fn state_file() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_STATE_HOME") {
        let mut path = PathBuf::from(dir);
        path.push("widgets/notifications.json");
        return path;
    }
    let mut path = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    path.push(".local/state/widgets/notifications.json");
    path
}

pub fn load() -> Vec<Notification> {
    std::fs::read_to_string(state_file())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(items: &[Notification]) {
    let path = state_file();
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(items) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

pub fn insert(items: &mut Vec<Notification>, item: Notification) {
    items.retain(|existing| existing.id != item.id);
    items.insert(0, item);
    items.truncate(MAX_NOTIFICATIONS);
}
