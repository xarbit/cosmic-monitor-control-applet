use crate::app::AppMsg;
use crate::fl;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{column, container, icon, text};

/// Empty state view shown when no displays are connected
pub fn empty_state_view() -> Element<'static, AppMsg> {
    container(
        column()
            .spacing(12)
            .align_x(Alignment::Center)
            .push(
                icon::from_name("video-display-symbolic")
                    .size(64)
                    .symbolic(true)
            )
            .push(
                text(fl!("no_displays"))
                    .size(14)
            )
            .push(
                text(fl!("no_displays_hint"))
                    .size(12)
            )
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding([40, 20])
    .into()
}
