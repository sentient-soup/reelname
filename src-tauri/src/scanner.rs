use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tracing::warn;

static VIDEO_EXTENSIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".m4v", ".mpg", ".mpeg", ".ts", ".m2ts",
        ".vob", ".iso", ".webm",
    ]
    .into_iter()
    .collect()
});

static SPECIALS_FOLDER_NAMES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ["specials", "season 0", "season 00", "season0", "season00"]
        .into_iter()
        .collect()
});

static EXTRA_FOLDER_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    [
        ("extras", "other"),
        ("extra", "other"),
        ("behind the scenes", "behind_the_scenes"),
        ("behindthescenes", "behind_the_scenes"),
        ("deleted scenes", "deleted_scenes"),
        ("deletedscenes", "deleted_scenes"),
        ("featurettes", "featurettes"),
        ("featurette", "featurettes"),
        ("interviews", "interviews"),
        ("interview", "interviews"),
        ("scenes", "scenes"),
        ("scene", "scenes"),
        ("shorts", "shorts"),
        ("short", "shorts"),
        ("trailers", "trailers"),
        ("trailer", "trailers"),
        ("other", "other"),
    ]
    .into_iter()
    .collect()
});

static SEASON_FOLDER_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?i)^(?:Season\s*|S)(\d+)$").unwrap());

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub source_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub file_extension: String,
}

#[derive(Debug, Clone)]
pub struct ScannedGroupFile {
    pub source_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub file_extension: String,
    pub detected_season: Option<i64>,
    pub file_category: String,
    pub extra_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScannedGroup {
    pub folder_path: String,
    pub folder_name: String,
    pub files: Vec<ScannedGroupFile>,
}

struct SubfolderClassification {
    detected_season: Option<i64>,
    file_category: String,
    extra_type: Option<String>,
}

fn classify_subfolder(folder_name: &str) -> SubfolderClassification {
    let lower = folder_name.to_lowercase().trim().to_string();

    // Check specials
    if SPECIALS_FOLDER_NAMES.contains(lower.as_str()) {
        return SubfolderClassification {
            detected_season: Some(0),
            file_category: "special".into(),
            extra_type: None,
        };
    }

    // Check season pattern
    if let Some(caps) = SEASON_FOLDER_RE.captures(folder_name) {
        if let Ok(num) = caps[1].parse::<i64>() {
            if num == 0 {
                return SubfolderClassification {
                    detected_season: Some(0),
                    file_category: "special".into(),
                    extra_type: None,
                };
            }
            return SubfolderClassification {
                detected_season: Some(num),
                file_category: "episode".into(),
                extra_type: None,
            };
        }
    }

    // Check extras
    if let Some(&extra_type) = EXTRA_FOLDER_MAP.get(lower.as_str()) {
        return SubfolderClassification {
            detected_season: None,
            file_category: "extra".into(),
            extra_type: Some(extra_type.into()),
        };
    }

    SubfolderClassification {
        detected_season: None,
        file_category: "episode".into(),
        extra_type: None,
    }
}

fn collect_video_files(dir: &Path) -> Vec<ScannedFile> {
    let mut results = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Cannot read directory {}: {}", dir.display(), e);
            return results;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(collect_video_files(&path));
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();
            if VIDEO_EXTENSIONS.contains(ext.as_str()) {
                let size = fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
                results.push(ScannedFile {
                    source_path: path.to_string_lossy().to_string(),
                    file_name: entry.file_name().to_string_lossy().to_string(),
                    file_size: size,
                    file_extension: ext,
                });
            }
        }
    }
    results
}

pub fn scan_directory_grouped(dir_path: &str) -> Vec<ScannedGroup> {
    let dir = Path::new(dir_path);
    let mut groups = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Cannot read scan directory {}: {}", dir_path, e);
            return groups;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let mut group = ScannedGroup {
                folder_path: path.to_string_lossy().to_string(),
                folder_name: name,
                files: Vec::new(),
            };

            let sub_entries = match fs::read_dir(&path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let mut has_season_folders = false;

            for sub in sub_entries.flatten() {
                let sub_path = sub.path();
                let sub_name = sub.file_name().to_string_lossy().to_string();

                if sub_path.is_dir() {
                    let classification = classify_subfolder(&sub_name);
                    if classification.file_category == "episode"
                        && classification.detected_season.is_some()
                    {
                        has_season_folders = true;
                    }

                    let files = collect_video_files(&sub_path);
                    for file in files {
                        group.files.push(ScannedGroupFile {
                            source_path: file.source_path,
                            file_name: file.file_name,
                            file_size: file.file_size,
                            file_extension: file.file_extension,
                            detected_season: classification.detected_season,
                            file_category: classification.file_category.clone(),
                            extra_type: classification.extra_type.clone(),
                        });
                    }
                } else if sub_path.is_file() {
                    let ext = sub_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{}", e.to_lowercase()))
                        .unwrap_or_default();
                    if VIDEO_EXTENSIONS.contains(ext.as_str()) {
                        let size = fs::metadata(&sub_path)
                            .map(|m| m.len() as i64)
                            .unwrap_or(0);
                        group.files.push(ScannedGroupFile {
                            source_path: sub_path.to_string_lossy().to_string(),
                            file_name: sub_name,
                            file_size: size,
                            file_extension: ext,
                            detected_season: None,
                            file_category: "episode".into(),
                            extra_type: None,
                        });
                    }
                }
            }

            if !group.files.is_empty() {
                // Media type heuristic: single file, no season folders → movie
                if !has_season_folders
                    && group.files.len() == 1
                    && group.files.iter().all(|f| f.file_category == "episode")
                {
                    group.files[0].file_category = "movie".into();
                }
                groups.push(group);
            }
        } else if path.is_file() {
            // Loose file in scan root → single-file group (movie)
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();
            if VIDEO_EXTENSIONS.contains(ext.as_str()) {
                let size = fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                groups.push(ScannedGroup {
                    folder_path: dir_path.to_string(),
                    folder_name: stem,
                    files: vec![ScannedGroupFile {
                        source_path: path.to_string_lossy().to_string(),
                        file_name: name,
                        file_size: size,
                        file_extension: ext,
                        detected_season: None,
                        file_category: "movie".into(),
                        extra_type: None,
                    }],
                });
            }
        }
    }

    groups
}
