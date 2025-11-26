use crate::app::{AppMsg, AppState, MonitorState};
use crate::fl;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, horizontal_space, icon, mouse_area, row, slider, text, toggler,
    tooltip,
};

use super::common::brightness_icon;

impl AppState {
    /// View for a list of all monitors
    pub fn monitors_view(&self) -> Option<Element<'_, AppMsg>> {
        (!self.monitors.is_empty()).then(|| {
            let mut monitors: Vec<_> = self.monitors.iter().collect();
            monitors.sort_by_key(|(id, _)| *id);

            column()
                .padding(8)
                .extend(
                    monitors
                        .into_iter()
                        .map(|(id, monitor)| self.monitor_view(id, monitor)),
                )
                .into()
        })
    }

    /// View for a single monitor with brightness slider and settings
    pub fn monitor_view<'a>(&self, id: &'a str, monitor: &'a MonitorState) -> Element<'a, AppMsg> {
        let gamma_map = self.config.get_gamma_map(id);

        row()
            .padding(2.0)
            .push(
                container(
                    mouse_area(
                        column()
                            .spacing(8.0)
                            .padding(4.0)
                            .push(tooltip(
                                icon::icon(brightness_icon(monitor.slider_brightness)).size(24),
                                text(&monitor.name),
                                tooltip::Position::Right,
                            ))
                            .push_maybe(monitor.settings_expanded.then(|| {
                                tooltip(
                                    icon::from_name("emblem-system-symbolic")
                                        .size(24)
                                        .symbolic(true),
                                    text(fl!("monitor_settings")),
                                    tooltip::Position::Right,
                                )
                            })),
                    )
                    .on_press(AppMsg::ToggleMinMaxBrightness(id.to_string()))
                    .on_right_press(AppMsg::ToggleMonSettings(id.to_string()))
                    .on_scroll(|delta| {
                        let change = match delta {
                            cosmic::iced::mouse::ScrollDelta::Lines { x, y } => (x + y) / 20.0,
                            cosmic::iced::mouse::ScrollDelta::Pixels { y, .. } => y / 300.0,
                        };
                        AppMsg::SetScreenBrightness(
                            id.to_string(),
                            (monitor.slider_brightness + change).clamp(0.0, 1.0),
                        )
                    }),
                )
                .class(if monitor.settings_expanded {
                    cosmic::style::Container::Dropdown
                } else {
                    cosmic::style::Container::Transparent
                }),
            )
            .push(
                column()
                    .spacing(8.0)
                    .padding(4.0)
                    .push(
                        text(&monitor.name)
                            .size(12),
                    )
                    .push(
                        row()
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(slider(
                                0..=100,
                                (monitor.slider_brightness * 100.0) as u16,
                                move |brightness| {
                                    AppMsg::SetScreenBrightness(
                                        id.to_string(),
                                        brightness as f32 / 100.0,
                                    )
                                },
                            ))
                            .push(
                                text(format!("{:.0}%", monitor.get_mapped_brightness(gamma_map)))
                                    .size(16)
                                    .width(Length::Fixed(35.0)),
                            ),
                    )
                    .push_maybe(monitor.settings_expanded.then(|| {
                        monitor_settings_view(self, id, gamma_map)
                    })),
            )
            .into()
    }
}

/// Expanded settings panel for a monitor (gamma, min brightness, sync)
fn monitor_settings_view<'a>(
    app_state: &AppState,
    id: &'a str,
    gamma_map: f32,
) -> Element<'a, AppMsg> {
    let min_brightness = app_state.config.get_min_brightness(id);

    column()
        .spacing(8)
        .push(
            row()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    icon::from_name("preferences-desktop-display-symbolic")
                        .size(16)
                        .symbolic(true)
                )
                .push(text(fl!("brightness_curve")).size(12))
                .push(horizontal_space())
                .push(
                    button::text("-")
                        .padding([2, 8])
                        .on_press(AppMsg::SetMonGammaMap(
                            id.to_string(),
                            (gamma_map - 0.1).max(0.3)
                        ))
                )
                .push(
                    text(format!("{gamma_map:.2}"))
                        .size(16)
                        .width(Length::Fixed(40.0))
                )
                .push(
                    button::text("+")
                        .padding([2, 8])
                        .on_press(AppMsg::SetMonGammaMap(
                            id.to_string(),
                            (gamma_map + 0.1).min(3.0)
                        ))
                )
        )
        .push(
            row()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    icon::from_name("display-brightness-symbolic")
                        .size(16)
                        .symbolic(true)
                )
                .push(text(fl!("minimum_brightness")).size(12))
                .push(horizontal_space())
                .push(slider(
                    0..=100,
                    min_brightness,
                    move |min_val| {
                        AppMsg::SetMonMinBrightness(id.to_string(), min_val)
                    },
                ))
                .push(
                    text(format!("{}%", min_brightness))
                        .size(16)
                        .width(Length::Fixed(35.0)),
                )
        )
        .push(
            row()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    icon::from_name("input-keyboard-symbolic")
                        .size(16)
                        .symbolic(true)
                )
                .push(text(fl!("sync_brightness_keys")).size(12))
                .push(horizontal_space())
                .push(
                    toggler(app_state.config.is_sync_enabled(id))
                        .on_toggle(move |enabled| AppMsg::SetMonitorSyncEnabled(id.to_string(), enabled))
                )
        )
        .into()
}
