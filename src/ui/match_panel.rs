use iced::widget::{
    button, column, container, image, row, scrollable, text, text_input, Space,
};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::db::schema::*;
use crate::theme;
use crate::ui::badges;

const PANEL_WIDTH: f32 = 420.0;
const POSTER_WIDTH: f32 = 60.0;
const POSTER_HEIGHT: f32 = 90.0;

pub fn match_panel<'a>(
    group: &'a GroupWithJobs,
    search_query: &'a str,
    search_results: &'a [MatchCandidate],
    editing_group: bool,
    edit_title: &'a str,
    edit_year: &'a str,
    poster_cache: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let g = &group.group;

    // Header
    let header = container(
        row![
            text("Match Details").size(16).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            button(text("✕").size(14).color(theme::TEXT_MUTED))
                .padding(Padding::from([2, 8]))
                .style(|_, _| button::Style {
                    background: None,
                    ..Default::default()
                })
                .on_press(Message::CloseMatchPanel),
        ]
        .align_y(iced::Alignment::Center)
        .padding(Padding::from([12, 16])),
    )
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    // Group info
    let folder_name = text(&g.folder_name)
        .size(12)
        .color(theme::TEXT_SECONDARY);

    let group_info = if editing_group {
        // Edit mode
        column![
            folder_name,
            row![
                text("Title:").size(12).color(theme::TEXT_MUTED),
                text_input("Title", edit_title)
                    .on_input(Message::EditTitleChanged)
                    .size(12)
                    .padding(Padding::from([4, 8])),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Year:").size(12).color(theme::TEXT_MUTED),
                text_input("Year", edit_year)
                    .on_input(Message::EditYearChanged)
                    .size(12)
                    .padding(Padding::from([4, 8]))
                    .width(80),
                button(text("Save").size(11).color(theme::TEXT_PRIMARY))
                    .padding(Padding::from([3, 10]))
                    .style(|_, _| button::Style {
                        background: Some(theme::ACCENT.into()),
                        border: Border::default().rounded(4),
                        ..Default::default()
                    })
                    .on_press(Message::SaveGroupEdit),
                button(text("Cancel").size(11).color(theme::TEXT_MUTED))
                    .padding(Padding::from([3, 10]))
                    .style(|_, _| button::Style {
                        background: Some(theme::BG_TERTIARY.into()),
                        border: Border::default().rounded(4),
                        ..Default::default()
                    })
                    .on_press(Message::CancelGroupEdit),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(6)
        .padding(Padding::from([8, 16]))
    } else {
        column![
            row![
                folder_name,
                Space::new().width(Length::Fill),
                badges::status_badge(g.status),
                button(text("Edit").size(11).color(theme::ACCENT))
                    .padding(Padding::from([2, 8]))
                    .style(|_, _| button::Style {
                        background: None,
                        ..Default::default()
                    })
                    .on_press(Message::StartGroupEdit),
            ]
            .align_y(iced::Alignment::Center),
            text(format!("{} files", g.total_file_count))
                .size(12)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(4)
        .padding(Padding::from([8, 16]))
    };

    // Files list
    let files_list: Vec<Element<'a, Message>> = group
        .jobs
        .iter()
        .map(|jwp| {
            let j = &jwp.job;
            let se = match (j.parsed_season, j.parsed_episode) {
                (Some(s), Some(e)) => format!("S{:02}E{:02} ", s, e),
                _ => String::new(),
            };
            let title = j
                .tmdb_episode_title
                .as_deref()
                .unwrap_or(&j.file_name);
            let job_id = j.id;
            let is_tv = g.media_type == MediaType::Tv && g.tmdb_id.is_some();

            let content = row![
                badges::file_category_badge(j.file_category),
                text(format!("{se}{title}"))
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);

            if is_tv {
                button(content)
                    .padding(Padding::from([4, 8]))
                    .width(Length::Fill)
                    .style(|_, status| {
                        let bg = match status {
                            button::Status::Hovered => theme::BG_HOVER,
                            _ => theme::BG_TERTIARY,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            border: Border::default().rounded(4),
                            ..Default::default()
                        }
                    })
                    .on_press(Message::OpenEpisodeResolve(job_id))
                    .into()
            } else {
                container(content)
                    .padding(Padding::from([4, 8]))
                    .width(Length::Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(theme::BG_TERTIARY.into()),
                        border: Border::default().rounded(4),
                        ..Default::default()
                    })
                    .into()
            }
        })
        .collect();

    let files_section = container(
        scrollable(column(files_list).spacing(4))
            .height(Length::Fixed(200.0)),
    )
    .padding(Padding::from([0, 16]));

    // Candidates
    let candidates_header = container(
        text("TMDB Candidates")
            .size(13)
            .color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([8, 16]));

    let candidate_cards: Vec<Element<'a, Message>> = group
        .candidates
        .iter()
        .map(|c| candidate_card(c, poster_cache))
        .collect();

    let candidates_section = container(
        scrollable(column(candidate_cards).spacing(8))
            .height(Length::Fill),
    )
    .padding(Padding::from([0, 16]));

    // Manual search
    let search_section = container(
        column![
            text("Manual Search").size(13).color(theme::TEXT_PRIMARY),
            row![
                text_input("Search TMDB...", search_query)
                    .on_input(Message::ManualSearchChanged)
                    .on_submit(Message::ManualSearchSubmit)
                    .size(12)
                    .padding(Padding::from([6, 10])),
                button(text("Search").size(12).color(theme::TEXT_PRIMARY))
                    .padding(Padding::from([6, 12]))
                    .style(|_, _| button::Style {
                        background: Some(theme::BG_TERTIARY.into()),
                        border: Border {
                            color: theme::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .on_press(Message::ManualSearchSubmit),
            ]
            .spacing(8),
        ]
        .spacing(8),
    )
    .padding(Padding::from([8, 16]));

    // Search results
    let search_results_section = if !search_results.is_empty() {
        let cards: Vec<Element<'a, Message>> = search_results
            .iter()
            .map(|c| candidate_card(c, poster_cache))
            .collect();
        container(
            scrollable(column(cards).spacing(8))
                .height(Length::Fixed(200.0)),
        )
        .padding(Padding::from([0, 16]))
    } else {
        container(column![]).width(0).height(0)
    };

    // Footer
    let can_confirm = matches!(g.status, GroupStatus::Matched | GroupStatus::Ambiguous);
    let group_id = g.id;

    let footer = container(
        row![
            button(text("Skip").size(13).color(theme::TEXT_PRIMARY))
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
                .on_press(Message::SkipGroup(group_id)),
            Space::new().width(Length::Fill),
            button(text("Confirm Top Match").size(13).color(theme::TEXT_PRIMARY))
                .padding(Padding::from([8, 20]))
                .style(|_, _| button::Style {
                    background: Some(theme::ACCENT.into()),
                    border: Border::default().rounded(6),
                    ..Default::default()
                })
                .on_press_maybe(if can_confirm {
                    Some(Message::ConfirmTopMatch(group_id))
                } else {
                    None
                }),
        ]
        .align_y(iced::Alignment::Center)
        .padding(Padding::from([12, 16])),
    )
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    container(
        column![
            header,
            group_info,
            files_section,
            candidates_header,
            candidates_section,
            search_section,
            search_results_section,
            footer,
        ]
        .width(Length::Fixed(PANEL_WIDTH)),
    )
    .height(Length::Fill)
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

fn candidate_card<'a>(
    candidate: &'a MatchCandidate,
    poster_cache: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let conf = candidate.confidence;
    let conf_color = crate::theme::confidence_color(conf);
    let conf_text = format!("{:.0}%", conf * 100.0);

    let year_str = candidate
        .year
        .map(|y| format!(" ({})", y))
        .unwrap_or_default();

    let overview = candidate
        .overview
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(120)
        .collect::<String>();

    // Poster or placeholder
    let poster: Element<'a, Message> = if let Some(path) = &candidate.poster_path {
        if let Some(handle) = poster_cache.get(path) {
            image(handle.clone())
                .width(POSTER_WIDTH)
                .height(POSTER_HEIGHT)
                .into()
        } else {
            container(text("").size(10))
                .width(POSTER_WIDTH)
                .height(POSTER_HEIGHT)
                .style(|_: &Theme| container::Style {
                    background: Some(theme::BG_TERTIARY.into()),
                    ..Default::default()
                })
                .into()
        }
    } else {
        container(text("").size(10))
            .width(POSTER_WIDTH)
            .height(POSTER_HEIGHT)
            .style(|_: &Theme| container::Style {
                background: Some(theme::BG_TERTIARY.into()),
                ..Default::default()
            })
            .into()
    };

    let tmdb_id = candidate.tmdb_id;
    let media_type = candidate.media_type;
    let group_id = candidate.group_id.unwrap_or(0);

    let info = column![
        row![
            text(&candidate.title).size(13).color(theme::TEXT_PRIMARY),
            text(year_str).size(13).color(theme::TEXT_SECONDARY),
        ]
        .spacing(2),
        row![
            badges::media_type_badge(candidate.media_type),
            text(conf_text).size(12).color(conf_color),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        text(overview).size(11).color(theme::TEXT_MUTED),
        button(text("Use").size(11).color(theme::TEXT_PRIMARY))
            .padding(Padding::from([3, 12]))
            .style(|_, _| button::Style {
                background: Some(theme::ACCENT.into()),
                border: Border::default().rounded(4),
                ..Default::default()
            })
            .on_press(Message::UseCandidate {
                group_id,
                tmdb_id,
                media_type,
            }),
    ]
    .spacing(4)
    .width(Length::Fill);

    container(
        row![poster, info].spacing(10).padding(8),
    )
    .style(|_: &Theme| container::Style {
        background: Some(theme::BG_TERTIARY.into()),
        border: Border::default().rounded(6),
        ..Default::default()
    })
    .into()
}
