use iced::widget::{button, container, row, text};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::theme;

const PER_PAGE: i64 = 50;

pub fn pagination_bar(page: i64, total_groups: i64) -> Element<'static, Message> {
    let total_pages = ((total_groups as f64) / PER_PAGE as f64).ceil() as i64;

    if total_pages <= 1 {
        return container(column![]).width(0).height(0).into();
    }

    let prev_btn = button(text("← Prev").size(12).color(theme::TEXT_PRIMARY))
        .padding(Padding::from([4, 12]))
        .style(|_, status| {
            let bg = match status {
                button::Status::Hovered => theme::BG_HOVER,
                _ => theme::BG_TERTIARY,
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press_maybe(if page > 1 {
            Some(Message::PageChanged(page - 1))
        } else {
            None
        });

    let next_btn = button(text("Next →").size(12).color(theme::TEXT_PRIMARY))
        .padding(Padding::from([4, 12]))
        .style(|_, status| {
            let bg = match status {
                button::Status::Hovered => theme::BG_HOVER,
                _ => theme::BG_TERTIARY,
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press_maybe(if page < total_pages {
            Some(Message::PageChanged(page + 1))
        } else {
            None
        });

    let page_info = text(format!(
        "Page {} of {} ({} groups)",
        page, total_pages, total_groups
    ))
    .size(12)
    .color(theme::TEXT_MUTED);

    container(
        row![prev_btn, page_info, next_btn]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding(Padding::from([8, 20])),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
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

use iced::widget::column;
