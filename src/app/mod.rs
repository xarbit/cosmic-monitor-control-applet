mod state;
mod messages;
mod popup;
mod update;

pub use state::{AppState, MonitorState, get_mapped_brightness};
pub use messages::AppMsg;
pub use popup::PopupKind;

use cosmic::app::{Core, Task};
use cosmic::cosmic_config::Config as CosmicConfig;
use cosmic::cosmic_theme::THEME_MODE_ID;
use cosmic::iced::{Subscription, window};
use cosmic::iced_runtime;
use cosmic::widget::Space;
use cosmic::Element;

use crate::config;

pub const APPID: &str = "io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness";

impl cosmic::Application for AppState {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = (Option<CosmicConfig>, config::Config);
    type Message = AppMsg;
    const APP_ID: &'static str = APPID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = AppState::new(
            core,
            flags.0.expect("need to be able to write config"),
            flags.1,
        );

        // Spawn brightness sync daemon if external displays are detected
        #[cfg(feature = "brightness-sync-daemon")]
        {
            tokio::spawn(async {
                crate::daemon::spawn_if_needed().await;
            });
        }

        (window, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<AppMsg> {
        debug!("on_close_requested");

        if let Some(popup) = &self.popup {
            if popup.id == id {
                return Some(AppMsg::ClosePopup);
            }
        }
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        self.update(message)
    }

    fn view(&self) -> Element<Self::Message> {
        self.applet_button_view()
    }

    fn view_window(&self, _id: window::Id) -> Element<Self::Message> {
        let Some(popup) = &self.popup else {
            return Space::new(0, 0).into();
        };

        let view = match &popup.kind {
            PopupKind::Popup => self.popup_view(),
            PopupKind::QuickSettings => self.quick_settings_view(),
        };

        self.core.applet.popup_container(view).into()
    }

    fn style(&self) -> Option<iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs = vec![
            self.core
                .watch_config(THEME_MODE_ID)
                .map(|u| AppMsg::ThemeModeConfigChanged(u.config)),
            Subscription::run(crate::monitor::sub),
            Subscription::run(crate::hotplug::hotplug_subscription),
            config::sub(),
        ];

        // Add UI sync subscription when daemon feature is enabled
        #[cfg(feature = "brightness-sync-daemon")]
        subs.push(Subscription::run(crate::ui_sync::sub));

        Subscription::batch(subs)
    }
}
