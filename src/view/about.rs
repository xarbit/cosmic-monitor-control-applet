use crate::app::{AppMsg, AppState};
use crate::fl;
use cosmic::Element;
use cosmic::applet::padded_control;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, divider, horizontal_space, icon, row, text, Space};
use cosmic::{cosmic_theme, theme};

impl AppState {
    /// View for the about page
    pub fn about_view(&self) -> Element<'_, AppMsg> {
        let cosmic_theme::Spacing {
            space_xxs,
            space_xs,
            space_s,
            space_m,
            ..
        } = theme::spacing();

        let col = column().spacing(0);
        let content = column().padding(space_s);

        col.push(content
            .push(
                column()
                    .spacing(space_s)
                    // Header with icon and title
                    .push(
                        row()
                            .align_y(Alignment::Center)
                            .spacing(space_m)
                            .push(icon::from_name("help-about-symbolic").size(64))
                            .push(
                                column()
                                    .spacing(space_xxs)
                                    .push(text("External Monitor Brightness").size(20))
                                    .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14))
                                    .push(Space::with_height(space_xxs))
                                    .push(text("Control external monitor brightness via DDC/CI and Apple HID protocols").size(11))
                            )
                    )
                    .push(Space::with_height(space_xs))
                    // Creator/Developer card
                    .push(
                        container(
                            column()
                                .spacing(space_xxs)
                                .push(text("Jason Scurtu (xarbit)").size(12))
                                .push(
                                    button::link("https://github.com/xarbit")
                                        .on_press(AppMsg::OpenUrl("https://github.com/xarbit".to_string()))
                                        .padding(0)
                                )
                        )
                        .padding(space_xs)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card)
                    )
                    .push(Space::with_height(space_xs))
                    .push(divider::horizontal::default())
                    .push(Space::with_height(space_xs))
                    .push(
                        row()
                            .spacing(space_xs)
                            .align_y(Alignment::Center)
                            .push(icon::from_name("starred-symbolic").size(16))
                            .push(text("Credits/Acknowledgements").size(14))
                    )
                    .push(Space::with_height(space_xxs))
                    // Based on card
                    .push(
                        container(
                            column()
                                .spacing(space_xxs)
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .align_y(Alignment::Center)
                                        .push(icon::from_name("folder-symbolic").size(16))
                                        .push(text("Based on").size(13))
                                )
                                .push(text("cosmic-ext-applet-external-monitor-brightness").size(12))
                                .push(text("by maciekk64").size(11))
                                .push(
                                    button::link("https://github.com/cosmic-utils/cosmic-ext-applet-external-monitor-brightness")
                                        .on_press(AppMsg::OpenUrl("https://github.com/cosmic-utils/cosmic-ext-applet-external-monitor-brightness".to_string()))
                                        .padding(0)
                                )
                                .push(Space::with_height(space_xxs))
                                .push(
                                    row()
                                        .spacing(space_xxs)
                                        .push(icon::from_name("emblem-documents-symbolic").size(12))
                                        .push(text("GPL-3.0-only").size(10))
                                )
                        )
                        .padding(space_xs)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card)
                    )
                    // Apple HID card
                    .push(
                        container(
                            column()
                                .spacing(space_xxs)
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .align_y(Alignment::Center)
                                        .push(icon::from_name("computer-symbolic").size(16))
                                        .push(text("Apple HID Protocol").size(13))
                                )
                                .push(text("Implementation based on asdbctl").size(11))
                                .push(text("by juliuszint").size(11))
                                .push(
                                    button::link("https://github.com/juliuszint/asdbctl")
                                        .on_press(AppMsg::OpenUrl("https://github.com/juliuszint/asdbctl".to_string()))
                                        .padding(0)
                                )
                                .push(Space::with_height(space_xxs))
                                .push(
                                    row()
                                        .spacing(space_xxs)
                                        .push(icon::from_name("emblem-documents-symbolic").size(12))
                                        .push(text("MIT License").size(10))
                                )
                        )
                        .padding(space_xs)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card)
                    )
                    // Dependencies card
                    .push(
                        container(
                            column()
                                .spacing(space_xxs)
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .align_y(Alignment::Center)
                                        .push(icon::from_name("package-symbolic").size(16))
                                        .push(text("Key Dependencies").size(13))
                                )
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .push(text("•").size(10))
                                        .push(text("ddc-hi 0.4.1").size(11))
                                        .push(text("-").size(10))
                                        .push(text("DDC/CI protocol (MIT/Apache-2.0)").size(10))
                                )
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .push(text("•").size(10))
                                        .push(text("hidapi 2.6").size(11))
                                        .push(text("-").size(10))
                                        .push(text("USB HID support (MIT/Apache-2.0)").size(10))
                                )
                                .push(
                                    row()
                                        .spacing(space_xs)
                                        .push(text("•").size(10))
                                        .push(text("libcosmic").size(11))
                                        .push(text("-").size(10))
                                        .push(text("COSMIC Desktop toolkit (MPL-2.0)").size(10))
                                )
                        )
                        .padding(space_xs)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card)
                    )
                    // Footer info
                    .push(Space::with_height(space_xs))
                    .push(
                        column()
                            .spacing(space_xxs)
                            .push(
                                row()
                                    .spacing(space_xs)
                                    .push(icon::from_name("emblem-documents-symbolic").size(12))
                                    .push(text("License:").size(11))
                                    .push(text(env!("CARGO_PKG_LICENSE")).size(11))
                            )
                            .push(
                                row()
                                    .spacing(space_xs)
                                    .align_y(Alignment::Center)
                                    .push(icon::from_name("folder-symbolic").size(12))
                                    .push(text("Repository:").size(11))
                                    .push(
                                        button::link(env!("CARGO_PKG_REPOSITORY"))
                                            .on_press(AppMsg::OpenUrl(env!("CARGO_PKG_REPOSITORY").to_string()))
                                            .padding(0)
                                    )
                            )
                    )
            )
            .push(padded_control(divider::horizontal::default()))
            .push(padded_control(
                row()
                    .align_y(Alignment::Center)
                    .push(text(fl!("close")))
                    .push(horizontal_space())
                    .push(
                        button::icon(icon::from_name("window-close-symbolic"))
                            .on_press(AppMsg::ToggleAboutView)
                    )
            ))
        )
        .into()
    }
}
