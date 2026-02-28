use serde::{Deserialize, Serialize};

// ── Group ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub status: String,
    pub media_type: String,
    pub folder_path: String,
    pub folder_name: String,
    pub total_file_count: i64,
    pub total_file_size: i64,
    pub parsed_title: Option<String>,
    pub parsed_year: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub tmdb_title: Option<String>,
    pub tmdb_year: Option<i64>,
    pub tmdb_poster_path: Option<String>,
    pub match_confidence: Option<f64>,
    pub destination_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Job ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: i64,
    pub group_id: Option<i64>,
    pub status: String,
    pub media_type: String,
    pub file_category: String,
    pub extra_type: Option<String>,
    pub source_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub file_extension: String,
    pub parsed_title: Option<String>,
    pub parsed_year: Option<i64>,
    pub parsed_season: Option<i64>,
    pub parsed_episode: Option<i64>,
    pub parsed_quality: Option<String>,
    pub parsed_codec: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tmdb_title: Option<String>,
    pub tmdb_year: Option<i64>,
    pub tmdb_poster_path: Option<String>,
    pub tmdb_episode_title: Option<String>,
    pub match_confidence: Option<f64>,
    pub destination_id: Option<i64>,
    pub destination_path: Option<String>,
    pub transfer_progress: Option<f64>,
    pub transfer_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobWithPreview {
    #[serde(flatten)]
    pub job: Job,
    pub preview_name: Option<String>,
}

// ── Match Candidate ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub id: i64,
    pub job_id: Option<i64>,
    pub group_id: Option<i64>,
    pub tmdb_id: i64,
    pub media_type: String,
    pub title: String,
    pub year: Option<i64>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub confidence: f64,
}

// ── Destination ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub dest_type: String,
    pub base_path: String,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub movie_template: Option<String>,
    pub tv_template: Option<String>,
}

// ── Setting ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

// ── Composite response types ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupWithJobs {
    #[serde(flatten)]
    pub group: Group,
    pub jobs: Vec<JobWithPreview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<MatchCandidate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupsResponse {
    pub groups: Vec<GroupWithJobs>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scanned_groups: usize,
    pub added_groups: usize,
    pub added_files: usize,
    pub skipped_groups: usize,
    pub matched: usize,
    pub ambiguous: usize,
    pub match_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub matched: usize,
    pub ambiguous: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub id: i64,
    pub status: String,
    pub file_name: String,
    pub file_size: i64,
    pub transfer_progress: Option<f64>,
    pub transfer_error: Option<String>,
    pub destination_path: Option<String>,
}

// ── Command input types ─────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDestinationInput {
    pub name: String,
    #[serde(rename = "type")]
    pub dest_type: Option<String>,
    pub base_path: String,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub movie_template: Option<String>,
    pub tv_template: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDestinationInput {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub dest_type: Option<String>,
    pub base_path: Option<String>,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub movie_template: Option<String>,
    pub tv_template: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSshInput {
    pub ssh_host: String,
    pub ssh_port: Option<i64>,
    pub ssh_user: String,
    pub ssh_key_path: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub base_path: Option<String>,
}
