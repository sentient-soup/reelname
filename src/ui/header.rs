use iced::widget::{button, container, row, text, Space};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::theme;

pub fn header_bar(
    total_groups: i64,
    total_files: i64,
    selected_count: usize,
    scanning: bool,
) -> Element<'static, Message> {
    let title = row![
        text("Reel").size(20).color(theme::ACCENT),
        text("Name").size(20).color(theme::TEXT_PRIMARY),
    ]
    .spacing(0);

    let mut stats_parts = vec![
        text(format!("{total_groups} groups")).size(13).color(theme::TEXT_SECONDARY).into(),
        text(" · ").size(13).color(theme::TEXT_MUTED).into(),
        text(format!("{total_files} files")).size(13).color(theme::TEXT_SECONDARY).into(),
    ];

    if selected_count > 0 {
        stats_parts.push(text(" · ").size(13).color(theme::TEXT_MUTED).into());
        stats_parts.push(
            text(format!("{selected_count} selected"))
                .size(13)
                .color(theme::ACCENT)
                .into(),
        );
    }

    let stats = row(stats_parts).spacing(0);

    let left = row![title, Space::new().width(16), stats]
        .align_y(iced::Alignment::Center);

    let transfer_btn = styled_button("Transfers", Message::ToggleTransferDrawer, false);
    let settings_btn = styled_button("Settings", Message::ToggleSettings, false);
    let match_btn = styled_button("Match", Message::MatchRequested, false);

    let scan_label = if scanning { "Scanning..." } else { "Scan" };
    let scan_btn = button(
        text(scan_label).size(13).color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([6, 16]))
    .style(move |_, status| {
        let bg = match status {
            button::Status::Hovered => theme::ACCENT_HOVER,
            _ => theme::ACCENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: theme::TEXT_PRIMARY,
            border: Border::default().rounded(6),
            ..Default::default()
        }
    })
    .on_press_maybe(if scanning { None } else { Some(Message::ScanRequested) });

    let right = row![transfer_btn, settings_btn, match_btn, scan_btn]
        .spacing(8)
        .align_y(iced::Alignment::Center);

    container(
        row![left, Space::new().width(Length::Fill), right]
            .align_y(iced::Alignment::Center)
            .padding(Padding::from([12, 20])),
    )
    .width(Length::Fill)
    .style(|_: &Theme| container::Style {
        background: Some(theme::BG_SECONDARY.into()),
        border: Border {
            color: theme::BORDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn styled_button(label: &str, msg: Message, _active: bool) -> Element<'static, Message> {
    let label = label.to_string();
    button(
        text(label).size(13).color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([6, 14]))
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
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(msg)
    .into()
}
