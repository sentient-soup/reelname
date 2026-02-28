use iced::widget::{column, container, mouse_area, text};
use iced::{Border, Color, Element, Length, Padding, Theme};
use std::time::Instant;

use crate::app::Message;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(id: u64, message: String, toast_type: ToastType) -> Self {
        Self {
            id,
            message,
            toast_type,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_millis() > 4000
    }
}

fn toast_colors(tt: ToastType) -> (Color, Color, Color) {
    // (border, background, text)
    match tt {
        ToastType::Success => (theme::SUCCESS, with_alpha(theme::SUCCESS, 0.1), theme::SUCCESS),
        ToastType::Error => (theme::ERROR, with_alpha(theme::ERROR, 0.1), theme::ERROR),
        ToastType::Warning => (theme::WARNING, with_alpha(theme::WARNING, 0.1), theme::WARNING),
        ToastType::Info => (theme::INFO, with_alpha(theme::INFO, 0.1), theme::INFO),
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

/// Render the toast container (bottom-right, overlaid via Stack).
pub fn toast_container(toasts: &[Toast]) -> Element<'_, Message> {
    if toasts.is_empty() {
        return container(column![]).width(0).height(0).into();
    }

    let toast_views: Vec<Element<'_, Message>> = toasts
        .iter()
        .map(|t| {
            let (border_color, bg_color, text_color) = toast_colors(t.toast_type);
            let id = t.id;

            mouse_area(
                container(
                    text(&t.message)
                        .size(13)
                        .color(text_color),
                )
                .padding(Padding::from([10, 16]))
                .width(320)
                .style(move |_: &Theme| container::Style {
                    background: Some(bg_color.into()),
                    border: Border {
                        color: border_color,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
            )
            .on_press(Message::DismissToast(id))
            .into()
        })
        .collect();

    container(column(toast_views).spacing(8))
        .padding(16)
        .width(Length::Shrink)
        .into()
}
