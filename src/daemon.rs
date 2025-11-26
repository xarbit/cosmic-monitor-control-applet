// SPDX-License-Identifier: GPL-3.0-only
//! Brightness synchronization daemon
//!
//! This daemon listens to COSMIC's DisplayBrightness changes (F1/F2 keys) and
//! applies them to external displays based on per-monitor sync configuration.
//!
//! Supports:
//! - DDC/CI displays (standard monitors via I2C)
//! - Apple HID displays (Apple Studio Display, Pro Display XDR, LG UltraFine)
//!
//! Only activates when external displays are detected.

#[cfg(feature = "brightness-sync-daemon")]
use anyhow::{Context, Result};
#[cfg(feature = "brightness-sync-daemon")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "brightness-sync-daemon")]
use zbus::{proxy, Connection};
#[cfg(feature = "brightness-sync-daemon")]
use cosmic::cosmic_config::{Config as CosmicConfig, CosmicConfigEntry};

#[cfg(feature = "brightness-sync-daemon")]
use crate::protocols::DisplayProtocol;
#[cfg(feature = "brightness-sync-daemon")]
use crate::protocols::ddc_ci::DdcCiDisplay;
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use crate::protocols::apple_hid::AppleHidDisplay;
#[cfg(feature = "brightness-sync-daemon")]
use crate::config::{Config, CONFIG_VERSION};
#[cfg(feature = "brightness-sync-daemon")]
use crate::app::APPID;

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
pub struct BrightnessSyncDaemon {
    display_manager: crate::monitor::DisplayManager,
    config_handler: CosmicConfig,
    last_brightness: Arc<tokio::sync::Mutex<std::collections::HashMap<String, u16>>>,  // Track last brightness per display
}

