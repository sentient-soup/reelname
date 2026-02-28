use iced::widget::{container, text};
use iced::{Border, Element, Padding, Theme};

use crate::app::Message;
use crate::db::schema::{FileCategory, GroupStatus, MediaType};
use crate::theme;

/// Render a status badge.
pub fn status_badge(status: GroupStatus) -> Element<'static, Message> {
    let color = theme::status_color(status);
    container(
        text(status.as_str())
            .size(11)
            .color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([2, 8]))
    .style(move |_: &Theme| container::Style {
        background: Some(color.into()),
        border: Border::default().rounded(4),
        ..Default::default()
    })
    .into()
}

/// Render a media type badge.
pub fn media_type_badge(mt: MediaType) -> Element<'static, Message> {
    let color = theme::media_type_color(mt);
    let label = mt.as_str().to_uppercase();
    container(
        text(label)
            .size(10)
            .color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([2, 6]))
    .style(move |_: &Theme| container::Style {
        background: Some(color.into()),
        border: Border::default().rounded(3),
        ..Default::default()
    })
    .into()
}

/// Render a file category badge.
pub fn file_category_badge(fc: FileCategory) -> Element<'static, Message> {
    let color = theme::file_category_color(fc);
    container(
        text(fc.as_str())
            .size(10)
            .color(theme::TEXT_PRIMARY),
    )
    .padding(Padding::from([1, 5]))
    .style(move |_: &Theme| container::Style {
        background: Some(color.into()),
        border: Border::default().rounded(3),
        ..Default::default()
    })
    .into()
}
