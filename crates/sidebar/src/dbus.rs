use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use crate::model::{self, Notification};

pub enum Event {
    Added(Notification),
    Removed(u32),
}

static EMITTER: OnceLock<SignalEmitter<'static>> = OnceLock::new();

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

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".to_string(), "persistence".to_string()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "widgets".to_string(),
            "saravenpi".to_string(),
            "1.2".to_string(),
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

pub fn start(next_id: u32) -> tokio::runtime::Handle {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("sidebar: tokio runtime");
    let handle = runtime.handle().clone();
    std::thread::spawn(move || {
        runtime.block_on(async move {
            match serve(next_id).await {
                Ok(()) => std::future::pending::<()>().await,
                Err(err) => eprintln!("sidebar: notification daemon error: {err}"),
            }
        });
    });
    handle
}

pub fn emit_closed(handle: &tokio::runtime::Handle, id: u32, reason: u32) {
    handle.spawn(async move {
        if let Some(emitter) = EMITTER.get() {
            let _ = emitter.notification_closed(id, reason).await;
        }
    });
}

async fn serve(next_id: u32) -> zbus::Result<()> {
    let connection = zbus::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at(
            "/org/freedesktop/Notifications",
            Notifications::new(next_id),
        )?
        .build()
        .await?;
    let emitter = SignalEmitter::new(&connection, "/org/freedesktop/Notifications")?.to_owned();
    let _ = EMITTER.set(emitter);
    std::future::pending::<()>().await;
    Ok(())
}
