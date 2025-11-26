use std::collections::HashMap;

use crate::protocols::ddc_ci::DdcCiDisplay;
use crate::protocols::DisplayProtocol;

#[cfg(feature = "apple-hid-displays")]
use crate::protocols::apple_hid::AppleHidDisplay;

use super::backend::{DisplayBackend, DisplayId, MonitorInfo};

/// Enumerate all available displays (DDC/CI and Apple HID)
/// Returns a map of display IDs to MonitorInfo and their backends
///
/// `known_ids`: Set of display IDs that are already cached and should be skipped
pub async fn enumerate_displays(
    known_ids: &std::collections::HashSet<DisplayId>,
) -> (
    HashMap<DisplayId, MonitorInfo>,
    HashMap<DisplayId, std::sync::Arc<tokio::sync::Mutex<DisplayBackend>>>,
    bool,
) {
    let mut res = HashMap::new();
    let mut displays = HashMap::new();
    let mut some_failed = false;

    info!("=== START ENUMERATE (known displays: {}) ===", known_ids.len());

    // Enumerate DDC/CI displays concurrently
    let ddc_displays = DdcCiDisplay::enumerate();
    info!("Found {} DDC/CI display(s) total", ddc_displays.len());
    let mut ddc_tasks = Vec::new();

    for display in ddc_displays {
        // Get display ID before moving it
        let id = display.id();

        // Skip displays that are already in cache
        if known_ids.contains(&id) {
            info!("Skipping cached DDC/CI display: {}", id);
            continue;
        }

        info!("Probing new DDC/CI display: {}", id);
        let task = tokio::spawn(async move {
            // Run blocking I/O operations in spawn_blocking to avoid blocking the runtime
            tokio::task::spawn_blocking(move || {
                let mut backend = DisplayBackend::DdcCi(display);

                // Wake up DDC by doing a read-write cycle
                // Some DDC monitors need an initial write to establish I2C communication
                if let Ok(current_brightness) = backend.get_brightness() {
                    let _ = backend.set_brightness(current_brightness);
                    // Small delay to let DDC settle
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }

                // Retry logic for DDC/CI communication errors
                // After hotplug, DDC/CI may not be ready immediately
                // Some monitors need multiple attempts with longer delays
                let brightness = {
                    let mut last_error = None;
                    let mut brightness_value = None;

                    // Try up to 5 times with increasing delays
                    for attempt in 1..=5 {
                        match backend.get_brightness() {
                            Ok(v) => {
                                if attempt > 1 {
                                    info!("DDC/CI display succeeded on attempt {}", attempt);
                                }
                                brightness_value = Some(v);
                                break;
                            }
                            Err(e) => {
                                debug!("DDC/CI attempt {} failed: {}", attempt, e);
                                last_error = Some(e);
                                if attempt < 5 {
                                    // Increase delay for each retry (100ms, 200ms, 300ms, 400ms)
                                    let delay_ms = attempt as u64 * 100;
                                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                                }
                            }
                        }
                    }

                    match brightness_value {
                        Some(v) => v,
                        None => {
                            let err = last_error.unwrap();
                            let id = backend.id();
                            let name = backend.name();
                            error!(
                                display_id = %id,
                                display_name = %name,
                                error = ?err,
                                "Failed to get brightness after 5 attempts - monitor may not support DDC/CI"
                            );
                            return Err(err);
                        }
                    }
                };
                debug_assert!(brightness <= 100);

                let id = backend.id();
                let name = backend.name();

                // Warn if monitor reports 0% brightness (common issue with some portable monitors)
                if brightness == 0 {
                    warn!(
                        display_id = %id,
                        display_name = %name,
                        "Monitor reports 0% brightness - this may indicate DDC/CI communication issues or unsupported monitor"
                    );
                }

                let mon = MonitorInfo {
                    name,
                    brightness,
                };

                Ok((id, mon, backend))
            }).await.unwrap()
        });
        ddc_tasks.push(task);
    }

    // Wait for all DDC tasks to complete
    for task in ddc_tasks {
        match task.await {
            Ok(Ok((id, mon, backend))) => {
                info!("Successfully initialized DDC/CI display: {} ({})", mon.name, id);
                res.insert(id.clone(), mon);
                displays.insert(id, std::sync::Arc::new(tokio::sync::Mutex::new(backend)));
            }
            Ok(Err(e)) => {
                error!("Failed to initialize DDC/CI display: {}", e);
                some_failed = true;
            }
            Err(e) => {
                error!("Task join error: {e}");
                some_failed = true;
            }
        }
    }

    // Enumerate Apple HID displays
    #[cfg(feature = "apple-hid-displays")]
    {
        // Clone known_ids for use in spawn_blocking
        let known_ids_clone = known_ids.clone();

        // Run Apple HID enumeration in spawn_blocking to avoid blocking the runtime
        let apple_result = tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            match hidapi::HidApi::new() {
                Ok(api) => {
                    match AppleHidDisplay::enumerate(&api) {
                        Ok(apple_displays) => {
                            for display in apple_displays {
                                let mut backend = DisplayBackend::AppleHid(display);
                                let id = backend.id();

                                // Skip displays that are already in cache
                                if known_ids_clone.contains(&id) {
                                    info!("Skipping cached Apple HID display: {}", id);
                                    continue;
                                }

                                info!("Probing new Apple HID display: {}", id);

                                let brightness = match backend.get_brightness() {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("can't get Apple HID display brightness: {e}");
                                        continue;
                                    }
                                };

                                let name = backend.name();

                                let mon = MonitorInfo {
                                    name,
                                    brightness,
                                };

                                results.push((id, mon, backend));
                            }
                        }
                        Err(e) => {
                            error!("Failed to enumerate Apple HID displays: {e}");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to initialize HID API: {e}");
                }
            }
            results
        }).await.unwrap();

        for (id, mon, backend) in apple_result {
            info!("Successfully initialized Apple HID display: {} ({})", mon.name, id);
            res.insert(id.clone(), mon);
            displays.insert(id, std::sync::Arc::new(tokio::sync::Mutex::new(backend)));
        }
    }

    info!("=== END ENUMERATE: Found {} monitors ===", res.len());

    (res, displays, some_failed)
}
