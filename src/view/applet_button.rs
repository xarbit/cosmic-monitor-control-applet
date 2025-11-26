use crate::app::{AppMsg, AppState};
use cosmic::Element;

use super::common::brightness_icon;
use crate::icon::icon_off;

impl AppState {
    pub fn applet_button_view(&self) -> Element<AppMsg> {
        self.core
            .applet
            .icon_button_from_handle(
                self.monitors
                    .values()
                    .next()
                    .map(|m| brightness_icon(m.slider_brightness))
                    .unwrap_or(icon_off()),
            )
            .on_press(AppMsg::TogglePopup)
            .into()
    }
}
