use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::theme;

pub fn settings_modal<'a>(
    scan_path: &'a str,
    tmdb_api_key: &'a str,
    auto_match_threshold: &'a str,
    data_dir: &'a str,
    naming_preset: &'a str,
    specials_folder_name: &'a str,
    extras_folder_name: &'a str,
) -> Element<'a, Message> {
    let preset_options = vec!["jellyfin".to_string(), "plex".to_string()];
    let selected_preset = Some(naming_preset.to_string());

    let preset_preview = match naming_preset {
        "plex" => "Movie Title (Year)/Movie Title (Year).ext\nShow (Year)/Season 01/Show (Year) - s01e01 - Episode.ext",
        _ => "Movie Title (Year)/Movie Title (Year).ext\nShow (Year)/Season 01/Show S01E01 - Episode.ext",
    };

    let content = column![
        text("Settings").size(18).color(theme::TEXT_PRIMARY),
        settings_field("Scan Path", scan_path, "scan_path"),
        settings_field("TMDB API Key", tmdb_api_key, "tmdb_api_key"),
        settings_field("Auto-Match Threshold", auto_match_threshold, "auto_match_threshold"),
        settings_field("Data Directory", data_dir, "data_dir"),
        column![
            text("Naming Preset").size(13).color(theme::TEXT_SECONDARY),
            pick_list(preset_options, selected_preset, |val| {
                Message::SettingChanged("naming_preset".to_string(), val)
            })
            .text_size(13)
            .padding(Padding::from([4, 8])),
            text(preset_preview)
                .size(11)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(4),
        settings_field("Specials Folder Name", specials_folder_name, "specials_folder_name"),
        settings_field("Extras Folder Name", extras_folder_name, "extras_folder_name"),
        Space::new().height(8),
        row![
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
                .on_press(Message::ToggleSettings),
            Space::new().width(Length::Fill),
            button(text("Save").size(13).color(theme::TEXT_PRIMARY))
                .padding(Padding::from([8, 20]))
                .style(|_, _| button::Style {
                    background: Some(theme::ACCENT.into()),
                    border: Border::default().rounded(6),
                    ..Default::default()
                })
                .on_press(Message::SaveSettings),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(12)
    .padding(24)
    .width(520);

    let modal = container(scrollable(content).height(Length::Shrink))
        .max_height(600)
        .style(|_: &Theme| container::Style {
            background: Some(theme::BG_SECONDARY.into()),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

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
    .on_press(Message::ToggleSettings)
    .into()
}

fn settings_field<'a>(label: &'a str, value: &'a str, field: &str) -> Element<'a, Message> {
    let field = field.to_string();
    column![
        text(label).size(13).color(theme::TEXT_SECONDARY),
        text_input("", value)
            .on_input(move |v| Message::SettingChanged(field.clone(), v))
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
