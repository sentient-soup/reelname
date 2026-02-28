use crate::db::schema::*;
use regex::Regex;

use std::sync::LazyLock;

/// Naming presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingPreset {
    Jellyfin,
    Plex,
}

impl NamingPreset {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "plex" => Self::Plex,
            _ => Self::Jellyfin,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jellyfin => "jellyfin",
            Self::Plex => "plex",
        }
    }
}

struct PresetTemplates {
    movie: &'static str,
    tv: &'static str,
    special: &'static str,
    extra: &'static str,
}

const JELLYFIN_TEMPLATES: PresetTemplates = PresetTemplates {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} S{season:2}E{episode:2} - {episodeTitle}.{ext}",
    special: "{title} ({year})/Season 00/{title} S00E{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
};

const PLEX_TEMPLATES: PresetTemplates = PresetTemplates {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} ({year}) - s{season:2}e{episode:2} - {episodeTitle}.{ext}",
    special: "{title} ({year})/Specials/{title} ({year}) - s00e{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
};

/// Extra type folder names for Jellyfin.
fn jellyfin_extra_folder(extra_type: ExtraType) -> &'static str {
    match extra_type {
        ExtraType::BehindTheScenes => "behind the scenes",
        ExtraType::DeletedScenes => "deleted scenes",
        ExtraType::Featurettes => "featurettes",
        ExtraType::Interviews => "interviews",
        ExtraType::Scenes => "clips",
        ExtraType::Shorts => "shorts",
        ExtraType::Trailers => "trailers",
        ExtraType::Other => "extras",
    }
}

/// Extra type folder names for Plex.
fn plex_extra_folder(extra_type: ExtraType) -> &'static str {
    match extra_type {
        ExtraType::BehindTheScenes => "Behind The Scenes",
        ExtraType::DeletedScenes => "Deleted Scenes",
        ExtraType::Featurettes => "Featurettes",
        ExtraType::Interviews => "Interviews",
        ExtraType::Scenes => "Scenes",
        ExtraType::Shorts => "Shorts",
        ExtraType::Trailers => "Trailers",
        ExtraType::Other => "Other",
    }
}

