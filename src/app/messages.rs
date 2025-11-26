use std::collections::HashMap;
use crate::config::Config;
use crate::monitor::{DisplayId, MonitorInfo};
use cosmic::cosmic_theme::ThemeMode;
use tokio::sync::watch::Sender;
use crate::monitor::EventToSub;

#[derive(Clone, Debug)]
pub enum AppMsg {
    TogglePopup,
    #[allow(dead_code)]
    ToggleQuickSettings,
    ClosePopup,

    ConfigChanged(Config),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),

    SetScreenBrightness(DisplayId, f32),
    ToggleMinMaxBrightness(DisplayId),
    ToggleMonSettings(DisplayId),
    SetMonGammaMap(DisplayId, f32),
    SetMonitorSyncEnabled(DisplayId, bool),  // Per-monitor keyboard brightness sync toggle
    SetMonMinBrightness(DisplayId, u16),  // Per-monitor minimum brightness (0-100)

    /// Send from the subscription
    SubscriptionReady((HashMap<DisplayId, MonitorInfo>, Sender<EventToSub>)),
    /// Send from the subscription
    BrightnessWasUpdated(DisplayId, u16),
    Refresh,
    RefreshMonitors,
    TogglePermissionView,
    ToggleAboutView,
    OpenUrl(String),

    // Profile management
    ToggleProfilesSection,  // Toggle profiles section expanded/collapsed
    OpenNewProfileDialog,  // Open dialog to create new profile
    OpenEditProfileDialog(String),  // Open dialog to edit existing profile
    ProfileNameInput(String),  // Update profile name input field
    SaveProfileConfirm,  // Confirm save (from dialog)
    CancelProfileDialog,  // Cancel profile creation/edit
    LoadProfile(String),  // Load brightness values from a profile
    DeleteProfile(String),  // Delete a profile

    /// No operation message (for daemon spawn task)
    #[allow(dead_code)]
    Noop,
}
