use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::models::{Group, Job};

// ── Naming presets ──────────────────────────────────────

struct PresetTemplates {
    movie: &'static str,
    tv: &'static str,
    special: &'static str,
    extra: &'static str,
}

static JELLYFIN: PresetTemplates = PresetTemplates {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} S{season:2}E{episode:2} - {episodeTitle}.{ext}",
    special: "{title} ({year})/Season 00/{title} S00E{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
};

static PLEX: PresetTemplates = PresetTemplates {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} ({year}) - s{season:2}e{episode:2} - {episodeTitle}.{ext}",
    special: "{title} ({year})/Specials/{title} ({year}) - s00e{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
};

static JELLYFIN_EXTRA_FOLDERS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    [
        ("behind_the_scenes", "behind the scenes"),
        ("deleted_scenes", "deleted scenes"),
        ("featurettes", "featurettes"),
        ("interviews", "interviews"),
        ("scenes", "clips"),
        ("shorts", "shorts"),
        ("trailers", "trailers"),
        ("other", "extras"),
    ]
    .into_iter()
    .collect()
});

static PLEX_EXTRA_FOLDERS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    [
        ("behind_the_scenes", "Behind The Scenes"),
        ("deleted_scenes", "Deleted Scenes"),
        ("featurettes", "Featurettes"),
        ("interviews", "Interviews"),
        ("scenes", "Scenes"),
        ("shorts", "Shorts"),
        ("trailers", "Trailers"),
        ("other", "Other"),
    ]
    .into_iter()
    .collect()
});

// ── Helpers ─────────────────────────────────────────────

fn sanitize(s: &str) -> String {
    let s = s.replace(['<', '>', ':', '"', '/', '\\', '|', '?', '*'], "");
    let re = regex::Regex::new(r"\s+").unwrap();
    re.replace_all(&s, " ").trim().to_string()
}

fn pad_num(val: Option<i64>, width: usize) -> String {
    let n = val.unwrap_or(0);
    format!("{:0>width$}", n, width = width)
}

// ── Public ──────────────────────────────────────────────

pub struct NamingSettings {
    pub naming_preset: String,
    pub specials_folder_name: String,
    pub extras_folder_name: String,
}

pub fn format_grouped_path(job: &Job, group: &Group, settings: &NamingSettings) -> String {
    let preset_name = if settings.naming_preset.is_empty() {
        "jellyfin"
    } else {
        &settings.naming_preset
    };
    let preset = if preset_name == "plex" { &PLEX } else { &JELLYFIN };

    // Select template by file category
    let template = match job.file_category.as_str() {
        "movie" => preset.movie,
        "special" => preset.special,
        "extra" => preset.extra,
        _ => preset.tv,
    };

    // Use group-level TMDB info, fallback to job-level
    let title = sanitize(
        group
            .tmdb_title
            .as_deref()
            .or(group.parsed_title.as_deref())
            .unwrap_or("Unknown"),
    );
    let year = group
        .tmdb_year
        .or(group.parsed_year)
        .or(job.tmdb_year)
        .or(job.parsed_year)
        .map(|y| y.to_string())
        .unwrap_or_default();
    let ext = job.file_extension.trim_start_matches('.');
    let episode_title = sanitize(job.tmdb_episode_title.as_deref().unwrap_or(""));
    let quality = job.parsed_quality.as_deref().unwrap_or("");
    let file_name_stem = sanitize(
        &job.file_name
            .rfind('.')
            .map(|i| &job.file_name[..i])
            .unwrap_or(&job.file_name),
    );

    // Extra type folder name
    let extra_folder_map = if preset_name == "plex" {
        &*PLEX_EXTRA_FOLDERS
    } else {
        &*JELLYFIN_EXTRA_FOLDERS
    };
    let extra_type_name = job
        .extra_type
        .as_deref()
        .and_then(|et| extra_folder_map.get(et).copied())
        .unwrap_or(if preset_name == "plex" {
            "Other"
        } else {
            "extras"
        });

    let mut result = template.to_string();

    result = result.replace("{title}", &title);
    result = result.replace("{year}", &year);
    result = result.replace("{ext}", ext);
    result = result.replace(
        "{episodeTitle}",
        if episode_title.is_empty() {
            "Episode"
        } else {
            &episode_title
        },
    );
    result = result.replace("{quality}", quality);
    result = result.replace("{fileName}", &file_name_stem);
    result = result.replace("{extraType}", extra_type_name);

    // Season/episode with padding: {season:2}, {episode:2}
    let season_pad_re = regex::Regex::new(r"\{season:(\d+)\}").unwrap();
    result = season_pad_re
        .replace_all(&result, |caps: &regex::Captures| {
            let width: usize = caps[1].parse().unwrap_or(2);
            pad_num(job.parsed_season, width)
        })
        .to_string();

    let episode_pad_re = regex::Regex::new(r"\{episode:(\d+)\}").unwrap();
    result = episode_pad_re
        .replace_all(&result, |caps: &regex::Captures| {
            let width: usize = caps[1].parse().unwrap_or(2);
            pad_num(job.parsed_episode, width)
        })
        .to_string();

    result = result.replace("{season}", &job.parsed_season.unwrap_or(0).to_string());
    result = result.replace("{episode}", &job.parsed_episode.unwrap_or(0).to_string());

    // Clean up empty episode titles leaving trailing " - "
    result = result.replace(" - .", ".");
    result = result.replace(" - Episode.", ".");

    // Clean up empty year leaving "()" in path
    result = result.replace(" ()", "");

    result
}
