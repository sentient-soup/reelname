use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

const BASE_URL: &str = "https://api.themoviedb.org/3";
const RATE_LIMIT_MAX: usize = 35;
const RATE_LIMIT_WINDOW_MS: u128 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSearchResult {
    pub id: i64,
    pub title: Option<String>,
    pub name: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub popularity: Option<f64>,
    pub media_type: Option<String>,
    pub vote_average: Option<f64>,
}

impl TmdbSearchResult {
    /// Get display title (title for movies, name for TV).
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Extract year from release_date or first_air_date.
    pub fn year(&self) -> Option<i64> {
        let date_str = self.release_date.as_deref().or(self.first_air_date.as_deref())?;
        if date_str.len() >= 4 {
            date_str[..4].parse().ok()
        } else {
            None
        }
    }

    /// Get resolved media type.
    pub fn resolved_media_type(&self) -> &str {
        self.media_type.as_deref().unwrap_or("unknown")
    }
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbEpisode {
    pub id: i64,
    pub name: String,
    pub episode_number: i64,
    pub season_number: i64,
    pub overview: Option<String>,
    pub still_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSeason {
    pub id: i64,
    pub name: String,
    pub season_number: i64,
    pub episode_count: Option<i64>,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbSeasonDetail {
    pub id: i64,
    pub name: String,
    pub season_number: i64,
    pub episodes: Vec<TmdbEpisode>,
}

#[derive(Debug, Deserialize)]
struct TmdbShowDetail {
    seasons: Vec<TmdbSeason>,
}

/// Rate-limited TMDB API client.
pub struct TmdbClient {
    client: reqwest::Client,
    api_key: String,
    timestamps: Arc<Mutex<Vec<u128>>>,
}

impl TmdbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            timestamps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Wait for rate limit window if needed.
    async fn rate_limit(&self) {
        loop {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let mut ts = self.timestamps.lock().await;
            ts.retain(|&t| now - t < RATE_LIMIT_WINDOW_MS);

            if ts.len() < RATE_LIMIT_MAX {
                ts.push(now);
                return;
            }

            // Need to wait
            let oldest = ts[0];
            let wait = RATE_LIMIT_WINDOW_MS - (now - oldest) + 100;
            drop(ts);
            debug!("TMDB rate limit: waiting {}ms", wait);
            tokio::time::sleep(std::time::Duration::from_millis(wait as u64)).await;
        }
    }

    /// Search multi (movies + TV).
    pub async fn search_multi(
        &self,
        query: &str,
        year: Option<i64>,
    ) -> Result<Vec<TmdbSearchResult>, String> {
        self.rate_limit().await;
        let mut url = format!(
            "{BASE_URL}/search/multi?api_key={}&query={}&include_adult=false",
            self.api_key,
            urlencoding::encode(query)
        );
        if let Some(y) = year {
            url.push_str(&format!("&year={y}"));
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        let body: TmdbSearchResponse = resp
            .json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))?;

        // Filter to movie + tv only
        Ok(body
            .results
            .into_iter()
            .filter(|r| {
                matches!(
                    r.media_type.as_deref(),
                    Some("movie") | Some("tv")
                )
            })
            .collect())
    }

    /// Search movies only.
    pub async fn search_movies(
        &self,
        query: &str,
        year: Option<i64>,
    ) -> Result<Vec<TmdbSearchResult>, String> {
        self.rate_limit().await;
        let mut url = format!(
            "{BASE_URL}/search/movie?api_key={}&query={}&include_adult=false",
            self.api_key,
            urlencoding::encode(query)
        );
        if let Some(y) = year {
            url.push_str(&format!("&year={y}"));
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        let body: TmdbSearchResponse = resp
            .json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))?;

        // Inject media_type = "movie"
        Ok(body
            .results
            .into_iter()
            .map(|mut r| {
                r.media_type = Some("movie".to_string());
                r
            })
            .collect())
    }

    /// Search TV shows only.
    pub async fn search_tv(
        &self,
        query: &str,
        year: Option<i64>,
    ) -> Result<Vec<TmdbSearchResult>, String> {
        self.rate_limit().await;
        let mut url = format!(
            "{BASE_URL}/search/tv?api_key={}&query={}&include_adult=false",
            self.api_key,
            urlencoding::encode(query)
        );
        if let Some(y) = year {
            url.push_str(&format!("&first_air_date_year={y}"));
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        let body: TmdbSearchResponse = resp
            .json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))?;

        // Inject media_type = "tv"
        Ok(body
            .results
            .into_iter()
            .map(|mut r| {
                r.media_type = Some("tv".to_string());
                r
            })
            .collect())
    }

    /// Get seasons list for a TV show.
    pub async fn get_seasons(&self, tv_id: i64) -> Result<Vec<TmdbSeason>, String> {
        self.rate_limit().await;
        let url = format!("{BASE_URL}/tv/{tv_id}?api_key={}", self.api_key);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        let body: TmdbShowDetail = resp
            .json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))?;

        Ok(body.seasons)
    }

    /// Get season detail with episodes.
    pub async fn get_season_detail(
        &self,
        tv_id: i64,
        season_number: i64,
    ) -> Result<TmdbSeasonDetail, String> {
        self.rate_limit().await;
        let url = format!(
            "{BASE_URL}/tv/{tv_id}/season/{season_number}?api_key={}",
            self.api_key
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        resp.json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))
    }

    /// Get a single episode.
    pub async fn get_episode(
        &self,
        tv_id: i64,
        season: i64,
        episode: i64,
    ) -> Result<TmdbEpisode, String> {
        self.rate_limit().await;
        let url = format!(
            "{BASE_URL}/tv/{tv_id}/season/{season}/episode/{episode}?api_key={}",
            self.api_key
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("TMDB request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("TMDB API error: {}", resp.status()));
        }

        resp.json()
            .await
            .map_err(|e| format!("TMDB parse error: {e}"))
    }
}
