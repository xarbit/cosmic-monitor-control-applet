// SPDX-License-Identifier: GPL-3.0-only
//! DDC/CI (Display Data Channel Command Interface) protocol implementation
//!
//! DDC/CI is a standard protocol for controlling monitors over I2C bus.
//! It's supported by most modern external monitors via the video cable.

use anyhow::Result;
use ddc_hi::{Ddc, Display};

use super::DisplayProtocol;

/// VCP (Virtual Control Panel) code for brightness
const BRIGHTNESS_CODE: u8 = 0x10;

/// DDC/CI display implementation
pub struct DdcCiDisplay {
    display: Display,
    /// EDID serial number from cosmic-randr (if available)
    /// Used to generate stable display IDs that persist across reboots
    edid_serial: Option<String>,
}

impl DdcCiDisplay {
    /// Create a new DDC/CI display wrapper
    pub fn new(display: Display) -> Self {
        Self { display, edid_serial: None }
    }

    /// Create a new DDC/CI display wrapper with an EDID serial number
    pub fn new_with_serial(display: Display, edid_serial: Option<String>) -> Self {
        Self { display, edid_serial }
    }

    /// Set the EDID serial number (used to generate stable display IDs)
    pub fn set_edid_serial(&mut self, serial: Option<String>) {
        self.edid_serial = serial;
    }

    /// Enumerate all DDC/CI displays
    pub fn enumerate() -> Vec<Self> {
        Display::enumerate()
            .into_iter()
            .map(Self::new)
            .collect()
    }
}

impl DisplayProtocol for DdcCiDisplay {
    fn id(&self) -> String {
        // Use EDID serial number for stable IDs if available
        if let Some(ref serial) = self.edid_serial {
            format!("ddc-{}", serial)
        } else {
            // Fallback to old I2C-based ID (unstable across reboots)
            // This will be logged as a warning during enumeration
            self.display.info.id.clone()
        }
    }

    fn name(&self) -> String {
        self.display
            .info
            .model_name
            .clone()
            .unwrap_or_default()
    }

    fn get_brightness(&mut self) -> Result<u16> {
        let value = self.display.handle.get_vcp_feature(BRIGHTNESS_CODE)?;
        Ok(value.value())
    }

    fn set_brightness(&mut self, value: u16) -> Result<()> {
        self.display
            .handle
            .set_vcp_feature(BRIGHTNESS_CODE, value)?;
        Ok(())
    }
}

impl std::fmt::Debug for DdcCiDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DdcCiDisplay(id: {}, name: {})", self.id(), self.name())
    }
}
