use std::collections::HashMap;
use std::path::Path;
use tracing::info;
use walkdir::WalkDir;

use crate::db::schema::{ExtraType, FileCategory};

/// Recognized video file extensions.
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "ts", "m2ts", "vob", "iso",
    "webm",
];

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub source_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_extension: String,
}

#[derive(Debug, Clone)]
pub struct ScannedGroupFile {
    pub source_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_extension: String,
    pub detected_season: Option<i64>,
    pub file_category: FileCategory,
    pub extra_type: Option<ExtraType>,
}

#[derive(Debug, Clone)]
pub struct ScannedGroup {
    pub folder_path: String,
    pub folder_name: String,
    pub files: Vec<ScannedGroupFile>,
}

/// Check if a file extension is a recognized video format.
fn is_video_extension(ext: &str) -> bool {
    VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

/// Season folder regex: "Season 1", "Season 01", "S1", "S01"
fn parse_season_folder(name: &str) -> Option<i64> {
    let re = regex::Regex::new(r"(?i)^(?:Season\s*|S)(\d+)$").unwrap();
    re.captures(name)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Check if folder name indicates specials.
fn is_specials_folder(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "specials" | "season 0" | "season 00" | "season0" | "season00"
    )
}

/// Map extra folder names to ExtraType values.
fn classify_extra_folder(name: &str) -> Option<ExtraType> {
    static EXTRA_MAP: std::sync::LazyLock<HashMap<&str, ExtraType>> =
        std::sync::LazyLock::new(|| {
            let mut m = HashMap::new();
            m.insert("extras", ExtraType::Other);
            m.insert("extra", ExtraType::Other);
            m.insert("behind the scenes", ExtraType::BehindTheScenes);
            m.insert("behindthescenes", ExtraType::BehindTheScenes);
            m.insert("deleted scenes", ExtraType::DeletedScenes);
            m.insert("deletedscenes", ExtraType::DeletedScenes);
            m.insert("featurettes", ExtraType::Featurettes);
            m.insert("featurette", ExtraType::Featurettes);
            m.insert("interviews", ExtraType::Interviews);
            m.insert("interview", ExtraType::Interviews);
            m.insert("scenes", ExtraType::Scenes);
            m.insert("scene", ExtraType::Scenes);
            m.insert("shorts", ExtraType::Shorts);
            m.insert("short", ExtraType::Shorts);
            m.insert("trailers", ExtraType::Trailers);
            m.insert("trailer", ExtraType::Trailers);
            m.insert("other", ExtraType::Other);
            m
        });

    EXTRA_MAP.get(name.to_lowercase().as_str()).copied()
}

/// Classify a subfolder: returns (season_number, file_category, extra_type)
fn classify_subfolder(name: &str) -> (Option<i64>, FileCategory, Option<ExtraType>) {
    // Check season folder first
    if let Some(season) = parse_season_folder(name) {
        return (Some(season), FileCategory::Episode, None);
    }
    // Check specials
    if is_specials_folder(name) {
        return (Some(0), FileCategory::Special, None);
    }
    // Check extras
    if let Some(extra_type) = classify_extra_folder(name) {
        return (None, FileCategory::Extra, Some(extra_type));
    }
    // Unknown subfolder - treat as episode files
    (None, FileCategory::Episode, None)
}

/// Scan a directory and return grouped results.
/// Each top-level directory in `scan_root` = one Group.
/// Loose video files at scan root = single-file Groups.
pub fn scan_directory_grouped(scan_root: &Path) -> Vec<ScannedGroup> {
    let mut groups: Vec<ScannedGroup> = Vec::new();

    // Collect all entries at the scan root level
    let entries = match std::fs::read_dir(scan_root) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to read scan directory {}: {}", scan_root.display(), e);
            return groups;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            // Top-level directory = one group
            let group = scan_group_folder(&path, &file_name);
            if !group.files.is_empty() {
                groups.push(group);
            }
        } else if path.is_file() {
            // Loose video file at scan root = single-file group
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            if is_video_extension(&ext) {
                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                groups.push(ScannedGroup {
                    folder_path: path.to_string_lossy().to_string(),
                    folder_name: file_name.clone(),
                    files: vec![ScannedGroupFile {
                        source_path: path.to_string_lossy().to_string(),
                        file_name,
                        file_size,
                        file_extension: ext,
                        detected_season: None,
                        file_category: FileCategory::Movie,
                        extra_type: None,
                    }],
                });
            }
        }
    }

    info!("Scanned {} groups from {}", groups.len(), scan_root.display());
    groups
}

/// Scan a single group folder (one top-level directory).
fn scan_group_folder(folder: &Path, folder_name: &str) -> ScannedGroup {
    let mut files: Vec<ScannedGroupFile> = Vec::new();
    let mut has_season_folders = false;

    // Read direct children
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(_) => {
            return ScannedGroup {
                folder_path: folder.to_string_lossy().to_string(),
                folder_name: folder_name.to_string(),
                files,
            };
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            // Classify subfolder
            let (season, category, extra_type) = classify_subfolder(&name);

            if season.is_some() && category == FileCategory::Episode {
                has_season_folders = true;
            }

            // Walk the subfolder for video files
            for sub_entry in WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let sub_path = sub_entry.path();
                let ext = sub_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();
                if is_video_extension(&ext) {
                    let file_size = sub_entry.metadata().map(|m| m.len()).unwrap_or(0);
                    files.push(ScannedGroupFile {
                        source_path: sub_path.to_string_lossy().to_string(),
                        file_name: sub_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        file_size,
                        file_extension: ext,
                        detected_season: season,
                        file_category: category,
                        extra_type,
                    });
                }
            }
        } else if path.is_file() {
            // Direct video file in group folder
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            if is_video_extension(&ext) {
                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(ScannedGroupFile {
                    source_path: path.to_string_lossy().to_string(),
                    file_name: name,
                    file_size,
                    file_extension: ext,
                    detected_season: None,
                    file_category: FileCategory::Episode,
                    extra_type: None,
                });
            }
        }
    }

    // Movie heuristic: no season folders, exactly 1 file, all files are "episode" category
    if !has_season_folders
        && files.len() == 1
        && files.iter().all(|f| f.file_category == FileCategory::Episode)
    {
        files[0].file_category = FileCategory::Movie;
    }

    ScannedGroup {
        folder_path: folder.to_string_lossy().to_string(),
        folder_name: folder_name.to_string(),
        files,
    }
}
