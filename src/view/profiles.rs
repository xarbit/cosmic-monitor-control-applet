use crate::app::{AppMsg, AppState};
use crate::config::MAX_PROFILES;
use crate::fl;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, divider, horizontal_space, icon, row, text, text_input, tooltip,
};
use cosmic::{cosmic_theme, theme};

impl AppState {
    /// View for brightness profiles section
    pub fn profiles_view(&self) -> Option<Element<'_, AppMsg>> {
        if self.monitors.is_empty() {
            return None;
        }

        let cosmic_theme::Spacing {
            space_xxxs,
            space_xxs,
            space_xs,
            space_s,
            ..
        } = theme::spacing();

        let mut col = column()
            .spacing(space_xs)
            .padding(space_xxs);

        debug!("Rendering profiles view: {} saved profiles, dialog_open={}, profiles_expanded={}",
               self.config.profiles.len(), self.profile_dialog_open, self.profiles_expanded);

        let at_max_profiles = self.config.profiles.len() >= MAX_PROFILES;

        // Header with dropdown icon, "Profiles" label and new profile button
        let dropdown_icon = if self.profiles_expanded {
            "go-down-symbolic"
        } else {
            "go-next-symbolic"
        };

        let mut header_row = row()
            .spacing(space_s)
            .align_y(Alignment::Center)
            .push(
                button::icon(icon::from_name(dropdown_icon))
                    .padding(0)
                    .on_press(AppMsg::ToggleProfilesSection)
            )
            .push(
                icon::from_name("folder-documents-symbolic")
                    .size(16)
                    .symbolic(true)
            )
            .push(text(fl!("profiles")).size(12))
            .push(horizontal_space());

        // Add new profile button (disabled if at max)
        if at_max_profiles {
            header_row = header_row.push(
                tooltip(
                    button::icon(icon::from_name("list-add-symbolic"))
                        .padding(space_xxs),
                    text(format!("{} ({}/{})", fl!("max_profiles_reached"), self.config.profiles.len(), MAX_PROFILES)),
                    tooltip::Position::Left,
                )
            );
        } else {
            header_row = header_row.push(
                tooltip(
                    button::icon(icon::from_name("list-add-symbolic"))
                        .padding(space_xxs)
                        .on_press(AppMsg::OpenNewProfileDialog),
                    text(fl!("new_profile")),
                    tooltip::Position::Left,
                )
            );
        }

        col = col.push(header_row);

        // Only show content if expanded
        if !self.profiles_expanded {
            return Some(col.into());
        }

        // Profile creation/edit dialog
        if self.profile_dialog_open {
            col = col.push(
                container(
                    column()
                        .spacing(space_xs)
                        .push(
                            text(if self.editing_profile.is_some() {
                                fl!("save_profile")
                            } else {
                                fl!("new_profile")
                            })
                            .size(14)
                        )
                        .push(divider::horizontal::default())
                        .push(
                            text_input(fl!("profile_name"), &self.profile_name_input)
                                .on_input(AppMsg::ProfileNameInput)
                        )
                        .push(
                            row()
                                .spacing(space_s)
                                .push(horizontal_space())
                                .push(
                                    button::text(fl!("cancel"))
                                        .padding([space_xxxs, space_xs])
                                        .on_press(AppMsg::CancelProfileDialog)
                                )
                                .push(
                                    button::text(fl!("save"))
                                        .padding([space_xxxs, space_xs])
                                        .on_press(AppMsg::SaveProfileConfirm)
                                        .class(cosmic::theme::Button::Suggested)
                                )
                        )
                )
                .padding(space_xs)
                .class(cosmic::style::Container::Card)
            );
        }

        // List of saved profiles
        if !self.config.profiles.is_empty() {
            let mut profiles_list = column().spacing(space_xxxs);

            for profile in &self.config.profiles {
                // Main profile row with icon buttons on the LEFT
                profiles_list = profiles_list.push(
                    row()
                        .spacing(space_xs)
                        .align_y(Alignment::Center)
                        .push(
                            button::icon(icon::from_name("document-edit-symbolic"))
                                .padding(space_xxs)
                                .on_press(AppMsg::OpenEditProfileDialog(profile.name.clone()))
                        )
                        .push(
                            button::icon(icon::from_name("edit-delete-symbolic"))
                                .padding(space_xxs)
                                .on_press(AppMsg::DeleteProfile(profile.name.clone()))
                        )
                        .push(
                            button::text(&profile.name)
                                .padding([space_xxxs, space_xs])
                                .width(Length::Fill)
                                .on_press(AppMsg::LoadProfile(profile.name.clone()))
                        )
                );
            }

            col = col.push(
                container(profiles_list)
                    .padding(space_xs)
                    .class(cosmic::style::Container::Card)
            );
        }

        Some(col.into())
    }
}