/// Sanitize a string for use in file/folder names.
pub fn sanitize(s: &str) -> String {
    static INVALID_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[<>:"/\\|?*]"#).unwrap());
    static MULTI_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let cleaned = INVALID_CHARS.replace_all(s, "");
    let cleaned = MULTI_SPACES.replace_all(&cleaned, " ");
    cleaned.trim().to_string()
}

/// Pad a number with leading zeros.
fn pad_num(val: Option<i64>, width: usize) -> String {
    let n = val.unwrap_or(0);
    format!("{:0>width$}", n, width = width)
}

/// Format a destination path for a job, given group info and settings.
pub fn format_grouped_path(
    group: &Group,
    job: &Job,
    preset: NamingPreset,
    _specials_folder_name: &str,
    extras_folder_name: &str,
) -> String {
    let templates = match preset {
        NamingPreset::Jellyfin => &JELLYFIN_TEMPLATES,
        NamingPreset::Plex => &PLEX_TEMPLATES,
    };

    // Choose template based on file category
    let template = match job.file_category {
        FileCategory::Movie => templates.movie,
        FileCategory::Episode => templates.tv,
        FileCategory::Special => templates.special,
        FileCategory::Extra => templates.extra,
    };

    // Build substitution values
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

    let episode_title = sanitize(
        job.tmdb_episode_title.as_deref().unwrap_or(""),
    );

    let quality = job.parsed_quality.as_deref().unwrap_or("");

    // File name without extension
    let file_name_no_ext = if let Some(pos) = job.file_name.rfind('.') {
        sanitize(&job.file_name[..pos])
    } else {
        sanitize(&job.file_name)
    };

    // Extra type folder name
    let extra_folder = if let Some(et) = job.extra_type {
        match preset {
            NamingPreset::Jellyfin => jellyfin_extra_folder(et).to_string(),
            NamingPreset::Plex => plex_extra_folder(et).to_string(),
        }
    } else {
        match preset {
            NamingPreset::Jellyfin => extras_folder_name.to_lowercase(),
            NamingPreset::Plex => extras_folder_name.to_string(),
        }
    };

    // Perform substitutions
    let mut result = template.to_string();
    result = result.replace("{title}", &title);
    result = result.replace("{year}", &year);
    result = result.replace("{ext}", ext);
    result = result.replace("{episodeTitle}", &episode_title);
    result = result.replace("{quality}", quality);
    result = result.replace("{fileName}", &file_name_no_ext);
    result = result.replace("{extraType}", &extra_folder);

    // Padded season/episode: {season:2}, {episode:2}
    static PAD_SEASON: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{season:(\d+)\}").unwrap());
    static PAD_EPISODE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{episode:(\d+)\}").unwrap());

    if let Some(caps) = PAD_SEASON.captures(&result) {
        if let Ok(width) = caps[1].parse::<usize>() {
            let padded = pad_num(job.parsed_season, width);
            result = PAD_SEASON.replace_all(&result, padded.as_str()).to_string();
        }
    }
    if let Some(caps) = PAD_EPISODE.captures(&result) {
        if let Ok(width) = caps[1].parse::<usize>() {
            let padded = pad_num(job.parsed_episode, width);
            result = PAD_EPISODE.replace_all(&result, padded.as_str()).to_string();
        }
    }

    // Plain season/episode (no padding specified)
    result = result.replace(
        "{season}",
        &job.parsed_season.unwrap_or(0).to_string(),
    );
    result = result.replace(
        "{episode}",
        &job.parsed_episode.unwrap_or(0).to_string(),
    );

    // Post-processing cleanups
    // " - ." -> "."  (trailing " - " before extension when no episode title)
    result = result.replace(" - .", ".");
    // " - Episode." -> "."  (literal fallback text cleanup)
    result = result.replace(" - Episode.", ".");
    // " ()" -> ""  (empty year cleanup)
    result = result.replace(" ()", "");

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_group(title: &str, year: Option<i64>) -> Group {
        Group {
            id: 1,
            status: GroupStatus::Confirmed,
            media_type: MediaType::Tv,
            folder_path: "/test".to_string(),
            folder_name: "test".to_string(),
            total_file_count: 1,
            total_file_size: 1000,
            parsed_title: Some(title.to_string()),
            parsed_year: year,
            tmdb_id: Some(1),
            tmdb_title: Some(title.to_string()),
            tmdb_year: year,
            tmdb_poster_path: None,
            match_confidence: Some(0.95),
            destination_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn make_job(category: FileCategory, season: Option<i64>, episode: Option<i64>) -> Job {
        Job {
            id: 1,
            group_id: Some(1),
            status: GroupStatus::Confirmed,
            media_type: MediaType::Tv,
            file_category: category,
            extra_type: None,
            source_path: "/test/file.mkv".to_string(),
            file_name: "file.mkv".to_string(),
            file_size: 1000,
            file_extension: "mkv".to_string(),
            parsed_title: None,
            parsed_year: None,
            parsed_season: season,
            parsed_episode: episode,
            parsed_quality: None,
            parsed_codec: None,
            tmdb_id: None,
            tmdb_title: None,
            tmdb_year: None,
            tmdb_poster_path: None,
            tmdb_episode_title: Some("Pilot".to_string()),
            match_confidence: None,
            destination_id: None,
            destination_path: None,
            transfer_progress: None,
            transfer_error: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_jellyfin_tv() {
        let group = make_group("Breaking Bad", Some(2008));
        let job = make_job(FileCategory::Episode, Some(1), Some(1));
        let result = format_grouped_path(&group, &job, NamingPreset::Jellyfin, "Specials", "Extras");
        assert_eq!(
            result,
            "Breaking Bad (2008)/Season 01/Breaking Bad S01E01 - Pilot.mkv"
        );
    }

    #[test]
    fn test_plex_tv() {
        let group = make_group("Breaking Bad", Some(2008));
        let job = make_job(FileCategory::Episode, Some(1), Some(1));
        let result = format_grouped_path(&group, &job, NamingPreset::Plex, "Specials", "Extras");
        assert_eq!(
            result,
            "Breaking Bad (2008)/Season 01/Breaking Bad (2008) - s01e01 - Pilot.mkv"
        );
    }

    #[test]
    fn test_movie_jellyfin() {
        let group = make_group("The Matrix", Some(1999));
        let mut job = make_job(FileCategory::Movie, None, None);
        job.tmdb_episode_title = None;
        job.media_type = MediaType::Movie;
        let result = format_grouped_path(&group, &job, NamingPreset::Jellyfin, "Specials", "Extras");
        assert_eq!(result, "The Matrix (1999)/The Matrix (1999).mkv");
    }

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize("Test: File?"), "Test File");
        assert_eq!(sanitize("a  b   c"), "a b c");
    }
}
