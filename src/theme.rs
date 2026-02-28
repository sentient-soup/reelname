use iced::Color;

// ── Background colors ──
pub const BG_PRIMARY: Color = Color::from_rgb(
    0x0c as f32 / 255.0,
    0x0f as f32 / 255.0,
    0x1a as f32 / 255.0,
);
pub const BG_SECONDARY: Color = Color::from_rgb(
    0x14 as f32 / 255.0,
    0x18 as f32 / 255.0,
    0x29 as f32 / 255.0,
);
pub const BG_TERTIARY: Color = Color::from_rgb(
    0x1c as f32 / 255.0,
    0x20 as f32 / 255.0,
    0x39 as f32 / 255.0,
);
pub const BG_HOVER: Color = Color::from_rgb(
    0x25 as f32 / 255.0,
    0x2a as f32 / 255.0,
    0x45 as f32 / 255.0,
);

// ── Border colors ──
pub const BORDER: Color = Color::from_rgb(
    0x2a as f32 / 255.0,
    0x2f as f32 / 255.0,
    0x4a as f32 / 255.0,
);
pub const BORDER_LIGHT: Color = Color::from_rgb(
    0x3a as f32 / 255.0,
    0x3f as f32 / 255.0,
    0x5a as f32 / 255.0,
);

// ── Text colors ──
pub const TEXT_PRIMARY: Color = Color::from_rgb(
    0xe8 as f32 / 255.0,
    0xea as f32 / 255.0,
    0xf0 as f32 / 255.0,
);
pub const TEXT_SECONDARY: Color = Color::from_rgb(
    0x9d as f32 / 255.0,
    0xa3 as f32 / 255.0,
    0xb8 as f32 / 255.0,
);
pub const TEXT_MUTED: Color = Color::from_rgb(
    0x6b as f32 / 255.0,
    0x71 as f32 / 255.0,
    0x94 as f32 / 255.0,
);

// ── Accent ──
pub const ACCENT: Color = Color::from_rgb(
    0x63 as f32 / 255.0,
    0x66 as f32 / 255.0,
    0xf1 as f32 / 255.0,
);
pub const ACCENT_HOVER: Color = Color::from_rgb(
    0x81 as f32 / 255.0,
    0x8c as f32 / 255.0,
    0xf8 as f32 / 255.0,
);
pub const ACCENT_DIM: Color = Color::from_rgb(
    0x43 as f32 / 255.0,
    0x38 as f32 / 255.0,
    0xca as f32 / 255.0,
);

// ── Semantic ──
pub const SUCCESS: Color = Color::from_rgb(
    0x22 as f32 / 255.0,
    0xc5 as f32 / 255.0,
    0x5e as f32 / 255.0,
);
pub const WARNING: Color = Color::from_rgb(
    0xf5 as f32 / 255.0,
    0x9e as f32 / 255.0,
    0x0b as f32 / 255.0,
);
pub const ERROR: Color = Color::from_rgb(
    0xef as f32 / 255.0,
    0x44 as f32 / 255.0,
    0x44 as f32 / 255.0,
);
pub const INFO: Color = Color::from_rgb(
    0x3b as f32 / 255.0,
    0x82 as f32 / 255.0,
    0xf6 as f32 / 255.0,
);

// ── Status colors ──
pub const STATUS_SCANNED: Color = Color::from_rgb(
    0x8b as f32 / 255.0,
    0x5c as f32 / 255.0,
    0xf6 as f32 / 255.0,
);
pub const STATUS_MATCHED: Color = SUCCESS;
pub const STATUS_AMBIGUOUS: Color = WARNING;
pub const STATUS_CONFIRMED: Color = INFO;
pub const STATUS_TRANSFERRING: Color = Color::from_rgb(
    0x06 as f32 / 255.0,
    0xb6 as f32 / 255.0,
    0xd4 as f32 / 255.0,
);
pub const STATUS_COMPLETED: Color = Color::from_rgb(
    0x10 as f32 / 255.0,
    0xb9 as f32 / 255.0,
    0x81 as f32 / 255.0,
);
pub const STATUS_FAILED: Color = ERROR;
pub const STATUS_SKIPPED: Color = Color::from_rgb(
    0x6b as f32 / 255.0,
    0x72 as f32 / 255.0,
    0x80 as f32 / 255.0,
);

use crate::db::schema::GroupStatus;

pub fn status_color(status: GroupStatus) -> Color {
    match status {
        GroupStatus::Scanned => STATUS_SCANNED,
        GroupStatus::Matched => STATUS_MATCHED,
        GroupStatus::Ambiguous => STATUS_AMBIGUOUS,
        GroupStatus::Confirmed => STATUS_CONFIRMED,
        GroupStatus::Transferring => STATUS_TRANSFERRING,
        GroupStatus::Completed => STATUS_COMPLETED,
        GroupStatus::Failed => STATUS_FAILED,
        GroupStatus::Skipped => STATUS_SKIPPED,
    }
}

use crate::db::schema::MediaType;

pub fn media_type_color(mt: MediaType) -> Color {
    match mt {
        MediaType::Movie => INFO,
        MediaType::Tv => ACCENT,
        MediaType::Unknown => BG_TERTIARY,
    }
}

use crate::db::schema::FileCategory;

pub fn file_category_color(fc: FileCategory) -> Color {
    match fc {
        FileCategory::Episode => Color { a: 0.7, ..ACCENT },
        FileCategory::Movie => Color { a: 0.7, ..INFO },
        FileCategory::Special => Color { a: 0.7, ..WARNING },
        FileCategory::Extra => BG_TERTIARY,
    }
}

/// Confidence color: green >= 85%, yellow >= 50%, red < 50%.
pub fn confidence_color(confidence: f64) -> Color {
    if confidence >= 0.85 {
        SUCCESS
    } else if confidence >= 0.50 {
        WARNING
    } else {
        ERROR
    }
}
