use crate::protocols::{ddc_ci::DdcCiDisplay, DisplayProtocol};

#[cfg(feature = "apple-hid-displays")]
use crate::protocols::apple_hid::AppleHidDisplay;

pub type DisplayId = String;
pub type ScreenBrightness = u16;

/// Backend type for display control
pub enum DisplayBackend {
    /// DDC/CI protocol (standard external monitors via I2C)
    DdcCi(DdcCiDisplay),
    /// Apple HID protocol (Apple Studio Display, LG UltraFine, etc.)
    #[cfg(feature = "apple-hid-displays")]
    AppleHid(AppleHidDisplay),
}

impl std::fmt::Debug for DisplayBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayBackend::DdcCi(display) => write!(f, "{:?}", display),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => write!(f, "{:?}", display),
        }
    }
}

impl DisplayBackend {
    /// Get the display ID
    pub fn id(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.id(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.id(),
        }
    }

    /// Get the display name
    pub fn name(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.name(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.name(),
        }
    }

    /// Get the current brightness (0-100)
    pub fn get_brightness(&mut self) -> anyhow::Result<u16> {
        match self {
            DisplayBackend::DdcCi(display) => display.get_brightness(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.get_brightness(),
        }
    }

    /// Set the brightness (0-100)
    pub fn set_brightness(&mut self, value: u16) -> anyhow::Result<()> {
        match self {
            DisplayBackend::DdcCi(display) => display.set_brightness(value),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.set_brightness(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub brightness: u16,
}

#[derive(Debug, Clone)]
pub enum EventToSub {
    Refresh,
    Set(DisplayId, ScreenBrightness),
    /// Set brightness for multiple displays atomically (won't be lost in watch channel)
    SetBatch(Vec<(DisplayId, ScreenBrightness)>),
    /// Re-enumerate with cache (for hotplug events)
    #[allow(dead_code)]
    ReEnumerate,
    /// Re-enumerate without cache (for manual refresh button)
    ReEnumerateFull,
}
