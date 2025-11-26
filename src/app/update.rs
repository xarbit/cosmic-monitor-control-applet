use cosmic::app::Task;
use cosmic::cosmic_theme::ThemeMode;
use cosmic::cosmic_config::CosmicConfigEntry;

use crate::monitor::EventToSub;

use super::messages::AppMsg;
use super::popup::PopupKind;
use super::state::AppState;

impl AppState {
    pub fn update(&mut self, message: AppMsg) -> Task<AppMsg> {
        // Log ALL messages at info level for debugging
        match &message {
            AppMsg::RefreshMonitors => info!(">>> UPDATE: AppMsg::RefreshMonitors"),
            AppMsg::SubscriptionReady((monitors, _)) => info!(">>> UPDATE: AppMsg::SubscriptionReady with {} monitors", monitors.len()),
            _ => debug!("{:?}", message),
        }

        match message {
            AppMsg::TogglePopup => {
                return self.toggle_popup(PopupKind::Popup);
            }
            AppMsg::ToggleQuickSettings => return self.toggle_popup(PopupKind::QuickSettings),
            AppMsg::ClosePopup => return self.close_popup(),
            AppMsg::SetScreenBrightness(id, slider_brightness) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.slider_brightness = slider_brightness;
                    let gamma = self.config.get_gamma_map(&id);
                    let min_brightness = self.config.get_min_brightness(&id);
                    let mut b = monitor.get_mapped_brightness(gamma);
                    // Apply minimum brightness clamp
                    if b < min_brightness {
                        b = min_brightness;
                    }
                    self.send(EventToSub::Set(id, b));
                }
            }
            AppMsg::ToggleMinMaxBrightness(id) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    let new_val = match monitor.slider_brightness {
                        x if x < 0.5 => 100,
                        _ => 0,
                    };
                    monitor.slider_brightness = new_val as f32 / 100.0;
                    self.send(EventToSub::Set(id, new_val));
                }
            }
            AppMsg::ThemeModeConfigChanged(config) => {
                self.theme_mode_config = config;
            }
            AppMsg::SetDarkMode(dark) => {
                fn set_theme_mode(mode: &ThemeMode) -> anyhow::Result<()> {
                    let helper = ThemeMode::config()?;
                    mode.write_entry(&helper)?;
                    Ok(())
                }

                self.theme_mode_config.is_dark = dark;

                if let Err(e) = set_theme_mode(&self.theme_mode_config) {
                    error!("can't write theme mode {e}");
                }
            }
            AppMsg::SubscriptionReady((monitors, sender)) => {
                self.set_monitors(monitors, sender);
            }
            AppMsg::BrightnessWasUpdated(id, brightness) => {
                self.update_brightness(id, brightness);
            }
            AppMsg::SetMonGammaMap(id, gamma) => {
                if let Some(monitor) = self.monitors.get(&id) {
                    let b = monitor.get_mapped_brightness(gamma);
                    self.send(EventToSub::Set(id.clone(), b));
                }

                self.update_monitor_config(&id, |monitor| {
                    monitor.gamma_map = gamma;
                });
            }
            AppMsg::ToggleMonSettings(id) => {
                if let Some(mon) = self.monitors.get_mut(&id) {
                    mon.settings_expanded = !mon.settings_expanded;
                }
            }
            AppMsg::SetMonitorSyncEnabled(id, enabled) => {
                self.update_monitor_config(&id, |monitor| {
                    monitor.sync_with_brightness_keys = enabled;
                });
            }
            AppMsg::SetMonMinBrightness(id, min_brightness) => {
                self.update_monitor_config(&id, |monitor| {
                    monitor.min_brightness = min_brightness;
                });
            }
            AppMsg::ConfigChanged(config) => self.config = config,
            AppMsg::Refresh => {
                // Refresh brightness values from monitors
                self.send(EventToSub::Refresh);
            }
            AppMsg::RefreshMonitors => {
                // Trigger re-enumeration of displays (hotplug detection)
                info!("RefreshMonitors message received, triggering re-enumeration");
                self.send(EventToSub::ReEnumerate);
            }
            AppMsg::Noop => {
                // No operation - used for daemon spawn task completion
            }
        }
        Task::none()
    }
}
