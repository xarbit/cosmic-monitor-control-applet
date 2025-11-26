use crate::app::{AppMsg, AppState};
use crate::fl;
use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{button, column};

impl AppState {
    pub fn quick_settings_view(&self) -> Element<AppMsg> {
        column()
            .width(Length::Fill)
            .spacing(20)
            .padding(10)
            .push(button::text(fl!("refresh")).on_press(AppMsg::Refresh))
            .into()
    }
}
