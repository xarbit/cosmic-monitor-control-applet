use std::collections::HashMap;

use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    iced::Subscription,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::{APPID, AppMsg},
    monitor::DisplayId,
};

pub const CONFIG_VERSION: u64 = 2;
pub const MAX_PROFILES: usize = 10;

/// A brightness profile stores brightness values and display settings for all monitors
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BrightnessProfile {
    pub name: String,
    /// Map of display_id -> brightness (0-100)
    pub brightness_values: HashMap<DisplayId, u16>,
    /// Map of display_id -> scale factor
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub scale_values: HashMap<DisplayId, f32>,
    /// Map of display_id -> transform/rotation
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transform_values: HashMap<DisplayId, String>,
    /// Map of display_id -> position (x, y)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub position_values: HashMap<DisplayId, (i32, i32)>,
}

impl BrightnessProfile {
    pub fn new(name: String, brightness_values: HashMap<DisplayId, u16>) -> Self {
        Self {
            name,
            brightness_values,
            scale_values: HashMap::new(),
            transform_values: HashMap::new(),
            position_values: HashMap::new(),
        }
    }
}

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub monitors: HashMap<DisplayId, MonitorConfig>,
    /// Saved brightness profiles
    #[serde(default)]
    pub profiles: Vec<BrightnessProfile>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MonitorConfig {
    pub gamma_map: f32,
    /// Whether this monitor should respond to keyboard brightness keys
    #[serde(default = "default_sync_enabled")]
    pub sync_with_brightness_keys: bool,
    /// Minimum brightness percentage (0-100) that will be sent to hardware
    #[serde(default = "default_min_brightness")]
    pub min_brightness: u16,
    /// Display scale factor (1.0, 1.5, 2.0, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    /// Display transform/rotation (normal, 90, 180, 270, flipped, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,
    /// Display position (x, y) in virtual desktop
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<(i32, i32)>,
}

fn default_sync_enabled() -> bool {
    true  // Default to enabled for all monitors
}

fn default_min_brightness() -> u16 {
    0  // Default to no minimum
}

impl MonitorConfig {
    pub fn new() -> Self {
        Self {
            gamma_map: 1.,
            sync_with_brightness_keys: true,
            min_brightness: 0,
            scale: None,
            transform: None,
            position: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_default_gamma(gamma: f32) -> Self {
        Self {
            gamma_map: gamma,
            sync_with_brightness_keys: true,
            min_brightness: 0,
            scale: None,
            transform: None,
            position: None,
        }
    }
}

impl Config {
    pub fn get_gamma_map(&self, id: &str) -> f32 {
        self.monitors.get(id).map(|m| m.gamma_map).unwrap_or_else(|| {
            // Default gamma based on display type
            if id.starts_with("apple-hid-") {
                // Apple displays and LG UltraFine displays (which use Apple HID protocol) work better with 1.8
                1.8
            } else {
                // DDC displays default to linear (1.0)
                1.0
            }
        })
    }

    pub fn is_sync_enabled(&self, id: &str) -> bool {
        self.monitors.get(id).map(|m| m.sync_with_brightness_keys).unwrap_or(true)
    }

    pub fn get_min_brightness(&self, id: &str) -> u16 {
        self.monitors.get(id).map(|m| m.min_brightness).unwrap_or(0)
    }

    /// Find a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&BrightnessProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Add or update a profile
    pub fn save_profile(&mut self, profile: BrightnessProfile) {
        // Remove any existing profile with the same name
        self.profiles.retain(|p| p.name != profile.name);
        // Add the new profile
        self.profiles.push(profile);
    }

    /// Delete a profile by name
    pub fn delete_profile(&mut self, name: &str) -> bool {
        let len_before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        self.profiles.len() != len_before
    }
}

pub fn sub() -> Subscription<AppMsg> {
    struct ConfigSubscription;

    cosmic_config::config_subscription(
        std::any::TypeId::of::<ConfigSubscription>(),
        APPID.into(),
        CONFIG_VERSION,
    )
    .map(|update| {
        if !update.errors.is_empty() {
            error!("can't load config {:?}: {:?}", update.keys, update.errors);
        }
        AppMsg::ConfigChanged(update.config)
    })
}
