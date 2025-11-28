// Copyright 2024 Jason Scurtu
// SPDX-License-Identifier: GPL-3.0-only

//! Configuration migration utilities
//!
//! Handles migrations between different config versions as the project evolves.

use crate::{app::APPID, config::Config};

/// Check if config contains old I2C-based DDC IDs and log migration warning
///
/// Detects numeric display IDs (like "22789") which were used before the
/// serial number-based ID system was introduced in config version 2.
pub fn check_v1_to_v2_migration(config: &Config) {
    // Detect old I2C-based IDs (numeric strings like "22789")
    let has_old_ddc_ids = config
        .monitors
        .keys()
        .any(|id| {
            !id.starts_with("ddc-")
                && !id.starts_with("apple-hid-")
                && id.parse::<u64>().is_ok()
        })
        || config.profiles.iter().any(|p| {
            p.brightness_values.keys().any(|id| {
                !id.starts_with("ddc-")
                    && !id.starts_with("apple-hid-")
                    && id.parse::<u64>().is_ok()
            })
        });

    if has_old_ddc_ids {
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        warn!("⚠️  CONFIGURATION UPDATE DETECTED");
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        warn!("");
        warn!("DDC/CI display IDs have been updated to use stable serial numbers.");
        warn!("Your existing monitor settings and profiles use the old format and");
        warn!("may not work correctly.");
        warn!("");
        warn!("What changed:");
        warn!("  • Old IDs: 22789, 22790 (unstable, changed on reboot)");
        warn!("  • New IDs: ddc-0x112E647C (stable, based on EDID serial)");
        warn!("");
        warn!("Action required:");
        warn!("  1. Your monitors will work, but settings may not apply");
        warn!("  2. Reconfigure per-monitor settings (gamma, sync, min brightness)");
        warn!("  3. Recreate any saved brightness profiles");
        warn!("");
        warn!("To start fresh, delete old config:");
        warn!("  rm -rf ~/.config/{}", APPID);
        warn!("");
        warn!("This is a one-time migration due to architectural improvements.");
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}
