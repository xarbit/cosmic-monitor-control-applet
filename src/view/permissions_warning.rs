use crate::app::AppMsg;
use crate::fl;
use crate::permissions::{PermissionCheckResult, RequirementStatus};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{column, container, icon, row, text};
use cosmic::{cosmic_theme, theme};

/// Permissions warning view showing detailed requirements with checkmarks/X marks
pub fn permissions_warning_view(result: &PermissionCheckResult) -> Element<'_, AppMsg> {
    let cosmic_theme::Spacing {
        space_xxxs,
        space_xs,
        space_s,
        space_m,
        space_l,
        ..
    } = theme::spacing();

    let mut requirements_column = column().spacing(space_xs);

    for req in result.requirements.clone() {
        let status_icon = match req.status {
            RequirementStatus::Met => "checkbox-checked-symbolic",
            RequirementStatus::NotMet => "window-close-symbolic",
            RequirementStatus::NotApplicable => "view-more-symbolic",
            RequirementStatus::Partial => "dialog-information-symbolic",
        };

        requirements_column = requirements_column.push(
            row()
                .spacing(space_s)
                .align_y(Alignment::Center)
                .push(
                    icon::from_name(status_icon)
                        .size(16)
                        .symbolic(true)
                )
                .push(
                    column()
                        .spacing(space_xxxs)
                        .push(
                            text(req.name)
                                .size(13)
                        )
                        .push(
                            text(req.description)
                                .size(11)
                        )
                )
        );
    }

    container(
        column()
            .spacing(space_m)
            .align_x(Alignment::Start)
            .push(
                row()
                    .spacing(space_s)
                    .align_y(Alignment::Center)
                    .push(
                        icon::from_name(if result.has_issues() {
                            "dialog-warning-symbolic"
                        } else {
                            "emblem-ok-symbolic"
                        })
                        .size(48)
                        .symbolic(true)
                    )
                    .push(
                        column()
                            .spacing(space_xxxs)
                            .push(
                                text(if result.has_issues() {
                                    fl!("permission_warning_title")
                                } else {
                                    "Hardware Access OK".to_string()
                                })
                                .size(16)
                            )
                            .push(
                                text(result.summary())
                                    .size(12)
                            )
                    )
            )
            .push(requirements_column)
            .push(
                text(fl!("permission_warning_hint"))
                    .size(11)
            )
    )
    .width(Length::Fill)
    .padding([space_l, space_l])
    .into()
}
