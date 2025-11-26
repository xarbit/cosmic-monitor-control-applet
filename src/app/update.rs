use cosmic::app::Task;
use cosmic::cosmic_theme::ThemeMode;
use cosmic::cosmic_config::CosmicConfigEntry;

use crate::monitor::EventToSub;
use crate::config::{BrightnessProfile, MAX_PROFILES};
use std::collections::HashMap;

use super::messages::AppMsg;
use super::popup::PopupKind;
use super::state::{AppState, get_mapped_brightness};

impl AppState {
    pub fn update(&mut self, message: AppMsg) -> Task<AppMsg> {
        // Log ALL messages at info level for debugging
        match &message {
            AppMsg::RefreshMonitors => info!(">>> UPDATE: AppMsg::RefreshMonitors (manual refresh button)"),
            AppMsg::HotplugDetected => info!(">>> UPDATE: AppMsg::HotplugDetected (automatic hotplug)"),
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
                // Refresh brightness values from monitors (quick refresh)
                self.send(EventToSub::Refresh);
            }
            AppMsg::RefreshMonitors => {
                // Trigger full re-enumeration without cache (for manual refresh button)
                // This clears the cache and does a complete re-scan of all displays
                info!("RefreshMonitors message received (manual refresh), triggering full re-enumeration");
                self.send(EventToSub::ReEnumerateFull);
            }
            AppMsg::HotplugDetected => {
                // Trigger re-enumeration with cache (for hotplug events)
                // This keeps working displays and only probes for new ones
                info!("HotplugDetected message received, triggering cached re-enumeration");
                self.send(EventToSub::ReEnumerate);
            }
            AppMsg::TogglePermissionView => {
                self.show_permission_view = !self.show_permission_view;
            }
            AppMsg::ToggleAboutView => {
                self.show_about_view = !self.show_about_view;
            }
            AppMsg::OpenUrl(url) => {
                // Try portal first (for Flatpak), fallback to open crate
                let url_clone = url.clone();
                tokio::spawn(async move {
                    // Try using XDG portal (works in Flatpak)
                    if let Ok(url_parsed) = url::Url::parse(&url_clone) {
                        match ashpd::desktop::open_uri::OpenFileRequest::default()
                            .send_uri(&url_parsed)
                            .await
                        {
                            Ok(_) => {
                                info!("Opened URL via portal: {}", url_clone);
                                return;
                            }
                            Err(e) => {
                                debug!("Portal failed ({}), falling back to open crate", e);
                            }
                        }
                    }

                    // Fallback to open crate for non-sandboxed environments
                    if let Err(e) = open::that(&url_clone) {
                        error!("Failed to open URL {}: {}", url_clone, e);
                    } else {
                        info!("Opened URL via open crate: {}", url_clone);
                    }
                });
            }
            AppMsg::ToggleProfilesSection => {
                self.profiles_expanded = !self.profiles_expanded;
            }
            AppMsg::OpenNewProfileDialog => {
                self.profile_dialog_open = true;
                self.profile_name_input = String::new();
                self.editing_profile = None;
            }
            AppMsg::OpenEditProfileDialog(name) => {
                self.profile_dialog_open = true;
                self.profile_name_input = name.clone();
                self.editing_profile = Some(name);
            }
            AppMsg::ProfileNameInput(input) => {
                self.profile_name_input = input;
            }
            AppMsg::SaveProfileConfirm => {
                if self.profile_name_input.trim().is_empty() {
                    warn!("Cannot save profile with empty name");
                    return Task::none();
                }

                let name = self.profile_name_input.trim().to_string();

                let profile = if let Some(old_name) = &self.editing_profile {
                    // Editing existing profile - preserve brightness values, update name
                    if let Some(existing_profile) = self.config.get_profile(old_name).cloned() {
                        // If name changed, this will be handled by save_profile removing old name
                        BrightnessProfile::new(name.clone(), existing_profile.brightness_values)
                    } else {
                        warn!("Editing profile '{}' not found, creating new", old_name);
                        // Fallback: collect current values
                        let mut brightness_values = HashMap::new();
                        for (id, monitor) in &self.monitors {
                            let gamma = self.config.get_gamma_map(id);
                            let brightness = get_mapped_brightness(monitor.slider_brightness, gamma);
                            brightness_values.insert(id.clone(), brightness);
                        }
                        BrightnessProfile::new(name.clone(), brightness_values)
                    }
                } else {
                    // Creating new profile - collect current brightness values from all monitors
                    let mut brightness_values = HashMap::new();
                    for (id, monitor) in &self.monitors {
                        let gamma = self.config.get_gamma_map(id);
                        let brightness = get_mapped_brightness(monitor.slider_brightness, gamma);
                        brightness_values.insert(id.clone(), brightness);
                    }
                    BrightnessProfile::new(name.clone(), brightness_values)
                };

                // Update config
                let mut new_config = self.config.clone();

                // If editing and name changed, delete the old profile
                if let Some(old_name) = &self.editing_profile {
                    if old_name != &name {
                        new_config.delete_profile(old_name);
                    }
                } else {
                    // Creating new profile - check limit
                    if new_config.profiles.len() >= MAX_PROFILES {
                        warn!("Cannot create profile '{}': maximum of {} profiles reached", name, MAX_PROFILES);
                        return Task::none();
                    }
                }

                new_config.save_profile(profile);

                // Write to disk
                if let Err(e) = new_config.write_entry(&self.config_handler) {
                    error!("Failed to save profile '{}': {}", name, e);
                } else {
                    info!("Saved brightness profile: {}", name);
                    self.config = new_config;
                    self.profile_dialog_open = false;
                    self.profile_name_input.clear();
                    self.editing_profile = None;
                }
            }
            AppMsg::CancelProfileDialog => {
                self.profile_dialog_open = false;
                self.profile_name_input.clear();
                self.editing_profile = None;
            }
            AppMsg::LoadProfile(name) => {
                info!(">>> LoadProfile message received for: '{}'", name);

                // Clone the profile data to avoid borrow checker issues
                if let Some(profile) = self.config.get_profile(&name).cloned() {
                    info!("Profile '{}' found with {} monitors", name, profile.brightness_values.len());

                    // Collect all brightness commands to send as a batch
                    let mut batch_commands = Vec::new();

                    // Apply brightness values to all monitors in the profile
                    for (id, brightness) in profile.brightness_values {
                        info!("Profile '{}': Processing monitor {} -> {}%", name, id, brightness);

                        if self.monitors.contains_key(&id) {
                            // Prepare hardware command
                            let min_brightness = self.config.get_min_brightness(&id);
                            let clamped_brightness = brightness.max(min_brightness);

                            info!(">>> Preparing brightness command: {} = {}% (clamped from {}%)",
                                  id, clamped_brightness, brightness);

                            batch_commands.push((id.clone(), clamped_brightness));

                            // Update UI state
                            if let Some(monitor) = self.monitors.get_mut(&id) {
                                let gamma = self.config.get_gamma_map(&id);
                                let old_slider = monitor.slider_brightness;
                                monitor.set_slider_brightness(clamped_brightness, gamma);
                                info!("Updated UI slider for {}: {:.2} -> {:.2}",
                                      id, old_slider, monitor.slider_brightness);
                            }
                        } else {
                            warn!("Profile '{}' contains monitor '{}' which is not currently connected", name, id);
                        }
                    }

                    // Send all brightness commands as a single batch (atomic operation)
                    if !batch_commands.is_empty() {
                        info!(">>> Sending batch of {} brightness commands", batch_commands.len());
                        self.send(EventToSub::SetBatch(batch_commands));
                    }

                    info!(">>> LoadProfile '{}' processing complete", name);
                } else {
                    error!("Profile '{}' not found in config!", name);
                }
            }
            AppMsg::DeleteProfile(name) => {
                let mut new_config = self.config.clone();
                if new_config.delete_profile(&name) {
                    if let Err(e) = new_config.write_entry(&self.config_handler) {
                        error!("Failed to delete profile '{}': {}", name, e);
                    } else {
                        info!("Deleted brightness profile: {}", name);
                        self.config = new_config;
                    }
                } else {
                    warn!("Profile '{}' not found for deletion", name);
                }
            }
            AppMsg::Noop => {
                // No operation - used for daemon spawn task completion
            }
        }
        Task::none()
    }
}
