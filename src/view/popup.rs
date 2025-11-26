use crate::app::{AppMsg, AppState};
use crate::fl;
use cosmic::Element;
use cosmic::applet::padded_control;
use cosmic::iced::Alignment;
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, text};

use super::empty_state::empty_state_view;

impl AppState {
    pub fn popup_view(&self) -> Element<AppMsg> {
        column()
            .padding(10)
            .push_maybe(self.monitors_view())
            .push_maybe(
                self.monitors.is_empty().then(|| empty_state_view()),
            )
            .push_maybe(
                (!self.monitors.is_empty()).then(|| padded_control(divider::horizontal::default())),
            )
            .push(self.dark_mode_view())
            .push(padded_control(divider::horizontal::default()))
            .push(padded_control(
                row()
                    .align_y(Alignment::Center)
                    .push(text(fl!("refresh_monitors")))
                    .push(horizontal_space())
                    .push(
                        button::icon(icon::from_name("view-refresh-symbolic"))
                            .on_press(AppMsg::RefreshMonitors)
                    )
            ))
            .into()
    }
}
