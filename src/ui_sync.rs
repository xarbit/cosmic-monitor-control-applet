// SPDX-License-Identifier: GPL-3.0-only
//! UI synchronization for F1/F2 brightness changes
//!
//! This module subscribes to COSMIC's DisplayBrightness changes to update
//! the UI sliders when F1/F2 keys are pressed. The actual brightness application
//! is handled by the daemon - this only refreshes the UI to reflect current values.

#[cfg(feature = "brightness-sync-daemon")]
use cosmic::iced::futures::{SinkExt, Stream};
#[cfg(feature = "brightness-sync-daemon")]
use cosmic::iced::stream;
#[cfg(feature = "brightness-sync-daemon")]
use zbus::{proxy, Connection};

#[cfg(feature = "brightness-sync-daemon")]
use crate::app::AppMsg;

#[cfg(feature = "brightness-sync-daemon")]
/// COSMIC Settings Daemon D-Bus proxy
#[proxy(
    interface = "com.system76.CosmicSettingsDaemon",
    default_service = "com.system76.CosmicSettingsDaemon",
    default_path = "/com/system76/CosmicSettingsDaemon"
)]
trait CosmicSettingsDaemon {
    /// DisplayBrightness property
    #[zbus(property)]
    fn display_brightness(&self) -> zbus::Result<i32>;
}

#[cfg(feature = "brightness-sync-daemon")]
pub fn sub() -> impl Stream<Item = AppMsg> {
    stream::channel(10, |mut output| async move {
        // Try to connect to D-Bus and subscribe to brightness changes
        match subscribe_to_brightness_changes(&mut output).await {
            Ok(_) => info!("UI brightness sync subscription ended"),
            Err(e) => warn!("Failed to subscribe to COSMIC brightness changes for UI sync: {}", e),
        }
    })
}

#[cfg(feature = "brightness-sync-daemon")]
async fn subscribe_to_brightness_changes(
    output: &mut futures::channel::mpsc::Sender<AppMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to session bus
    let connection = Connection::session().await?;

    // Create proxy to COSMIC Settings Daemon
    let proxy = CosmicSettingsDaemonProxy::new(&connection).await?;

    debug!("Connected to COSMIC Settings Daemon for UI brightness sync");

    // Subscribe to DisplayBrightness property changes
    use futures::StreamExt;
    let mut brightness_changed = proxy.receive_display_brightness_changed().await;

    debug!("Listening for COSMIC brightness-key changes to update UI sliders...");

    // Debounce to avoid excessive refreshes
    let debounce_duration = tokio::time::Duration::from_millis(50);

    while let Some(change) = brightness_changed.next().await {
        if let Ok(_brightness) = change.get().await {
            debug!("COSMIC brightness changed (F1/F2 keys), debouncing...");

            // Wait briefly and drain any rapid changes
            tokio::time::sleep(debounce_duration).await;
            loop {
                match tokio::time::timeout(
                    tokio::time::Duration::from_millis(5),
                    brightness_changed.next()
                ).await {
                    Ok(Some(newer_change)) => {
                        if let Ok(_) = newer_change.get().await {
                            debug!("Skipping intermediate brightness change");
                        }
                    }
                    _ => break,
                }
            }

            debug!("Waiting for daemon to finish setting brightness...");

            // Wait for daemon to complete brightness change (daemon waits 200ms + DDC time ~125ms = ~325ms)
            // Add extra buffer for safety
            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

            debug!("Refreshing UI with current brightness values");

            // Now safe to refresh UI from hardware
            if output.send(AppMsg::Refresh).await.is_err() {
                break;
            }
        }
    }

    Ok(())
}

/// No-op when feature is disabled
#[cfg(not(feature = "brightness-sync-daemon"))]
pub fn sub() -> impl Stream<Item = crate::app::AppMsg> {
    cosmic::iced::stream::channel(1, |_| async move {
        // Empty stream - do nothing
        futures::future::pending::<()>().await;
    })
}
