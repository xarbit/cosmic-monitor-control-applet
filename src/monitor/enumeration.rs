use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::protocols::ddc_ci::DdcCiDisplay;

#[cfg(feature = "apple-hid-displays")]
use crate::protocols::apple_hid::AppleHidDisplay;

use super::backend::{DisplayBackend, DisplayId, MonitorInfo};

/// Enumerate all available displays (DDC/CI and Apple HID)
/// Returns a map of display IDs to MonitorInfo and their backends
pub async fn enumerate_displays() -> (
    HashMap<DisplayId, MonitorInfo>,
    HashMap<DisplayId, Arc<Mutex<DisplayBackend>>>,
    bool,
) {
    let mut res = HashMap::new();
    let mut displays = HashMap::new();
    let mut some_failed = false;

    info!("=== START ENUMERATE ===");

    // Enumerate DDC/CI displays concurrently
    let ddc_displays = DdcCiDisplay::enumerate();
    info!("Found {} DDC/CI display(s) to probe", ddc_displays.len());
    let mut ddc_tasks = Vec::new();

    for display in ddc_displays {
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

                let brightness = match backend.get_brightness() {
                    Ok(v) => v,
                    // on my machine, i get this error when starting the session
                    // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                    // This go away after the third attempt
                    Err(e) => {
                        error!("can't get_vcp_feature: {e}");
                        return Err(e);
                    }
                };
                debug_assert!(brightness <= 100);

                let id = backend.id();
                let name = backend.name();

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
                displays.insert(id, Arc::new(Mutex::new(backend)));
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
        // Run Apple HID enumeration in spawn_blocking to avoid blocking the runtime
        let apple_result = tokio::task::spawn_blocking(|| {
            let mut results = Vec::new();
            match hidapi::HidApi::new() {
                Ok(api) => {
                    match AppleHidDisplay::enumerate(&api) {
                        Ok(apple_displays) => {
                            for display in apple_displays {
                                let mut backend = DisplayBackend::AppleHid(display);

                                let brightness = match backend.get_brightness() {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("can't get Apple HID display brightness: {e}");
                                        continue;
                                    }
                                };

                                let id = backend.id();
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
            displays.insert(id, Arc::new(Mutex::new(backend)));
        }
    }

    info!("=== END ENUMERATE: Found {} monitors ===", res.len());

    (res, displays, some_failed)
}
