use crate::app::{AppMsg, AppState};
use crate::fl;
use cosmic::Element;
use cosmic::applet::padded_control;
use cosmic::iced::Alignment;
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, text, tooltip, Space};
use cosmic::{cosmic_theme, theme};

use super::empty_state::empty_state_view;
use super::permissions_warning::permissions_warning_view;

impl AppState {
    pub fn popup_view(&self) -> Element<'_, AppMsg> {
        let cosmic_theme::Spacing {
            space_s,
            space_m,
            space_l,
            ..
        } = theme::spacing();

        let mut col = column().spacing(0);

        // Top bar with rescan button in top-right corner (only in normal view)
        if !self.show_permission_view {
            col = col
                .push(Space::with_height(space_m))
                .push(
                    row()
                        .align_y(Alignment::Center)
                        .push(Space::with_width(space_l))
                        .push(horizontal_space())
                        .push(
                            tooltip(
                                button::icon(icon::from_name("view-refresh-symbolic"))
                                    .on_press(AppMsg::RefreshMonitors),
                                text(fl!("refresh_monitors")),
                                tooltip::Position::Bottom,
                            )
                        )
                        .push(Space::with_width(space_l))
                );
        }

        // Content area
        let mut content = column().padding(space_s);

        // If user toggled to permission view, show it
        if self.show_permission_view {
            if let Some(perm_result) = &self.permission_status {
                return col
                    .push(content
                        .push(permissions_warning_view(perm_result))
                        .push(padded_control(divider::horizontal::default()))
                        .push(padded_control(
                            row()
                                .align_y(Alignment::Center)
                                .push(text(fl!("close")))
                                .push(horizontal_space())
                                .push(
                                    button::icon(icon::from_name("window-close-symbolic"))
                                        .on_press(AppMsg::TogglePermissionView)
                                )
                        ))
                    )
                    .into();
            }
        }

        // Show permission warning if there are BLOCKING issues
        if let Some(perm_result) = &self.permission_status {
            if perm_result.has_issues() {
                return col
                    .push(content
                        .push(permissions_warning_view(perm_result))
                        .push(padded_control(divider::horizontal::default()))
                        .push(self.dark_mode_view())
                    )
                    .into();
            }
        }

        // Normal view (monitors or empty state)
        content = content
            .push_maybe(self.monitors_view())
            .push_maybe(
                self.monitors.is_empty().then(|| empty_state_view()),
            )
            .push_maybe(
                (!self.monitors.is_empty()).then(|| padded_control(divider::horizontal::default())),
            );

        // Add profiles section if there are monitors
        content = content.push_maybe(self.profiles_view());

        col.push(content
            .push_maybe(
                (!self.monitors.is_empty()).then(|| padded_control(divider::horizontal::default())),
            )
            .push(padded_control(
                row()
                    .align_y(Alignment::Center)
                    .push(text(fl!("permissions")))
                    .push(horizontal_space())
                    .push(
                        button::icon(icon::from_name("security-medium-symbolic"))
                            .on_press(AppMsg::TogglePermissionView)
                    )
            ))
            .push(padded_control(divider::horizontal::default()))
            .push(self.dark_mode_view())
        )
        .into()
    }
}
