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

    // Query cosmic-randr EARLY to get serial numbers for correlation
    let randr_outputs = match crate::randr::get_outputs().await {
        Ok(outputs) => {
            info!("Found {} Wayland output(s) from cosmic-randr (early query)", outputs.len());
            Some(outputs)
        }
        Err(e) => {
            warn!("Failed to query cosmic-randr early: {}", e);
            None
        }
    };

    // Enumerate DDC/CI displays concurrently
    let ddc_displays = DdcCiDisplay::enumerate();
    info!("Found {} DDC/CI display(s) total", ddc_displays.len());
    let mut ddc_tasks = Vec::new();

    for mut display in ddc_displays {
        // Try to match with cosmic-randr output and set serial number BEFORE getting ID
        if let Some(ref outputs) = randr_outputs {
            let model_name = display.name();
            if let Some(output_info) = crate::randr::find_matching_output(&model_name, outputs) {
                if output_info.enabled {
                    if let Some(ref serial) = output_info.serial_number {
                        debug!("Setting EDID serial for DDC display '{}': {}", model_name, serial);
                        display.set_edid_serial(Some(serial.clone()));
                    }
                }
            }
        }

        // Get display ID after setting serial number
        let id = display.id();

        // Warn if using unstable I2C-based ID (no serial number)
        if !id.starts_with("ddc-") {
            warn!("DDC/CI display '{}' using unstable I2C-based ID: {} - settings may not persist across reboots",
                  display.name(), id);
        }

        // Skip displays that are already in cache
        if known_ids.contains(&id) {
            info!("Skipping cached DDC/CI display: {}", id);
            continue;
        }

        info!("Probing new DDC/CI display: {} (ID: {})", display.name(), id);
        let task = tokio::spawn(async move {
            // Run blocking I/O operations in spawn_blocking to avoid blocking the runtime
            tokio::task::spawn_blocking(move || {
                let mut backend = DisplayBackend::DdcCi(display);

                // Wake up DDC by doing a read-write cycle
                // Some DDC monitors need an initial write to establish I2C communication
                // Try to read current brightness, and if successful, write it back to wake up the display
                // If the first read fails, still try a write with a default value to wake it up
                match backend.get_brightness() {
                    Ok(current_brightness) => {
                        // Display responded, write back to ensure wake-up
                        let _ = backend.set_brightness(current_brightness);
                    }
                    Err(_) => {
                        // Display didn't respond, try writing a value to wake it up
                        // Use 50% as a safe default that won't blind or go dark
                        let _ = backend.set_brightness(50);
                    }
                }
                // Always wait for DDC to settle after wake-up attempt
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Retry logic for DDC/CI communication errors
                // After hotplug/wake-up, DDC/CI may not be ready immediately
                // Some monitors need multiple attempts with delays
                let brightness = {
                    let mut last_error = None;
                    let mut brightness_value = None;

                    // Try up to 5 times with delays for initial startup
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
                                    // Progressive delay for wake-up: 100ms, 150ms, 200ms, 250ms
                                    let delay_ms = 50 + (attempt as u64 * 50);
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
                    connector_name: None,
                    edid_serial: None,
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
                                    connector_name: None,
                                    edid_serial: None,
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

    // Correlate displays with Wayland outputs from cosmic-randr
    // Reuse randr_outputs if we already fetched it, or query now for Apple HID displays
    if !res.is_empty() {
        let outputs = match randr_outputs {
            Some(outputs) => Some(outputs),
            None => {
                match crate::randr::get_outputs().await {
                    Ok(outputs) => {
                        info!("Found {} Wayland output(s) from cosmic-randr (late query)", outputs.len());
                        Some(outputs)
                    }
                    Err(e) => {
                        warn!("Failed to query cosmic-randr for output info: {}", e);
                        None
                    }
                }
            }
        };

        if let Some(outputs) = outputs {
            // Try to match each display with a Wayland output
            for (id, mon) in res.iter_mut() {
                // Only populate connector_name and edid_serial if not already set
                if mon.connector_name.is_none() || mon.edid_serial.is_none() {
                    if let Some(output_info) = crate::randr::find_matching_output(&mon.name, &outputs) {
                        if output_info.enabled {
                            info!("Matched display '{}' ({}) to connector '{}' (serial: {:?})",
                                mon.name, id, output_info.connector_name, output_info.serial_number);
                            if mon.connector_name.is_none() {
                                mon.connector_name = Some(output_info.connector_name);
                            }
                            if mon.edid_serial.is_none() {
                                mon.edid_serial = output_info.serial_number.clone();
                            }
                        } else {
                            debug!("Found match for '{}' but output is disabled", mon.name);
                        }
                    } else {
                        debug!("No matching Wayland output found for display: {} ({})", mon.name, id);
                    }
                }
            }
        } else {
            debug!("Display connector names and serials will not be available");
        }
    }

    (res, displays, some_failed)
}
