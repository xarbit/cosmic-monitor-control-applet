use crate::app::{AppMsg, AppState};
use crate::fl;
use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{button, column};
use cosmic::{cosmic_theme, theme};

impl AppState {
    pub fn quick_settings_view(&self) -> Element<'_, AppMsg> {
        let cosmic_theme::Spacing {
            space_s,
            space_l,
            ..
        } = theme::spacing();

        column()
            .width(Length::Fill)
            .spacing(space_l)
            .padding(space_s)
            .push(button::text(fl!("refresh")).on_press(AppMsg::Refresh))
            .into()
    }
}
