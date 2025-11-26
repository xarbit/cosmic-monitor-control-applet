use std::collections::HashMap;
use crate::config::Config;
use crate::monitor::{DisplayId, MonitorInfo};
use cosmic::cosmic_theme::ThemeMode;
use tokio::sync::watch::Sender;
use crate::monitor::EventToSub;

#[derive(Clone, Debug)]
pub enum AppMsg {
    TogglePopup,
    ToggleQuickSettings,
    ClosePopup,

    ConfigChanged(Config),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),

    SetScreenBrightness(DisplayId, f32),
    ToggleMinMaxBrightness(DisplayId),
    ToggleMonSettings(DisplayId),
    SetMonGammaMap(DisplayId, f32),
    SetMonitorSyncEnabled(DisplayId, bool),  // Per-monitor F1/F2 sync toggle
    SetMonMinBrightness(DisplayId, u16),  // Per-monitor minimum brightness (0-100)

    /// Send from the subscription
    SubscriptionReady((HashMap<DisplayId, MonitorInfo>, Sender<EventToSub>)),
    /// Send from the subscription
    BrightnessWasUpdated(DisplayId, u16),
    Refresh,
    RefreshMonitors,
    /// No operation message (for daemon spawn task)
    Noop,
}
