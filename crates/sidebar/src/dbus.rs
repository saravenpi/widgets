//! `sidebar`'s notification daemon: the `org.freedesktop.Notifications`
//! interface the panel answers on.
//!
//! This file is the interface itself — its methods, its signal, and the
//! translation between what a client sends and the [`Event`] the panel folds
//! in. The connection and its runtime live in [`runtime`].

mod runtime;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use crate::model::{self, Notification};

pub use runtime::{emit_closed, start};

/// One change to the panel's list.
pub enum Event {
    Added(Notification),
    Removed(u32),
}

/// The interface's own signal emitter, captured once the connection is up so
/// the widget layers can report a close without holding the connection.
static EMITTER: OnceLock<SignalEmitter<'static>> = OnceLock::new();

/// The interface, plus the id counter that names each notification. A client
/// that supplies `replaces_id` keeps its own id, which is why the counter is
/// bumped past it rather than reused.
pub struct Notifications {
    next_id: AtomicU32,
}

impl Notifications {
    fn new(next_id: u32) -> Self {
        Self {
            next_id: AtomicU32::new(next_id),
        }
    }
}

#[zbus::interface(name = "org.freedesktop.Notifications", spawn = false)]
impl Notifications {
    /// The freedesktop `Notify` method. Its eight arguments are the spec's, not
    /// the panel's: they are what `notify-send` and every other client send, so
    /// the signature is fixed and the ones the panel cannot render are ignored
    /// on purpose.
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        _expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            self.next_id
                .fetch_max(replaces_id.saturating_add(1), Ordering::SeqCst);
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::SeqCst)
        };
        let urgent = hints
            .get("urgency")
            .and_then(|value| u8::try_from(value).ok())
            .map(|urgency| urgency >= 3)
            .unwrap_or(false);
        let notification = Notification {
            id,
            app_name,
            summary,
            body,
            ts: model::now(),
            urgent,
        };
        crate::deliver(Event::Added(notification));
        id
    }

    async fn close_notification(
        &self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        emitter.notification_closed(id, 3).await?;
        crate::deliver(Event::Removed(id));
        Ok(())
    }

    /// What clients are allowed to send. `body` is the whole of it: the panel
    /// renders text as it arrives, so markup, images, sounds and actions are
    /// deliberately absent and clients fall back to plain text.
    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".to_string()]
    }

    /// (name, vendor, version, specification version).
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "widgets".to_string(),
            "saravenpi".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        )
    }

    #[zbus(signal)]
    async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
}
