use crate::app::{AppMsg, AppState, MonitorState};
use crate::fl;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, horizontal_space, icon, mouse_area, row, slider, text, toggler,
    tooltip,
};
use cosmic::{cosmic_theme, theme};

use super::common::brightness_icon;

/// Format display name with connector if available
fn format_display_name(name: &str, connector: &Option<String>) -> String {
    match connector {
        Some(conn) => format!("{} ({})", name, conn),
        None => name.to_string(),
    }
}

impl AppState {
    /// View for a list of all monitors
    pub fn monitors_view(&self) -> Option<Element<'_, AppMsg>> {
        let cosmic_theme::Spacing {
            space_xs,
            space_s,
            ..
        } = theme::spacing();

        (!self.monitors.is_empty()).then(|| {
            let mut monitors: Vec<_> = self.monitors.iter().collect();
            monitors.sort_by_key(|(id, _)| *id);

            column()
                .padding(space_xs)
                .spacing(space_s)
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
        let cosmic_theme::Spacing {
            space_xxxs,
            space_xxs,
            space_xs,
            space_s,
            ..
        } = theme::spacing();

        let gamma_map = self.config.get_gamma_map(id);

        column()
            .spacing(space_xs)
            .padding(space_xxs)
            .push(
                // Header row with icon, name, and settings cog
                row()
                    .spacing(space_xs)
                    .align_y(Alignment::Center)
                    .push(
                        mouse_area(
                            icon::icon(brightness_icon(monitor.slider_brightness))
                                .size(20)
                        )
                        .on_press(AppMsg::ToggleMinMaxBrightness(id.to_string()))
                    )
                    .push(
                        column()
                            .spacing(space_xxxs)
                            .push(
                                text(format_display_name(&monitor.name, &monitor.connector_name))
                                    .size(12)
                            )
                            .push(
                                text(id)
                                    .size(9)
                                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6)))
                            )
                    )
                    .push(horizontal_space())
                    .push(
                        button::icon(icon::from_name("emblem-system-symbolic"))
                            .padding(space_xxs)
                            .on_press(AppMsg::ToggleMonSettings(id.to_string()))
                    )
            )
            .push(
                // Brightness slider row
                row()
                    .spacing(space_s)
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
            }))
            .into()
    }
}

/// Expanded settings panel for a monitor (gamma, min brightness, sync)
fn monitor_settings_view<'a>(
    app_state: &AppState,
    id: &'a str,
    gamma_map: f32,
) -> Element<'a, AppMsg> {
    let cosmic_theme::Spacing {
        space_xxxs,
        space_xs,
        space_s,
        ..
    } = theme::spacing();

    let min_brightness = app_state.config.get_min_brightness(id);

    container(
        column()
            .spacing(space_xs)
            .push(
                // Brightness Curve (Gamma) Setting
                tooltip(
                    row()
                        .spacing(space_s)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("preferences-desktop-display-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
                        .push(horizontal_space())
                        .push(
                            button::text("-")
                                .padding([space_xxxs, space_xs])
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
                                .padding([space_xxxs, space_xs])
                                .on_press(AppMsg::SetMonGammaMap(
                                    id.to_string(),
                                    (gamma_map + 0.1).min(3.0)
                                ))
                        )
                        .push(horizontal_space()),
                    text(fl!("brightness_curve")),
                    tooltip::Position::Top,
                )
            )
            .push(
                // Minimum Brightness Setting
                tooltip(
                    row()
                        .spacing(space_s)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("display-brightness-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
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
                        ),
                    text(fl!("minimum_brightness")),
                    tooltip::Position::Top,
                )
            )
            .push(
                // Sync with Brightness Keys Setting
                tooltip(
                    row()
                        .spacing(space_s)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("input-keyboard-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
                        .push(horizontal_space())
                        .push(
                            toggler(app_state.config.is_sync_enabled(id))
                                .on_toggle(move |enabled| AppMsg::SetMonitorSyncEnabled(id.to_string(), enabled))
                        ),
                    text(fl!("sync_brightness_keys")),
                    tooltip::Position::Top,
                )
            )
    )
    .padding(12)
    .class(cosmic::style::Container::Card)
    .into()
}