#[cfg(feature = "brightness-sync-daemon")]
impl BrightnessSyncDaemon {
    /// Create a new brightness sync daemon
    /// Returns None if no external displays are detected after waiting
    pub async fn new(display_manager: crate::monitor::DisplayManager) -> Result<Option<Self>> {
        // Wait for DisplayManager to be populated by the subscription
        // The subscription enumerates displays asynchronously, so we need to wait
        tracing::info!("Waiting for display enumeration to complete...");

        let mut attempts = 0;
        let display_count = loop {
            let count = display_manager.count().await;
            if count > 0 {
                break count;
            }

            attempts += 1;
            if attempts >= 50 {  // 50 * 100ms = 5 seconds max wait
                tracing::info!("No external displays detected after waiting, brightness sync daemon disabled");
                return Ok(None);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        };

        tracing::info!("DisplayManager ready with {} display(s)", display_count);


        // Load configuration handler for runtime config access
        let config_handler = match CosmicConfig::new(APPID, CONFIG_VERSION) {
            Ok(handler) => {
                tracing::info!("Loaded config for per-monitor F1/F2 sync settings");
                handler
            }
            Err(err) => {
                tracing::warn!("Failed to load config: {}, monitors will default to sync enabled", err);
                CosmicConfig::new(APPID, CONFIG_VERSION).unwrap_or_else(|e| {
                    panic!("Cannot create config handler: {}", e);
                })
            }
        };

        tracing::info!(
            "Found {} external display(s) in DisplayManager, enabling brightness sync daemon with per-monitor control",
            display_count
        );

        Ok(Some(Self {
            display_manager,
            config_handler,
            last_brightness: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }))
    }

    pub async fn run(self) -> Result<()> {
        tracing::info!("Starting brightness sync daemon");

        // Connect to session bus
        let connection = Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;

        // Create proxy to COSMIC Settings Daemon
        let proxy = CosmicSettingsDaemonProxy::new(&connection)
            .await
            .context("Failed to create COSMIC Settings Daemon proxy")?;

        tracing::info!("Connected to COSMIC Settings Daemon");

        // Get max brightness for conversion
        let max_brightness = proxy
            .max_display_brightness()
            .await
            .context("Failed to get max display brightness")?;

        tracing::info!("Max display brightness: {}", max_brightness);

        // Subscribe to DisplayBrightness property changes
        use futures::StreamExt;
        let mut brightness_changed = proxy.receive_display_brightness_changed().await;

        tracing::info!("Listening for COSMIC brightness-key changes...");

        // Debounce rapid brightness changes to prevent overwhelming DDC/CI displays
        let debounce_duration = tokio::time::Duration::from_millis(50);

        while let Some(change) = brightness_changed.next().await {
            if let Ok(mut brightness) = change.get().await {
                tracing::debug!("COSMIC brightness changed to: {}", brightness);

                // Wait briefly and drain any rapid subsequent changes
                tokio::time::sleep(debounce_duration).await;

                // Drain any changes that arrived during the debounce period
                loop {
                    match tokio::time::timeout(
                        tokio::time::Duration::from_millis(5),
                        brightness_changed.next()
                    ).await {
                        Ok(Some(newer_change)) => {
                            if let Ok(newer_brightness) = newer_change.get().await {
                                tracing::debug!("Skipping intermediate brightness {}, using {}", brightness, newer_brightness);
                                brightness = newer_brightness;
                            }
                        }
                        _ => break, // Timeout or end of stream
                    }
                }

                // Convert COSMIC brightness (0-max) to percentage (0-100)
                let percentage = if max_brightness > 0 {
                    ((brightness as f64 / max_brightness as f64) * 100.0) as u16
                } else {
                    0
                };
                let percentage = percentage.min(100);

                tracing::debug!(
                    "Brightness change: {}% (COSMIC value: {}/{})",
                    percentage,
                    brightness,
                    max_brightness
                );

                // Apply brightness based on per-monitor sync configuration
                let config = match Config::get_entry(&self.config_handler) {
                    Ok(config) => config,
                    Err((errs, config)) => {
                        tracing::warn!("Errors loading config: {:?}, using defaults", errs);
                        config
                    }
                };

                let slider_value = percentage as f32 / 100.0;

                // Apply brightness to all displays in parallel
                let mut tasks = Vec::new();
                let mut synced_count = 0;
                let mut last_brightness_map = self.last_brightness.lock().await;

                // Get all display IDs from DisplayManager
                let display_ids = self.display_manager.get_all_ids().await;

                for id in display_ids {
                    if config.is_sync_enabled(&id) {
                        // Get display from DisplayManager
                        let display = match self.display_manager.get(&id).await {
                            Some(d) => d,
                            None => {
                                tracing::warn!("Display {} not found in DisplayManager", id);
                                continue;
                            }
                        };
                        // Apply gamma correction for this monitor
                        let gamma = config.get_gamma_map(&id);
                        let mut gamma_corrected = crate::app::get_mapped_brightness(slider_value, gamma);

                        // Apply minimum brightness clamp
                        let min_brightness = config.get_min_brightness(&id);
                        if gamma_corrected < min_brightness {
                            gamma_corrected = min_brightness;
                        }

                        // Check if brightness actually changed or if at min/max boundary
                        let last_value = last_brightness_map.get(&id).copied();

                        // Skip if brightness hasn't changed
                        if last_value == Some(gamma_corrected) {
                            // Log if we're at a boundary
                            if gamma_corrected == 0 {
                                tracing::info!("Display {} at minimum brightness (0%)", id);
                            } else if gamma_corrected == 100 {
                                tracing::info!("Display {} at maximum brightness (100%)", id);
                            } else {
                                tracing::debug!("Skipping display {} - brightness unchanged at {}%", id, gamma_corrected);
                            }
                            continue;
                        }

                        // Skip if we're at a boundary and trying to go further in the same direction
                        if let Some(last) = last_value {
                            if (gamma_corrected == 0 && last == 0 && gamma_corrected <= last) ||
                               (gamma_corrected == 100 && last == 100 && gamma_corrected >= last) {
                                if gamma_corrected == 0 {
                                    tracing::info!("Display {} at minimum brightness (0%)", id);
                                } else {
                                    tracing::info!("Display {} at maximum brightness (100%)", id);
                                }
                                continue;
                            }
                        }

                        // Update last brightness
                        last_brightness_map.insert(id.clone(), gamma_corrected);

                        tracing::debug!("Sending brightness command to {} ({}% -> {}%)",
                            id, last_value.unwrap_or(0), gamma_corrected);

                        // Clone what we need for the async task
                        let id_clone = id.clone();
                        let display_clone = display.clone();

                        // Spawn blocking task for each display to set brightness in parallel
                        let task = tokio::task::spawn_blocking(move || {
                            let start = std::time::Instant::now();
                            let mut display_guard = futures::executor::block_on(display_clone.lock());

                            // Retry once if first attempt fails
                            // DDC/CI protocol requires 40ms between commands, so we add 50ms delay before retry
                            match display_guard.set_brightness(gamma_corrected) {
                                Ok(_) => {
                                    let elapsed = start.elapsed();
                                    tracing::info!("Set {} to {}% in {:?}", id_clone, gamma_corrected, elapsed);
                                }
                                Err(e) => {
                                    tracing::debug!("Display {} first attempt failed: {}, waiting 50ms before retry", id_clone, e);
                                    // DDC/CI spec requires 40ms between commands, use 50ms to be safe
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                    match display_guard.set_brightness(gamma_corrected) {
                                        Ok(_) => {
                                            let elapsed = start.elapsed();
                                            tracing::info!("Set {} to {}% in {:?} (succeeded on retry)", id_clone, gamma_corrected, elapsed);
                                        }
                                        Err(e2) => {
                                            tracing::error!("Failed to set brightness on display {}: {}", id_clone, e2);
                                        }
                                    }
                                }
                            }
                        });

                        tasks.push(task);
                        synced_count += 1;
                    } else {
                        tracing::debug!("Skipping brightness sync for display {} (sync disabled)", id);
                    }
                }

                // Release the lock before awaiting tasks
                drop(last_brightness_map);

                // Wait for all brightness changes to complete in parallel
                if !tasks.is_empty() {
                    for task in tasks {
                        let _ = task.await;
                    }

                    tracing::debug!("Synced brightness on {} display(s) in parallel", synced_count);

                    // Delay to allow DDC monitors to process the brightness change
                    // DDC/CI takes ~125ms for set_brightness + 40ms protocol delay = ~165ms minimum
                    // Using 200ms to be safe and prevent UI read errors
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
        }

        tracing::warn!("Brightness change stream ended");
        Ok(())
    }
}

/// Spawn the brightness sync daemon if external displays are detected
#[cfg(feature = "brightness-sync-daemon")]
pub async fn spawn_if_needed(display_manager: crate::monitor::DisplayManager) {
    match BrightnessSyncDaemon::new(display_manager).await {
        Ok(Some(daemon)) => {
            // Spawn daemon in background
            tokio::spawn(async move {
                if let Err(e) = daemon.run().await {
                    tracing::error!("Brightness sync daemon error: {}", e);
                }
            });
        }
        Ok(None) => {
            // No external displays, daemon not needed
        }
        Err(e) => {
            tracing::error!("Failed to initialize brightness sync daemon: {}", e);
        }
    }
}

/// No-op when feature is disabled
#[cfg(not(feature = "brightness-sync-daemon"))]
pub async fn spawn_if_needed() {
    // No-op
}
