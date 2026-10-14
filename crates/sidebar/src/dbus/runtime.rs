//! The daemon's session-bus runtime: owning the connection to
//! `org.freedesktop.Notifications`, and reporting closes back to clients.
//!
//! The interface itself is [`super`]'s.

use zbus::object_server::SignalEmitter;

use super::{Notifications, NotificationsSignals, EMITTER};

/// Own the name on the session bus on its own thread and hand back a handle the
/// widget layers can spawn on. The daemon ends with the process.
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

/// Tell every client on the bus that a notification is gone. `reason` is the
/// spec's: 1 expired, 2 dismissed, 3 closed by request.
pub fn emit_closed(handle: &tokio::runtime::Handle, id: u32, reason: u32) {
    handle.spawn(async move {
        if let Some(emitter) = EMITTER.get() {
            let _ = emitter.notification_closed(id, reason).await;
        }
    });
}

/// Claim the notifications name and serve it forever.
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
