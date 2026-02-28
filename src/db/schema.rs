use serde::{Deserialize, Serialize};
use std::fmt;

// ── Enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GroupStatus {
    #[default]
    Scanned,
    Matched,
    Ambiguous,
    Confirmed,
    Transferring,
    Completed,
    Failed,
    Skipped,
}

impl GroupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scanned => "scanned",
            Self::Matched => "matched",
            Self::Ambiguous => "ambiguous",
            Self::Confirmed => "confirmed",
            Self::Transferring => "transferring",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "scanned" => Self::Scanned,
            "matched" => Self::Matched,
            "ambiguous" => Self::Ambiguous,
            "confirmed" => Self::Confirmed,
            "transferring" => Self::Transferring,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Scanned,
        }
    }

    pub const ALL: &[GroupStatus] = &[
        Self::Scanned,
        Self::Matched,
        Self::Ambiguous,
        Self::Confirmed,
        Self::Transferring,
        Self::Completed,
        Self::Failed,
        Self::Skipped,
    ];
}

impl fmt::Display for GroupStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MediaType {
    Movie,
    Tv,
    #[default]
    Unknown,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Tv => "tv",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "movie" => Self::Movie,
            "tv" => Self::Tv,
            _ => Self::Unknown,
        }
    }

    pub const ALL: &[MediaType] = &[Self::Movie, Self::Tv, Self::Unknown];
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FileCategory {
    #[default]
    Episode,
    Movie,
    Special,
    Extra,
}

impl FileCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Episode => "episode",
            Self::Movie => "movie",
            Self::Special => "special",
            Self::Extra => "extra",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "episode" => Self::Episode,
            "movie" => Self::Movie,
            "special" => Self::Special,
            "extra" => Self::Extra,
            _ => Self::Episode,
        }
    }
}

impl fmt::Display for FileCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtraType {
    BehindTheScenes,
    DeletedScenes,
    Featurettes,
    Interviews,
    Scenes,
    Shorts,
    Trailers,
    Other,
}

impl ExtraType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BehindTheScenes => "behind_the_scenes",
            Self::DeletedScenes => "deleted_scenes",
            Self::Featurettes => "featurettes",
            Self::Interviews => "interviews",
            Self::Scenes => "scenes",
            Self::Shorts => "shorts",
            Self::Trailers => "trailers",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "behind_the_scenes" => Self::BehindTheScenes,
            "deleted_scenes" => Self::DeletedScenes,
            "featurettes" => Self::Featurettes,
            "interviews" => Self::Interviews,
            "scenes" => Self::Scenes,
            "shorts" => Self::Shorts,
            "trailers" => Self::Trailers,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for ExtraType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DestinationType {
    #[default]
    Local,
    Ssh,
}

impl DestinationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ssh => "ssh",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "ssh" => Self::Ssh,
            _ => Self::Local,
        }
    }
}

impl fmt::Display for DestinationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Table structs ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub status: GroupStatus,
    pub media_type: MediaType,

    // Source info
    pub folder_path: String,
    pub folder_name: String,
    pub total_file_count: i64,
    pub total_file_size: i64,

    // Parsed info
    pub parsed_title: Option<String>,
    pub parsed_year: Option<i64>,

    // TMDB info
    pub tmdb_id: Option<i64>,
    pub tmdb_title: Option<String>,
    pub tmdb_year: Option<i64>,
    pub tmdb_poster_path: Option<String>,
    pub match_confidence: Option<f64>,

    // Transfer
    pub destination_id: Option<i64>,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub group_id: Option<i64>,
    pub status: GroupStatus,
    pub media_type: MediaType,
    pub file_category: FileCategory,
    pub extra_type: Option<ExtraType>,

    // Source info
    pub source_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub file_extension: String,

    // Parsed info
    pub parsed_title: Option<String>,
    pub parsed_year: Option<i64>,
    pub parsed_season: Option<i64>,
    pub parsed_episode: Option<i64>,
    pub parsed_quality: Option<String>,
    pub parsed_codec: Option<String>,

    // TMDB info
    pub tmdb_id: Option<i64>,
    pub tmdb_title: Option<String>,
    pub tmdb_year: Option<i64>,
    pub tmdb_poster_path: Option<String>,
    pub tmdb_episode_title: Option<String>,
    pub match_confidence: Option<f64>,

    // Transfer info
    pub destination_id: Option<i64>,
    pub destination_path: Option<String>,
    pub transfer_progress: Option<f64>,
    pub transfer_error: Option<String>,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCandidate {
    pub id: i64,
    pub job_id: Option<i64>,
    pub group_id: Option<i64>,
    pub tmdb_id: i64,
    pub media_type: MediaType,
    pub title: String,
    pub year: Option<i64>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub id: i64,
    pub name: String,
    pub dest_type: DestinationType,
    pub base_path: String,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub movie_template: Option<String>,
    pub tv_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

// ── Composite types for UI ──

#[derive(Debug, Clone)]
pub struct JobWithPreview {
    pub job: Job,
    pub preview_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GroupWithJobs {
    pub group: Group,
    pub jobs: Vec<JobWithPreview>,
    pub candidates: Vec<MatchCandidate>,
}
