use iced::widget::{
    button, checkbox, column, container, row, scrollable, text, Space,
};
use iced::{Border, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::db::schema::*;
use crate::theme;
use crate::ui::badges;

/// Format file size in human-readable form.
fn format_size(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn queue_table<'a>(
    groups: &'a [GroupWithJobs],
    expanded_ids: &'a std::collections::HashMap<i64, bool>,
    selected_ids: &'a std::collections::HashMap<i64, bool>,
    active_group_id: Option<i64>,
    sort_by: &'a str,
    sort_dir: &'a str,
) -> Element<'a, Message> {
    let all_selected = !groups.is_empty()
        && groups.iter().all(|g| selected_ids.get(&g.group.id).copied().unwrap_or(false));

    // Header
    let header = container(
        row![
            // Checkbox
            container(
                checkbox(all_selected)
                    .on_toggle(|_| Message::ToggleSelectAll),
            )
            .width(40)
            .center_x(40),
            // Type
            container(text("Type").size(12).color(theme::TEXT_MUTED)).width(70),
            // Title (sortable)
            sort_header("Title", "folderName", sort_by, sort_dir, Length::Fill),
            // Size (sortable)
            sort_header("Size", "totalFileSize", sort_by, sort_dir, Length::Fixed(90.0)),
            // Status (sortable)
            sort_header("Status", "status", sort_by, sort_dir, Length::Fixed(100.0)),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .padding(Padding::from([8, 16])),
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
    });

    // Rows
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for gwj in groups {
        let g = &gwj.group;
        let is_expanded = expanded_ids.get(&g.id).copied().unwrap_or(false);
        let is_selected = selected_ids.get(&g.id).copied().unwrap_or(false);
        let is_active = active_group_id == Some(g.id);

        rows.push(group_row(g, is_expanded, is_selected, is_active));

        if is_expanded {
            for jwp in &gwj.jobs {
                rows.push(file_row(&jwp.job, jwp.preview_name.as_deref()));
            }
        }
    }

    if rows.is_empty() {
        rows.push(
            container(
                text("No groups found. Scan a directory to get started.")
                    .size(14)
                    .color(theme::TEXT_MUTED),
            )
            .width(Length::Fill)
            .padding(40)
            .center_x(Length::Fill)
            .into(),
        );
    }

    let table_body = scrollable(column(rows).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill);

    column![header, table_body].width(Length::Fill).into()
}

fn sort_header<'a>(
    label: &str,
    field: &str,
    current_sort: &str,
    current_dir: &str,
    width: Length,
) -> Element<'a, Message> {
    let is_active = current_sort == field;
    let arrow = if is_active {
        if current_dir == "asc" { " ▲" } else { " ▼" }
    } else {
        ""
    };

    let label_text = format!("{label}{arrow}");
    let color = if is_active { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };
    let field = field.to_string();

    button(text(label_text).size(12).color(color))
        .padding(0)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        })
        .on_press(Message::SortChanged(field))
        .width(width)
        .into()
}

fn group_row(group: &Group, expanded: bool, selected: bool, active: bool) -> Element<'static, Message> {
    let id = group.id;
    let bg = if active {
        theme::BG_HOVER
    } else {
        theme::BG_PRIMARY
    };

    let expand_icon = if expanded { "▼" } else { "▶" };
    let title_display = group
        .tmdb_title
        .as_deref()
        .or(group.parsed_title.as_deref())
        .unwrap_or(&group.folder_name);

    let title_str = format!("{} {}", expand_icon, title_display);
    let year_str = group
        .tmdb_year
        .or(group.parsed_year)
        .map(|y| format!(" ({})", y))
        .unwrap_or_default();
    let files_str = format!(" · {} files", group.total_file_count);

    let row_content = row![
        // Checkbox
        container(
            checkbox(selected)
                .on_toggle(move |_| Message::ToggleGroupSelected(id)),
        )
        .width(40)
        .center_x(40),
        // Type badge
        container(badges::media_type_badge(group.media_type)).width(70),
        // Title + details
        container(
            row![
                text(title_str).size(13).color(theme::TEXT_PRIMARY),
                text(year_str).size(13).color(theme::TEXT_SECONDARY),
                text(files_str).size(12).color(theme::TEXT_MUTED),
            ]
            .spacing(2),
        )
        .width(Length::Fill),
        // Size
        container(
            text(format_size(group.total_file_size))
                .size(12)
                .color(theme::TEXT_SECONDARY),
        )
        .width(90),
        // Status badge
        container(badges::status_badge(group.status)).width(100),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .padding(Padding::from([8, 16]));

    button(row_content)
        .padding(0)
        .width(Length::Fill)
        .style(move |_, status| {
            let bg = match status {
                button::Status::Hovered => theme::BG_HOVER,
                _ => bg,
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    color: theme::BORDER,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(Message::GroupClicked(id))
        .into()
}

fn file_row(job: &Job, preview_name: Option<&str>) -> Element<'static, Message> {
    let se_label = match (job.parsed_season, job.parsed_episode) {
        (Some(s), Some(e)) => format!("S{:02}E{:02}", s, e),
        (None, Some(e)) => format!("E{:02}", e),
        _ => String::new(),
    };

    let ep_title = job.tmdb_episode_title.clone().unwrap_or_default();
    let file_name = job.file_name.clone();

    let mut details = row![
        Space::new().width(40), // indent
        container(badges::file_category_badge(job.file_category)).width(70),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let mut title_parts = row![].spacing(4);

    if !se_label.is_empty() {
        title_parts = title_parts.push(
            text(se_label)
                .size(12)
                .color(theme::TEXT_SECONDARY),
        );
    }

    title_parts = title_parts.push(
        text(file_name)
            .size(12)
            .color(theme::TEXT_SECONDARY),
    );

    if !ep_title.is_empty() {
        title_parts = title_parts.push(
            text(format!("· {ep_title}"))
                .size(12)
                .color(theme::TEXT_MUTED),
        );
    }

    details = details.push(container(title_parts).width(Length::Fill));

    // Size
    details = details.push(
        container(
            text(format_size(job.file_size))
                .size(11)
                .color(theme::TEXT_MUTED),
        )
        .width(90),
    );

    // Preview name
    if let Some(preview) = preview_name {
        details = details.push(
            container(
                text(format!("→ {preview}"))
                    .size(11)
                    .color(theme::ACCENT),
            )
            .width(100),
        );
    } else {
        details = details.push(Space::new().width(100));
    }

    container(details.padding(Padding::from([4, 16])))
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
