use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{Config, MonitorConfig};
use crate::monitor::{DisplayId, DisplayManager, EventToSub, MonitorInfo};
use crate::permissions::PermissionCheckResult;
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::Config as CosmicConfig;
use tokio::sync::watch::Sender;

use super::messages::AppMsg;
use super::popup::{Popup, PopupKind};

#[derive(Debug, Clone)]
pub struct MonitorState {
    pub name: String,
    /// Between 0 and 1
    pub slider_brightness: f32,
    pub settings_expanded: bool,
    pub connector_name: Option<String>,
}

pub fn get_mapped_brightness(slider_brightness: f32, gamma: f32) -> u16 {
    (slider_brightness.powf(gamma) * 100.0).round() as u16
}

pub fn get_slider_brightness(brightness: u16, gamma: f32) -> f32 {
    (brightness as f32 / 100.0).powf(1.0 / gamma)
}

impl MonitorState {
    pub fn get_mapped_brightness(&self, gamma: f32) -> u16 {
        get_mapped_brightness(self.slider_brightness, gamma)
    }

    pub fn set_slider_brightness(&mut self, brightness: u16, gamma: f32) {
        self.slider_brightness = get_slider_brightness(brightness, gamma)
    }
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub struct AppState {
    pub core: Core,
    pub(super) popup: Option<Popup>,
    pub monitors: HashMap<DisplayId, MonitorState>,
    pub theme_mode_config: cosmic::cosmic_theme::ThemeMode,
    pub(super) sender: Option<Sender<EventToSub>>,
    pub config: Config,
    pub(super) config_handler: CosmicConfig,
    pub(super) last_quit: Option<(u128, PopupKind)>,
    pub permission_status: Option<PermissionCheckResult>,
    pub show_permission_view: bool,
    pub show_about_view: bool,
    pub display_manager: DisplayManager,
    // Profile UI state
    pub profile_dialog_open: bool,
    pub profile_name_input: String,
    pub editing_profile: Option<String>, // If Some, we're editing an existing profile
    pub profiles_expanded: bool,
}

impl AppState {
    pub fn new(core: Core, config_handler: CosmicConfig, config: Config) -> Self {
        // Check permissions on startup
        let permission_status = crate::permissions::check_i2c_permissions();

        // Log permission status
        debug!("Permission check results:");
        for req in &permission_status.requirements {
            let icon = match req.status {
                crate::permissions::RequirementStatus::Met => "✓",
                crate::permissions::RequirementStatus::NotMet => "✗",
                crate::permissions::RequirementStatus::NotApplicable => "-",
                crate::permissions::RequirementStatus::Partial => "ⓘ",
            };
            debug!("  {} {}: {}", icon, req.name, req.description);
        }

        if permission_status.has_issues() {
            warn!("Hardware permission issues detected:");
            for req in &permission_status.requirements {
                if req.status == crate::permissions::RequirementStatus::NotMet {
                    warn!("  ✗ {}: {}", req.name, req.description);
                }
            }
        } else {
            info!("{}", permission_status.summary());
        }

        AppState {
            core,
            config_handler,
            config,
            popup: None,
            monitors: HashMap::new(),
            theme_mode_config: cosmic::cosmic_theme::ThemeMode::default(),
            sender: None,
            last_quit: None,
            permission_status: Some(permission_status),
            show_permission_view: false,
            show_about_view: false,
            display_manager: DisplayManager::new(),
            profile_dialog_open: false,
            profile_name_input: String::new(),
            editing_profile: None,
            profiles_expanded: false,
        }
    }

    pub fn send(&self, e: EventToSub) {
        if let Some(sender) = &self.sender {
            if let Err(err) = sender.send(e) {
                // This can happen if the monitor subscription is already re-enumerating
                // Just log it, don't panic
                debug!("Failed to send event to monitor subscription: {:?}", err);
            }
        }
    }

    pub fn update_monitor_config(&mut self, id: &str, f: impl Fn(&mut MonitorConfig)) {
        let mut monitors = self.config.monitors.clone();

        if let Some(monitor) = monitors.get_mut(id) {
            f(monitor);
        } else {
            let mut monitor = MonitorConfig::new();
            f(&mut monitor);
            monitors.insert(id.to_string(), monitor);
        }

        if let Err(e) = self.config.set_monitors(&self.config_handler, monitors) {
            error!("can't write config: {e}");
        }
    }

    pub fn set_monitors(&mut self, monitors: HashMap<DisplayId, MonitorInfo>, sender: Sender<EventToSub>) {
        info!("SubscriptionReady received with {} monitors", monitors.len());
        for (id, m) in monitors.iter() {
            info!("  - Monitor: {} ({})", m.name, id);
        }

        self.monitors = monitors
            .into_iter()
            .map(|(id, m)| {
                (
                    id.clone(),
                    MonitorState {
                        name: m.name,
                        slider_brightness: get_slider_brightness(
                            m.brightness,
                            self.config.get_gamma_map(&id),
                        ),
                        settings_expanded: false,
                        connector_name: m.connector_name,
                    },
                )
            })
            .collect();

        self.sender.replace(sender);
    }

    pub fn update_brightness(&mut self, id: DisplayId, brightness: u16) {
        if let Some(monitor) = self.monitors.get_mut(&id) {
            monitor.set_slider_brightness(brightness, self.config.get_gamma_map(&id));
        }
    }

    pub fn close_popup(&mut self) -> Task<AppMsg> {
        for mon in self.monitors.values_mut() {
            mon.settings_expanded = false;
        }

        // Reset permission view and about view when closing popup
        self.show_permission_view = false;
        self.show_about_view = false;

        if let Some(popup) = self.popup.take() {
            self.last_quit = Some((now(), popup.kind));
            cosmic::iced_winit::commands::popup::destroy_popup(popup.id)
        } else {
            Task::none()
        }
    }

    pub fn should_suppress_popup(&self, kind: PopupKind) -> bool {
        self.last_quit
            .map(|(t, k)| (now() - t) < 200 && k == kind)
            .unwrap_or(false)
    }
}
