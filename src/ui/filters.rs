use iced::widget::{button, container, pick_list, row, text, text_input, Space};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::db::schema::{GroupStatus, MediaType};
use crate::theme;

pub fn filters_bar(
    search_query: &str,
    status_filter: Option<GroupStatus>,
    media_type_filter: Option<MediaType>,
    selected_count: usize,
) -> Element<'_, Message> {
    let search = text_input("Search groups...", search_query)
        .on_input(Message::SearchChanged)
        .width(264)
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
        });

    // Status filter
    let status_options: Vec<String> = std::iter::once("All Statuses".to_string())
        .chain(GroupStatus::ALL.iter().map(|s| s.as_str().to_string()))
        .collect();
    let selected_status = status_filter
        .map(|s| s.as_str().to_string())
        .unwrap_or_else(|| "All Statuses".to_string());

    let status_pick = pick_list(status_options, Some(selected_status), |val| {
        if val == "All Statuses" {
            Message::StatusFilterChanged(None)
        } else {
            Message::StatusFilterChanged(Some(GroupStatus::from_str(&val)))
        }
    })
    .text_size(13)
    .padding(Padding::from([4, 8]));

    // Media type filter
    let type_options: Vec<String> = std::iter::once("All Types".to_string())
        .chain(MediaType::ALL.iter().map(|t| t.as_str().to_string()))
        .collect();
    let selected_type = media_type_filter
        .map(|t| t.as_str().to_string())
        .unwrap_or_else(|| "All Types".to_string());

    let type_pick = pick_list(type_options, Some(selected_type), |val| {
        if val == "All Types" {
            Message::MediaTypeFilterChanged(None)
        } else {
            Message::MediaTypeFilterChanged(Some(MediaType::from_str(&val)))
        }
    })
    .text_size(13)
    .padding(Padding::from([4, 8]));

    let left = row![search, status_pick, type_pick].spacing(8).align_y(iced::Alignment::Center);

    // Bulk actions (visible when groups are selected)
    let right = if selected_count > 0 {
        row![
            bulk_btn("Confirm", Message::BulkAction("confirm".to_string())),
            bulk_btn("Skip", Message::BulkAction("skip".to_string())),
            bulk_btn("Rematch", Message::BulkAction("rematch".to_string())),
            bulk_btn("Delete", Message::BulkAction("delete".to_string())),
            bulk_btn("Clear", Message::ClearSelection),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    } else {
        row![]
    };

    container(
        row![left, Space::new().width(Length::Fill), right]
            .align_y(iced::Alignment::Center)
            .padding(Padding::from([8, 20])),
    )
    .width(Length::Fill)
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn bulk_btn(label: &str, msg: Message) -> Element<'static, Message> {
    let label = label.to_string();
    button(text(label).size(12).color(theme::TEXT_PRIMARY))
        .padding(Padding::from([4, 10]))
        .style(|_, status| {
            let bg = match status {
                button::Status::Hovered => theme::BG_HOVER,
                _ => theme::BG_TERTIARY,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(msg)
        .into()
}
