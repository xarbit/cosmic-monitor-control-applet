use cosmic::app::Task;
use cosmic::iced::{Limits, window};
use cosmic::iced_winit::commands::popup::get_popup;

use super::messages::AppMsg;
use super::state::AppState;

#[derive(Debug, Clone)]
pub struct Popup {
    pub kind: PopupKind,
    pub id: window::Id,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PopupKind {
    Popup,
    QuickSettings,
}

impl AppState {
    pub fn toggle_popup(&mut self, kind: PopupKind) -> Task<AppMsg> {
        match &self.popup {
            Some(popup) => {
                if popup.kind == kind {
                    self.close_popup()
                } else {
                    Task::batch(vec![self.close_popup(), self.open_popup(kind)])
                }
            }
            None => self.open_popup(kind),
        }
    }

    pub fn open_popup(&mut self, kind: PopupKind) -> Task<AppMsg> {
        // handle the case where the popup was closed by clicking the icon
        if self.should_suppress_popup(kind) {
            return Task::none();
        }

        let new_id = window::Id::unique();

        let popup = Popup { kind, id: new_id };
        self.popup.replace(popup);

        match kind {
            PopupKind::Popup => {
                let mut popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    None,
                    None,
                    None,
                );

                // Let popup size naturally to content
                popup_settings.positioner.size_limits = Limits::NONE;

                // No fixed size - will auto-size to content
                popup_settings.positioner.size = None;

                // Don't trigger re-enumeration on popup open - makes it feel slow
                // User can click refresh button if needed for hotplug detection
                get_popup(popup_settings)
            }
            PopupKind::QuickSettings => {
                let mut popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    None,
                    None,
                    None,
                );

                popup_settings.positioner.size_limits = Limits::NONE
                    .min_width(200.0)
                    .max_width(250.0)
                    .min_height(200.0)
                    .max_height(550.0);

                get_popup(popup_settings)
            }
        }
    }
}
