use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, Space,
};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::core::tmdb::{TmdbEpisode, TmdbSeason};
use crate::theme;

pub fn episode_resolve_modal<'a>(
    job_id: i64,
    seasons: &'a [TmdbSeason],
    selected_season: i64,
    episodes: &'a [TmdbEpisode],
    current_season: Option<i64>,
    current_episode: Option<i64>,
) -> Element<'a, Message> {
    let season_options: Vec<String> = seasons
        .iter()
        .map(|s| format!("Season {}", s.season_number))
        .collect();

    let selected = format!("Season {}", selected_season);

    let season_picker = pick_list(season_options, Some(selected), move |val| {
        // Parse "Season N" back to number
        let num: i64 = val
            .strip_prefix("Season ")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Message::EpisodeSeasonChanged(num)
    })
    .text_size(13);

    let episode_rows: Vec<Element<'a, Message>> = episodes
        .iter()
        .map(|ep| {
            let is_current = current_season == Some(ep.season_number)
                && current_episode == Some(ep.episode_number);
            let ep_num = ep.episode_number;
            let season_num = ep.season_number;
            let ep_title = ep.name.clone();

            let btn_label = if is_current { "Current" } else { "Use" };
            let btn_style = if is_current { theme::SUCCESS } else { theme::ACCENT };

            container(
                row![
                    text(format!("E{:02}", ep.episode_number))
                        .size(13)
                        .color(theme::ACCENT),
                    column![
                        text(&ep.name).size(13).color(theme::TEXT_PRIMARY),
                        text(
                            ep.overview
                                .as_deref()
                                .unwrap_or("")
                                .chars()
                                .take(100)
                                .collect::<String>()
                        )
                        .size(11)
                        .color(theme::TEXT_MUTED),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(btn_label).size(11).color(theme::TEXT_PRIMARY))
                        .padding(Padding::from([4, 12]))
                        .style(move |_, _| button::Style {
                            background: Some(btn_style.into()),
                            border: Border::default().rounded(4),
                            ..Default::default()
                        })
                        .on_press_maybe(if is_current {
                            None
                        } else {
                            Some(Message::UseEpisode {
                                job_id,
                                season: season_num,
                                episode: ep_num,
                                title: ep_title.clone(),
                            })
                        }),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .padding(8),
            )
            .style(|_: &Theme| container::Style {
                background: Some(theme::BG_TERTIARY.into()),
                border: Border::default().rounded(6),
                ..Default::default()
            })
            .into()
        })
        .collect();

    let content = column![
        text("Resolve Episode").size(16).color(theme::TEXT_PRIMARY),
        season_picker,
        scrollable(column(episode_rows).spacing(6)).height(Length::Fill),
        row![
            Space::new().width(Length::Fill),
            button(text("Close").size(13).color(theme::TEXT_PRIMARY))
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
                .on_press(Message::CloseEpisodeResolve),
        ],
    ]
    .spacing(12)
    .padding(24)
    .width(520);

    let modal = container(content).max_height(600).style(|_: &Theme| container::Style {
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
    .on_press(Message::CloseEpisodeResolve)
    .into()
}
