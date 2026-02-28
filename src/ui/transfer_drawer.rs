use iced::widget::{
    button, column, container, mouse_area, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::core::transfer::TransferProgress;
use crate::db::schema::*;
use crate::theme;

const DRAWER_HEIGHT: f32 = 340.0;

pub fn transfer_drawer<'a>(
    destinations: &'a [Destination],
    selected_destination_id: Option<i64>,
    confirmed_count: usize,
    active_transfers: &'a [TransferProgress],
    _show_add_modal: bool,
) -> Element<'a, Message> {
    // Header
    let header = container(
        row![
            text("Transfers").size(16).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            button(text("✕").size(14).color(theme::TEXT_MUTED))
                .padding(Padding::from([2, 8]))
                .style(|_, _| button::Style {
                    background: None,
                    ..Default::default()
                })
                .on_press(Message::ToggleTransferDrawer),
        ]
        .align_y(iced::Alignment::Center)
        .padding(Padding::from([10, 16])),
    )
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    // Left: Destinations list
    let mut dest_items: Vec<Element<'a, Message>> = destinations
        .iter()
        .map(|d| {
            let is_selected = selected_destination_id == Some(d.id);
            let id = d.id;
            let bg = if is_selected {
                theme::ACCENT_DIM
            } else {
                theme::BG_TERTIARY
            };

            let type_label = match d.dest_type {
                DestinationType::Local => "Local",
                DestinationType::Ssh => "SSH",
            };

            button(
                row![
                    text(&d.name).size(13).color(theme::TEXT_PRIMARY),
                    Space::new().width(Length::Fill),
                    text(type_label).size(11).color(theme::TEXT_MUTED),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding(Padding::from([8, 12]))
            .width(Length::Fill)
            .style(move |_, _| button::Style {
                background: Some(bg.into()),
                border: Border {
                    color: if is_selected { theme::ACCENT } else { theme::BORDER },
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .on_press(Message::SelectDestination(id))
            .into()
        })
        .collect();

    dest_items.push(
        button(text("+ Add Destination").size(12).color(theme::ACCENT))
            .padding(Padding::from([6, 12]))
            .width(Length::Fill)
            .style(|_, _| button::Style {
                background: Some(theme::BG_TERTIARY.into()),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .on_press(Message::ShowAddDestination)
            .into(),
    );

    let dest_list = container(
        scrollable(column(dest_items).spacing(6).padding(8))
            .height(Length::Fill),
    )
    .width(280)
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    // Right: Transfer area
    let right_content: Element<'a, Message> = if !active_transfers.is_empty() {
        // Show active transfers
        let transfer_rows: Vec<Element<'a, Message>> = active_transfers
            .iter()
            .map(|tp| {
                let status_icon = match tp.status {
                    crate::core::transfer::TransferStatus::Transferring => "⟳",
                    crate::core::transfer::TransferStatus::Completed => "✓",
                    crate::core::transfer::TransferStatus::Failed => "✗",
                };

                let progress_pct = format!("{:.0}%", tp.progress * 100.0);
                let size_info = format!(
                    "{:.1} MB / {:.1} MB",
                    tp.bytes_transferred as f64 / 1_048_576.0,
                    tp.total_bytes as f64 / 1_048_576.0,
                );

                container(
                    column![
                        row![
                            text(status_icon).size(14),
                            text(format!("Job {}", tp.job_id))
                                .size(12)
                                .color(theme::TEXT_PRIMARY),
                            Space::new().width(Length::Fill),
                            text(progress_pct).size(12).color(theme::ACCENT),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                        progress_bar(0.0..=1.0, tp.progress as f32)
                            .girth(4),
                        text(size_info).size(11).color(theme::TEXT_MUTED),
                    ]
                    .spacing(4),
                )
                .padding(8)
                .style(|_: &Theme| container::Style {
                    background: Some(theme::BG_TERTIARY.into()),
                    border: Border::default().rounded(4),
                    ..Default::default()
                })
                .into()
            })
            .collect();

        scrollable(column(transfer_rows).spacing(6).padding(12))
            .height(Length::Fill)
            .into()
    } else if selected_destination_id.is_none() {
        container(
            text("Select a destination to start transferring")
                .size(14)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else if confirmed_count == 0 {
        container(
            text("No confirmed groups selected for transfer")
                .size(14)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        // Ready to transfer
        container(
            column![
                text(format!("{} groups ready to transfer", confirmed_count))
                    .size(14)
                    .color(theme::TEXT_PRIMARY),
                button(text("Start Transfer").size(14).color(theme::TEXT_PRIMARY))
                    .padding(Padding::from([10, 24]))
                    .style(|_, status| {
                        let bg = match status {
                            button::Status::Hovered => theme::ACCENT_HOVER,
                            _ => theme::ACCENT,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            border: Border::default().rounded(8),
                            ..Default::default()
                        }
                    })
                    .on_press(Message::StartTransfer),
            ]
            .spacing(16)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    };

    container(
        column![
            header,
            row![dest_list, container(right_content).width(Length::Fill)]
                .height(Length::Fixed(DRAWER_HEIGHT - 48.0)),
        ]
        .height(Length::Fixed(DRAWER_HEIGHT)),
    )
    .width(Length::Fill)
    .style(|_: &Theme| container::Style {
        background: Some(theme::BG_SECONDARY.into()),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Add Destination modal overlay.
pub fn add_destination_modal<'a>(
    name: &'a str,
    dest_type: &'a str,
    base_path: &'a str,
    ssh_host: &'a str,
    ssh_port: &'a str,
    ssh_user: &'a str,
    ssh_key_path: &'a str,
    ssh_key_passphrase: &'a str,
    test_result: Option<&'a str>,
) -> Element<'a, Message> {
    let is_ssh = dest_type == "ssh";

    let mut fields = column![
        labeled_input("Name", name, Message::DestFieldChanged("name".into(), name.to_string())),
        row![
            text("Type:").size(13).color(theme::TEXT_SECONDARY),
            button(text("Local").size(12).color(if !is_ssh { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED }))
                .padding(Padding::from([4, 12]))
                .style(move |_, _| button::Style {
                    background: Some(if !is_ssh { theme::ACCENT.into() } else { theme::BG_TERTIARY.into() }),
                    border: Border::default().rounded(4),
                    ..Default::default()
                })
                .on_press(Message::DestFieldChanged("type".into(), "local".into())),
            button(text("SSH").size(12).color(if is_ssh { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED }))
                .padding(Padding::from([4, 12]))
                .style(move |_, _| button::Style {
                    background: Some(if is_ssh { theme::ACCENT.into() } else { theme::BG_TERTIARY.into() }),
                    border: Border::default().rounded(4),
                    ..Default::default()
                })
                .on_press(Message::DestFieldChanged("type".into(), "ssh".into())),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        labeled_input("Base Path", base_path, Message::DestFieldChanged("base_path".into(), base_path.to_string())),
    ]
    .spacing(10);

    if is_ssh {
        fields = fields
            .push(labeled_input("SSH Host", ssh_host, Message::DestFieldChanged("ssh_host".into(), ssh_host.to_string())))
            .push(labeled_input("SSH Port", ssh_port, Message::DestFieldChanged("ssh_port".into(), ssh_port.to_string())))
            .push(labeled_input("Username", ssh_user, Message::DestFieldChanged("ssh_user".into(), ssh_user.to_string())))
            .push(labeled_input("Key Path", ssh_key_path, Message::DestFieldChanged("ssh_key_path".into(), ssh_key_path.to_string())))
            .push(labeled_input("Key Passphrase", ssh_key_passphrase, Message::DestFieldChanged("ssh_key_passphrase".into(), ssh_key_passphrase.to_string())));

        if let Some(result) = test_result {
            fields = fields.push(
                text(result).size(12).color(
                    if result.starts_with("Success") { theme::SUCCESS } else { theme::ERROR }
                ),
            );
        }

        fields = fields.push(
            button(text("Test Connection").size(12).color(theme::TEXT_PRIMARY))
                .padding(Padding::from([6, 14]))
                .style(|_, _| button::Style {
                    background: Some(theme::BG_TERTIARY.into()),
                    border: Border {
                        color: theme::BORDER,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .on_press(Message::TestConnection),
        );
    }

    let footer = row![
        button(text("Cancel").size(13).color(theme::TEXT_PRIMARY))
            .padding(Padding::from([8, 20]))
            .style(|_, _| button::Style {
                background: Some(theme::BG_TERTIARY.into()),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .on_press(Message::HideAddDestination),
        Space::new().width(Length::Fill),
        button(text("Add").size(13).color(theme::TEXT_PRIMARY))
            .padding(Padding::from([8, 20]))
            .style(|_, _| button::Style {
                background: Some(theme::ACCENT.into()),
                border: Border::default().rounded(6),
                ..Default::default()
            })
            .on_press(Message::SaveDestination),
    ]
    .align_y(iced::Alignment::Center);

    // Modal overlay
    let modal = container(
        column![
            text("Add Destination").size(16).color(theme::TEXT_PRIMARY),
            scrollable(fields).height(Length::Fill),
            footer,
        ]
        .spacing(16)
        .padding(24)
        .width(480)
        .height(Length::Shrink),
    )
    .style(|_: &Theme| container::Style {
        background: Some(theme::BG_SECONDARY.into()),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    // Backdrop
    mouse_area(
        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
                ..Default::default()
            }),
    )
    .on_press(Message::HideAddDestination)
    .into()
}

fn labeled_input<'a>(
    label: &str,
    value: &'a str,
    _on_change: Message,
) -> Element<'a, Message> {
    let label = label.to_string();
    let field_name = label.to_lowercase().replace(' ', "_");
    column![
        text(label).size(13).color(theme::TEXT_SECONDARY),
        text_input("", value)
            .on_input(move |v| Message::DestFieldChanged(field_name.clone(), v))
            .size(13)
            .padding(Padding::from([6, 10]))
            .style(|_, _| text_input::Style {
                background: theme::BG_TERTIARY.into(),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                icon: theme::TEXT_MUTED,
                placeholder: theme::TEXT_MUTED,
                value: theme::TEXT_PRIMARY,
                selection: theme::ACCENT,
            }),
    ]
    .spacing(4)
    .into()
}
