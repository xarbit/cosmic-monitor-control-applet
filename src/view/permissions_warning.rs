use crate::app::AppMsg;
use crate::fl;
use crate::permissions::{PermissionCheckResult, RequirementStatus};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{column, container, icon, row, text};

/// Permissions warning view showing detailed requirements with checkmarks/X marks
pub fn permissions_warning_view(result: &PermissionCheckResult) -> Element<AppMsg> {
    let mut requirements_column = column().spacing(8);

    for req in result.requirements.clone() {
        let status_icon = match req.status {
            RequirementStatus::Met => "checkbox-checked-symbolic",
            RequirementStatus::NotMet => "window-close-symbolic",
            RequirementStatus::NotApplicable => "view-more-symbolic",
            RequirementStatus::Partial => "dialog-information-symbolic",
        };

        requirements_column = requirements_column.push(
            row()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    icon::from_name(status_icon)
                        .size(16)
                        .symbolic(true)
                )
                .push(
                    column()
                        .spacing(2)
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
            .spacing(16)
            .align_x(Alignment::Start)
            .push(
                row()
                    .spacing(12)
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
                            .spacing(4)
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
    .padding([20, 20])
    .into()
}
