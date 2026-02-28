use crate::core::tmdb::{TmdbClient, TmdbSearchResult};
use crate::db::queries;
use crate::db::schema::*;
use crate::db::DbConn;
use strsim::normalized_levenshtein;
use tracing::{debug, info, warn};

const AUTO_MATCH_GAP: f64 = 0.15;

/// Calculate title similarity (normalized Levenshtein distance).
pub fn title_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    normalized_levenshtein(&a.to_lowercase(), &b.to_lowercase())
}

/// Calculate confidence score for a TMDB result against parsed info.
/// Returns 0.0..1.0.
pub fn calculate_confidence(
    parsed_title: &str,
    parsed_year: Option<i64>,
    parsed_media_type: MediaType,
    result: &TmdbSearchResult,
) -> f64 {
    // Title similarity (60% weight)
    let title_score = title_similarity(parsed_title, result.display_title()) * 0.60;

    // Year score (25% weight)
    let tmdb_year = result.year();
    let year_score = match (parsed_year, tmdb_year) {
        (Some(py), Some(ty)) => {
            let diff = (py - ty).unsigned_abs();
            match diff {
                0 => 0.25,
                1 => 0.15,
                2 => 0.05,
                _ => 0.00,
            }
        }
        (None, _) => 0.10, // neutral
        (_, None) => 0.00,
    };

    // Media type score (10% weight)
    let type_score = if parsed_media_type == MediaType::Unknown {
        0.05 // neutral
    } else {
        let tmdb_type = match result.resolved_media_type() {
            "movie" => MediaType::Movie,
            "tv" => MediaType::Tv,
            _ => MediaType::Unknown,
        };
        if parsed_media_type == tmdb_type {
            0.10
        } else {
            0.00
        }
    };

    // Popularity score (5% weight)
    let pop = result.popularity.unwrap_or(0.0);
    let pop_score = (pop / 100.0).min(1.0) * 0.05;

    title_score + year_score + type_score + pop_score
}

/// Match a group against TMDB. Saves candidates, potentially auto-matches.
pub async fn match_group(
    conn: &DbConn,
    group: &Group,
    tmdb: &TmdbClient,
    threshold: f64,
) -> Result<(), String> {
    let parsed_title = group
        .parsed_title
        .as_deref()
        .unwrap_or(&group.folder_name);

    // Choose search strategy based on media type
    let results = match group.media_type {
        MediaType::Tv => tmdb.search_tv(parsed_title, group.parsed_year).await?,
        MediaType::Movie => tmdb.search_movies(parsed_title, group.parsed_year).await?,
        MediaType::Unknown => tmdb.search_multi(parsed_title, group.parsed_year).await?,
    };

    if results.is_empty() {
        info!("No TMDB results for group {} ({})", group.id, parsed_title);
        return Ok(());
    }

    // Score and sort top 10
    let mut scored: Vec<(TmdbSearchResult, f64)> = results
        .into_iter()
        .take(10)
        .map(|r| {
            let conf = calculate_confidence(parsed_title, group.parsed_year, group.media_type, &r);
            (r, conf)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Delete old candidates
    queries::delete_candidates_for_group(conn, group.id)
        .map_err(|e| format!("DB error: {e}"))?;

    // Save candidates (group-level, job_id = null)
    for (result, confidence) in &scored {
        queries::insert_match_candidate(
            conn,
            group.id,
            result.id,
            MediaType::from_str(result.resolved_media_type()),
            result.display_title(),
            result.year(),
            result.poster_path.as_deref(),
            result.overview.as_deref(),
            *confidence,
        )
        .map_err(|e| format!("DB error: {e}"))?;
    }

    // Auto-match logic
    let top = &scored[0];
    let gap = if scored.len() > 1 {
        top.1 - scored[1].1
    } else {
        1.0 // Only one result, gap is max
    };

    if top.1 >= threshold && gap >= AUTO_MATCH_GAP {
        // Auto-match!
        let result = &top.0;
        let confidence = top.1;

        info!(
            "Auto-matched group {} to '{}' (conf={:.2}, gap={:.2})",
            group.id,
            result.display_title(),
            confidence,
            gap
        );

        let tmdb_id_val = result.id;
        let tmdb_title_val = result.display_title().to_string();
        let tmdb_year_val = result.year();
        let poster_path_val = result.poster_path.clone();
        let media_type_val = MediaType::from_str(result.resolved_media_type());
        let status_str = "matched".to_string();

        // Update group
        queries::update_group(
            conn,
            group.id,
            &[
                ("status", &status_str as &dyn rusqlite::types::ToSql),
                ("tmdb_id", &tmdb_id_val),
                ("tmdb_title", &tmdb_title_val),
                ("tmdb_year", &tmdb_year_val),
                ("tmdb_poster_path", &poster_path_val),
                ("match_confidence", &confidence),
                ("media_type", &media_type_val.as_str().to_string()),
            ],
        )
        .map_err(|e| format!("DB error: {e}"))?;

        // Update all child jobs
        queries::update_jobs_for_group(
            conn,
            group.id,
            &[
                ("status", &status_str as &dyn rusqlite::types::ToSql),
                ("tmdb_id", &tmdb_id_val),
                ("tmdb_title", &tmdb_title_val),
                ("tmdb_year", &tmdb_year_val),
                ("tmdb_poster_path", &poster_path_val),
                ("match_confidence", &confidence),
                ("media_type", &media_type_val.as_str().to_string()),
            ],
        )
        .map_err(|e| format!("DB error: {e}"))?;

        // If TV, fetch episode titles
        if media_type_val == MediaType::Tv {
            if let Err(e) = fetch_episode_titles(conn, group.id, tmdb_id_val, tmdb).await {
                warn!("Failed to fetch episode titles for group {}: {}", group.id, e);
            }
        }
    } else {
        // Set to ambiguous
        let status_str = "ambiguous".to_string();
        queries::update_group(
            conn,
            group.id,
            &[("status", &status_str as &dyn rusqlite::types::ToSql)],
        )
        .map_err(|e| format!("DB error: {e}"))?;

        debug!(
            "Group {} is ambiguous (top conf={:.2}, gap={:.2})",
            group.id, top.1, gap
        );
    }

    Ok(())
}

/// Fetch episode titles from TMDB for all jobs in a group that have parsed season/episode.
pub async fn fetch_episode_titles(
    conn: &DbConn,
    group_id: i64,
    tmdb_id: i64,
    tmdb: &TmdbClient,
) -> Result<(), String> {
    let jobs = queries::fetch_jobs_for_group(conn, group_id)
        .map_err(|e| format!("DB error: {e}"))?;

    for job in &jobs {
        if let (Some(season), Some(episode)) = (job.parsed_season, job.parsed_episode) {
            match tmdb.get_episode(tmdb_id, season, episode).await {
                Ok(ep) => {
                    let title = ep.name;
                    queries::update_job(
                        conn,
                        job.id,
                        &[("tmdb_episode_title", &title as &dyn rusqlite::types::ToSql)],
                    )
                    .map_err(|e| format!("DB error: {e}"))?;
                }
                Err(e) => {
                    debug!(
                        "Could not fetch episode S{:02}E{:02} for job {}: {}",
                        season, episode, job.id, e
                    );
                }
            }
        }
    }

    Ok(())
}
