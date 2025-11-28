use crate::app::{AppMsg, AppState, MonitorState};
use crate::fl;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, horizontal_space, icon, mouse_area, row, slider, text,
    toggler, tooltip,
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

            // Sort monitors by X position (left to right), falling back to ID if no position available
            monitors.sort_by(|(id_a, mon_a), (id_b, mon_b)| {
                let x_a = mon_a.output_info.as_ref().map(|info| info.position.0).unwrap_or(i32::MAX);
                let x_b = mon_b.output_info.as_ref().map(|info| info.position.0).unwrap_or(i32::MAX);

                // Sort by X position first, then by ID as tiebreaker
                x_a.cmp(&x_b).then_with(|| id_a.cmp(id_b))
            });

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
                        button::icon(icon::from_name("dialog-information-symbolic"))
                            .padding(space_xxs)
                            .on_press(AppMsg::ToggleMonInfo(id.to_string()))
                    )
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
            .push_maybe(monitor.info_expanded.then(|| {
                monitor_info_view(self, id, monitor)
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

    let mut settings_column = column()
            .spacing(space_xs);

    settings_column = settings_column.push(
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
            );
    settings_column = settings_column.push(
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
            );
    settings_column = settings_column.push(
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
            );

    // Add display configuration section if output_info is available
    if let Some(monitor) = app_state.monitors.get(id) {
        if let Some(ref output_info) = monitor.output_info {
            // Display Configuration Header
            settings_column = settings_column.push(
                row()
                    .spacing(space_s)
                    .push(text("Display Configuration").size(12))
            );

            // Rotation/Transform buttons
            let current_transform = &output_info.transform;
            settings_column = settings_column.push(
                tooltip(
                    row()
                        .spacing(space_xs)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("object-rotate-right-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
                        .push(horizontal_space())
                        .push(
                            button::text(if current_transform == "normal" { "▶ ↑" } else { "↑" })
                                .padding([space_xxxs, space_xs])
                                .on_press(AppMsg::SetMonTransform(id.to_string(), "normal".to_string()))
                        )
                        .push(
                            button::text(if current_transform == "90" { "▶ →" } else { "→" })
                                .padding([space_xxxs, space_xs])
                                .on_press(AppMsg::SetMonTransform(id.to_string(), "90".to_string()))
                        )
                        .push(
                            button::text(if current_transform == "180" { "▶ ↓" } else { "↓" })
                                .padding([space_xxxs, space_xs])
                                .on_press(AppMsg::SetMonTransform(id.to_string(), "180".to_string()))
                        )
                        .push(
                            button::text(if current_transform == "270" { "▶ ←" } else { "←" })
                                .padding([space_xxxs, space_xs])
                                .on_press(AppMsg::SetMonTransform(id.to_string(), "270".to_string()))
                        )
                        .push(horizontal_space()),
                    text(format!("Rotation ({})", current_transform)),
                    tooltip::Position::Top,
                )
            );

            // Scale control
            let current_scale = output_info.scale;
            let scale_options = vec![1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
            settings_column = settings_column.push(
                tooltip(
                    row()
                        .spacing(space_s)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("zoom-in-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
                        .push(horizontal_space())
                        .push(
                            button::text("-")
                                .padding([space_xxxs, space_xs])
                                .on_press_maybe({
                                    let current_idx = scale_options.iter().position(|&s| (s - current_scale).abs() < 0.01);
                                    current_idx.and_then(|idx| {
                                        if idx > 0 {
                                            Some(AppMsg::SetMonScale(id.to_string(), scale_options[idx - 1]))
                                        } else {
                                            None
                                        }
                                    })
                                })
                        )
                        .push(
                            text(format!("{:.2}×", current_scale))
                                .size(16)
                                .width(Length::Fixed(50.0))
                        )
                        .push(
                            button::text("+")
                                .padding([space_xxxs, space_xs])
                                .on_press_maybe({
                                    let current_idx = scale_options.iter().position(|&s| (s - current_scale).abs() < 0.01);
                                    current_idx.and_then(|idx| {
                                        if idx < scale_options.len() - 1 {
                                            Some(AppMsg::SetMonScale(id.to_string(), scale_options[idx + 1]))
                                        } else {
                                            None
                                        }
                                    })
                                })
                        )
                        .push(horizontal_space()),
                    text("Scale"),
                    tooltip::Position::Top,
                )
            );

            // Position controls
            let (pos_x, pos_y) = output_info.position;
            settings_column = settings_column.push(
                tooltip(
                    row()
                        .spacing(space_s)
                        .align_y(Alignment::Center)
                        .push(
                            icon::from_name("preferences-desktop-display-symbolic")
                                .size(16)
                                .symbolic(true)
                        )
                        .push(text("X:").size(12))
                        .push(
                            text(format!("{}", pos_x))
                                .size(16)
                                .width(Length::Fixed(50.0))
                        )
                        .push(text("Y:").size(12))
                        .push(
                            text(format!("{}", pos_y))
                                .size(16)
                                .width(Length::Fixed(50.0))
                        )
                        .push(horizontal_space()),
                    text("Position (read-only)"),
                    tooltip::Position::Top,
                )
            );
        }
    }

    container(settings_column)
        .padding(12)
        .class(cosmic::style::Container::Card)
        .into()
}

/// Monitor information view showing all display details
fn monitor_info_view<'a>(
    _app_state: &AppState,
    id: &'a str,
    monitor: &'a MonitorState,
) -> Element<'a, AppMsg> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xs,
        ..
    } = theme::spacing();

    let mut info_column = column().spacing(space_xs);

    // Display Name
    info_column = info_column.push(
        row()
            .spacing(space_xs)
            .push(text("Display Name:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
            .push(text(&monitor.name).size(11))
    );

    // Display ID
    info_column = info_column.push(
        row()
            .spacing(space_xs)
            .push(text("Display ID:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
            .push(text(id).size(11))
    );

    // Connector
    if let Some(ref connector) = monitor.connector_name {
        info_column = info_column.push(
            row()
                .spacing(space_xs)
                .push(text("Connector:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                .push(text(connector).size(11))
        );
    }

    // Output info from cosmic-randr (if available)
    if let Some(ref output_info) = monitor.output_info {
        // Manufacturer
        if let Some(ref make) = output_info.make {
            info_column = info_column.push(
                row()
                    .spacing(space_xs)
                    .push(text("Manufacturer:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                    .push(text(make).size(11))
            );
        }

        // Serial Number
        if let Some(ref serial) = output_info.serial_number {
            info_column = info_column.push(
                row()
                    .spacing(space_xs)
                    .push(text("Serial Number:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                    .push(text(serial).size(11))
            );
        }

        // Physical Size
        let (width_mm, height_mm) = output_info.physical_size;
        if width_mm > 0 && height_mm > 0 {
            let diagonal_mm = ((width_mm.pow(2) + height_mm.pow(2)) as f64).sqrt();
            let diagonal_inch = diagonal_mm / 25.4;
            info_column = info_column.push(
                row()
                    .spacing(space_xs)
                    .push(text("Physical Size:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                    .push(text(format!("{}mm × {}mm ({:.1}\")", width_mm, height_mm, diagonal_inch)).size(11))
            );
        }

        // Resolution and Refresh Rate
        if let Some(ref mode) = output_info.current_mode {
            let refresh_hz = mode.refresh_rate as f64 / 1000.0;
            info_column = info_column.push(
                row()
                    .spacing(space_xs)
                    .push(text("Resolution:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                    .push(text(format!("{} × {} @ {:.0}Hz", mode.width, mode.height, refresh_hz)).size(11))
            );
        }

        // Scale
        info_column = info_column.push(
            row()
                .spacing(space_xs)
                .push(text("Scale:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                .push(text(format!("{:.2}×", output_info.scale)).size(11))
        );

        // Transform/Rotation
        info_column = info_column.push(
            row()
                .spacing(space_xs)
                .push(text("Rotation:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                .push(text(&output_info.transform).size(11))
        );

        // Position
        let (x, y) = output_info.position;
        info_column = info_column.push(
            row()
                .spacing(space_xs)
                .push(text("Position:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                .push(text(format!("({}, {})", x, y)).size(11))
        );

        // Enabled status
        info_column = info_column.push(
            row()
                .spacing(space_xs)
                .push(text("Status:").size(11).class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6))))
                .push(text(if output_info.enabled { "Enabled" } else { "Disabled" }).size(11))
        );
    } else {
        // No cosmic-randr info available
        info_column = info_column.push(
            text("(cosmic-randr information not available)")
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6)))
        );
    }

    container(info_column)
        .padding(space_xxs)
        .class(cosmic::style::Container::Card)
        .into()
}
