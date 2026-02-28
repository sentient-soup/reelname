use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tokio::time::sleep;
use tracing::debug;

const TMDB_BASE: &str = "https://api.themoviedb.org/3";
const RATE_LIMIT: usize = 35;
const RATE_WINDOW_MS: u128 = 10_000;

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());
static REQUEST_TIMESTAMPS: Lazy<Mutex<Vec<Instant>>> = Lazy::new(|| Mutex::new(Vec::new()));

async fn rate_limited_get(url: &str) -> Result<reqwest::Response> {
    // Check rate limit and compute wait time (drop guard before any .await)
    let wait_ms = {
        let mut timestamps = REQUEST_TIMESTAMPS.lock().unwrap();
        let now = Instant::now();
        timestamps.retain(|t| now.duration_since(*t).as_millis() < RATE_WINDOW_MS);

        if timestamps.len() >= RATE_LIMIT {
            let wait = RATE_WINDOW_MS - now.duration_since(timestamps[0]).as_millis();
            Some(wait)
        } else {
            timestamps.push(now);
            None
        }
    }; // MutexGuard dropped here

    if let Some(wait) = wait_ms {
        debug!("Rate limit reached, waiting {}ms", wait);
        sleep(std::time::Duration::from_millis(wait as u64)).await;
        let mut timestamps = REQUEST_TIMESTAMPS.lock().unwrap();
        timestamps.push(Instant::now());
    }

    let res = HTTP_CLIENT.get(url).send().await?;
    Ok(res)
}

fn get_api_key() -> Result<String> {
    crate::db::get_setting("tmdb_api_key")
        .filter(|k| !k.is_empty())
        .ok_or_else(|| anyhow!("TMDB API key not configured"))
}

// ── Response types ──────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbSeason {
    pub id: i64,
    pub name: String,
    pub season_number: i64,
    pub episode_count: i64,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbSeasonDetail {
    pub id: i64,
    pub name: String,
    pub season_number: i64,
    pub episodes: Vec<TmdbEpisode>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbEpisode {
    pub id: i64,
    pub name: String,
    pub episode_number: i64,
    pub season_number: i64,
    pub overview: Option<String>,
    pub still_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TvDetailResponse {
    seasons: Option<Vec<TmdbSeason>>,
}

// ── Public API ──────────────────────────────────────────

pub async fn search_multi(query: &str, year: Option<i64>) -> Result<Vec<TmdbSearchResult>> {
    let api_key = get_api_key()?;
    let mut url = format!(
        "{}/search/multi?api_key={}&query={}&include_adult=false",
        TMDB_BASE,
        api_key,
        urlencoding::encode(query)
    );
    if let Some(y) = year {
        url.push_str(&format!("&year={}", y));
    }

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Err(anyhow!("TMDB API error: {}", res.status()));
    }

    let data: SearchResponse = res.json().await?;
    Ok(data
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

pub async fn search_movies(query: &str, year: Option<i64>) -> Result<Vec<TmdbSearchResult>> {
    let api_key = get_api_key()?;
    let mut url = format!(
        "{}/search/movie?api_key={}&query={}&include_adult=false",
        TMDB_BASE,
        api_key,
        urlencoding::encode(query)
    );
    if let Some(y) = year {
        url.push_str(&format!("&year={}", y));
    }

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Err(anyhow!("TMDB API error: {}", res.status()));
    }

    let data: SearchResponse = res.json().await?;
    Ok(data
        .results
        .into_iter()
        .map(|mut r| {
            r.media_type = Some("movie".to_string());
            r
        })
        .collect())
}

pub async fn search_tv(query: &str, year: Option<i64>) -> Result<Vec<TmdbSearchResult>> {
    let api_key = get_api_key()?;
    let mut url = format!(
        "{}/search/tv?api_key={}&query={}&include_adult=false",
        TMDB_BASE,
        api_key,
        urlencoding::encode(query)
    );
    if let Some(y) = year {
        url.push_str(&format!("&first_air_date_year={}", y));
    }

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Err(anyhow!("TMDB API error: {}", res.status()));
    }

    let data: SearchResponse = res.json().await?;
    Ok(data
        .results
        .into_iter()
        .map(|mut r| {
            r.media_type = Some("tv".to_string());
            r
        })
        .collect())
}

pub async fn get_show_seasons(tv_id: i64) -> Result<Vec<TmdbSeason>> {
    let api_key = get_api_key()?;
    let url = format!("{}/tv/{}?api_key={}", TMDB_BASE, tv_id, api_key);

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Err(anyhow!("TMDB API error: {}", res.status()));
    }

    let data: TvDetailResponse = res.json().await?;
    Ok(data.seasons.unwrap_or_default())
}

pub async fn get_season(tv_id: i64, season_number: i64) -> Result<Option<TmdbSeasonDetail>> {
    let api_key = get_api_key()?;
    let url = format!(
        "{}/tv/{}/season/{}?api_key={}",
        TMDB_BASE, tv_id, season_number, api_key
    );

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Ok(None);
    }

    let data: TmdbSeasonDetail = res.json().await?;
    Ok(Some(data))
}

pub async fn get_episode(
    tv_id: i64,
    season: i64,
    episode: i64,
) -> Result<Option<TmdbEpisode>> {
    let api_key = get_api_key()?;
    let url = format!(
        "{}/tv/{}/season/{}/episode/{}?api_key={}",
        TMDB_BASE, tv_id, season, episode, api_key
    );

    let res = rate_limited_get(&url).await?;
    if !res.status().is_success() {
        return Ok(None);
    }

    let data: TmdbEpisode = res.json().await?;
    Ok(Some(data))
}
