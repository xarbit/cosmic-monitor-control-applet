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
use crate::brightness::BrightnessCalculator;
#[cfg(feature = "brightness-sync-daemon")]
use crate::config::{Config, CONFIG_VERSION};
#[cfg(feature = "brightness-sync-daemon")]
use crate::app::APPID;
#[cfg(feature = "brightness-sync-daemon")]
use cosmic::cosmic_config::{Config as CosmicConfig, CosmicConfigEntry};

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

    /// MaxDisplayBrightness property
    #[zbus(property)]
    fn max_display_brightness(&self) -> zbus::Result<i32>;
}

#[cfg(feature = "brightness-sync-daemon")]
pub fn sub(display_manager: crate::monitor::DisplayManager) -> impl Stream<Item = AppMsg> {
    stream::channel(10, |mut output| async move {
        // Try to connect to D-Bus and subscribe to brightness changes
        match subscribe_to_brightness_changes(&mut output, display_manager).await {
            Ok(_) => info!("UI brightness sync subscription ended"),
            Err(e) => warn!("Failed to subscribe to COSMIC brightness changes for UI sync: {}", e),
        }
    })
}

#[cfg(feature = "brightness-sync-daemon")]
async fn subscribe_to_brightness_changes(
    output: &mut futures::channel::mpsc::Sender<AppMsg>,
    display_manager: crate::monitor::DisplayManager,
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

    // Get max brightness for calculating percentage
    let max_brightness = proxy.max_display_brightness().await?;
    debug!("Max COSMIC brightness: {}", max_brightness);

    // Load config for per-monitor gamma/min brightness
    let config_handler = CosmicConfig::new(APPID, CONFIG_VERSION)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    // Debounce to avoid excessive refreshes
    let debounce_duration = tokio::time::Duration::from_millis(50);

    while let Some(change) = brightness_changed.next().await {
        if let Ok(mut brightness) = change.get().await {
            debug!("COSMIC brightness changed (F1/F2 keys), debouncing...");

            // Wait briefly and drain any rapid changes
            tokio::time::sleep(debounce_duration).await;
            loop {
                match tokio::time::timeout(
                    tokio::time::Duration::from_millis(5),
                    brightness_changed.next()
                ).await {
                    Ok(Some(newer_change)) => {
                        if let Ok(newer_brightness) = newer_change.get().await {
                            debug!("Skipping intermediate brightness change");
                            brightness = newer_brightness;
                        }
                    }
                    _ => break,
                }
            }

            // Calculate brightness percentage (same as daemon does)
            let percentage = if max_brightness > 0 {
                ((brightness as f64 / max_brightness as f64) * 100.0) as u16
            } else {
                0
            };
            let percentage = percentage.min(100);

            debug!(
                percentage = %percentage,
                "COSMIC brightness changed, calculating UI slider values"
            );

            // Load current config
            let config = match Config::get_entry(&config_handler) {
                Ok(config) => config,
                Err((errs, config)) => {
                    warn!(
                        errors = ?errs,
                        "Errors loading config, using defaults"
                    );
                    config
                }
            };

            // Use BrightnessCalculator for consistent calculations
            let calculator = BrightnessCalculator::new(&config);

            // Get all display IDs from DisplayManager
            let display_ids = display_manager.get_all_ids().await;

            // Calculate brightness for each monitor and update UI
            for id in display_ids {
                if !calculator.is_sync_enabled(&id) {
                    debug!(
                        display_id = %id,
                        "Skipping UI update (sync disabled)"
                    );
                    continue;
                }

                // Calculate brightness using shared calculator
                let gamma_corrected = calculator.calculate_for_display(percentage, &id);

                debug!(
                    display_id = %id,
                    brightness = %gamma_corrected,
                    "Updating UI slider"
                );

                // Send calculated brightness to UI (no DDC read needed!)
                if output.send(AppMsg::BrightnessWasUpdated(id, gamma_corrected)).await.is_err() {
                    break;
                }
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
