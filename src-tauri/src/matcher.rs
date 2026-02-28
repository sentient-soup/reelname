use anyhow::Result;
use rusqlite::params;
use tracing::{info, warn};

use crate::db;
use crate::models::{Group, MatchResult};
use crate::tmdb::{self, TmdbSearchResult};

/// Calculate title similarity using strsim's normalized Levenshtein
fn title_similarity(a: &str, b: &str) -> f64 {
    let s1 = a.to_lowercase();
    let s1 = s1.trim();
    let s2 = b.to_lowercase();
    let s2 = s2.trim();

    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    strsim::normalized_levenshtein(s1, s2)
}

/// Calculate confidence score for a TMDB match
fn calculate_confidence(
    parsed_title: &str,
    parsed_year: Option<i64>,
    parsed_media_type: &str,
    result: &TmdbSearchResult,
) -> f64 {
    let tmdb_title = result
        .title
        .as_deref()
        .or(result.name.as_deref())
        .unwrap_or("");
    let tmdb_year: Option<i64> = result
        .release_date
        .as_deref()
        .or(result.first_air_date.as_deref())
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse().ok());
    let tmdb_media_type = result.media_type.as_deref().unwrap_or("unknown");

    // Title similarity: 60% weight
    let title_score = title_similarity(parsed_title, tmdb_title) * 0.6;

    // Year match: 25% weight
    let year_score = match (parsed_year, tmdb_year) {
        (Some(py), Some(ty)) => {
            let diff = (py - ty).unsigned_abs();
            if diff == 0 {
                0.25
            } else if diff == 1 {
                0.15
            } else if diff == 2 {
                0.05
            } else {
                0.0
            }
        }
        (None, _) => 0.1, // neutral when we don't have a year
        _ => 0.0,
    };

    // Media type consistency: 10% weight
    let type_score = if parsed_media_type == "unknown" {
        0.05
    } else if (parsed_media_type == "tv" && tmdb_media_type == "tv")
        || (parsed_media_type == "movie" && tmdb_media_type == "movie")
    {
        0.1
    } else {
        0.0
    };

    // Popularity tiebreaker: 5% weight
    let pop = result.popularity.unwrap_or(0.0);
    let pop_score = (pop / 100.0).min(1.0) * 0.05;

    title_score + year_score + type_score + pop_score
}

/// Match a single group against TMDB
pub async fn match_group(group: &Group) -> Result<()> {
    let parsed_title = match &group.parsed_title {
        Some(t) if !t.is_empty() => t.clone(),
        _ => {
            let db = db::conn();
            db.execute(
                "UPDATE groups SET status = 'ambiguous', updated_at = ?1 WHERE id = ?2",
                params![db::now_iso(), group.id],
            )?;
            return Ok(());
        }
    };

    let year = group.parsed_year;

    // Search TMDB based on media type
    let results = match group.media_type.as_str() {
        "tv" => tmdb::search_tv(&parsed_title, year).await?,
        "movie" => tmdb::search_movies(&parsed_title, year).await?,
        _ => tmdb::search_multi(&parsed_title, year).await?,
    };

    if results.is_empty() {
        let db = db::conn();
        db.execute(
            "UPDATE groups SET status = 'ambiguous', updated_at = ?1 WHERE id = ?2",
            params![db::now_iso(), group.id],
        )?;
        return Ok(());
    }

    // Score all results (top 10)
    let mut scored: Vec<(TmdbSearchResult, f64)> = results
        .into_iter()
        .take(10)
        .map(|r| {
            let conf = calculate_confidence(&parsed_title, year, &group.media_type, &r);
            (r, conf)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Save candidates at group level
    {
        let db = db::conn();
        db.execute(
            "DELETE FROM match_candidates WHERE group_id = ?1",
            params![group.id],
        )?;

        for (result, confidence) in &scored {
            let tmdb_title = result.title.as_deref().or(result.name.as_deref()).unwrap_or("");
            let tmdb_year: Option<i64> = result
                .release_date
                .as_deref()
                .or(result.first_air_date.as_deref())
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse().ok());
            let media_type = result.media_type.as_deref().unwrap_or("movie");
            let overview = result
                .overview
                .as_ref()
                .map(|o| if o.len() > 500 { &o[..500] } else { o.as_str() });

            db.execute(
                "INSERT INTO match_candidates (group_id, job_id, tmdb_id, media_type, title, year, poster_path, overview, confidence)
                 VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    group.id,
                    result.id,
                    media_type,
                    tmdb_title,
                    tmdb_year,
                    result.poster_path,
                    overview,
                    confidence,
                ],
            )?;
        }
    }

    // Auto-match logic
    let threshold: f64 = db::get_setting("auto_match_threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.85);

    let top = &scored[0];
    let gap = if scored.len() > 1 {
        top.1 - scored[1].1
    } else {
        1.0
    };

    let now = db::now_iso();

    // Determine whether to fetch episode titles (computed before any .await)
    let mut fetch_episodes_for: Option<i64> = None;

    if top.1 >= threshold && gap >= 0.15 {
        let tmdb_title = top
            .0
            .title
            .as_deref()
            .or(top.0.name.as_deref())
            .unwrap_or("")
            .to_string();
        let tmdb_year: Option<i64> = top
            .0
            .release_date
            .as_deref()
            .or(top.0.first_air_date.as_deref())
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse().ok());
        let media_type = top
            .0
            .media_type
            .as_deref()
            .unwrap_or(&group.media_type)
            .to_string();

        {
            let db = db::conn();

            // Update group with match
            db.execute(
                "UPDATE groups SET status = 'matched', tmdb_id = ?1, tmdb_title = ?2, tmdb_year = ?3,
                 tmdb_poster_path = ?4, match_confidence = ?5, media_type = ?6, updated_at = ?7
                 WHERE id = ?8",
                params![
                    top.0.id,
                    tmdb_title,
                    tmdb_year,
                    top.0.poster_path,
                    top.1,
                    media_type,
                    now,
                    group.id,
                ],
            )?;

            // Update child jobs
            db.execute(
                "UPDATE jobs SET status = 'matched', tmdb_id = ?1, tmdb_title = ?2, tmdb_year = ?3,
                 tmdb_poster_path = ?4, match_confidence = ?5, updated_at = ?6
                 WHERE group_id = ?7",
                params![top.0.id, tmdb_title, tmdb_year, top.0.poster_path, top.1, now, group.id],
            )?;
        } // db guard dropped here

        if media_type == "tv" {
            fetch_episodes_for = Some(top.0.id);
        }
    } else {
        let db = db::conn();
        db.execute(
            "UPDATE groups SET status = 'ambiguous', match_confidence = ?1, updated_at = ?2 WHERE id = ?3",
            params![top.1, now, group.id],
        )?;
    }

    // Now safe to .await since no MutexGuard is alive
    if let Some(tid) = fetch_episodes_for {
        fetch_episode_titles(group.id, tid).await;
    }

    Ok(())
}

/// Fetch episode titles from TMDB for all jobs in a TV group
pub async fn fetch_episode_titles(group_id: i64, tmdb_id: i64) {
    let jobs = db::get_jobs_for_group(group_id);

    for job in jobs {
        if job.file_category == "extra" {
            continue;
        }
        let (season, episode) = match (job.parsed_season, job.parsed_episode) {
            (Some(s), Some(e)) => (s, e),
            _ => continue,
        };

        match tmdb::get_episode(tmdb_id, season, episode).await {
            Ok(Some(ep)) => {
                let db = db::conn();
                let _ = db.execute(
                    "UPDATE jobs SET tmdb_episode_title = ?1, updated_at = ?2 WHERE id = ?3",
                    params![ep.name, db::now_iso(), job.id],
                );
            }
            Ok(None) => {}
            Err(e) => {
                warn!("Failed to fetch episode S{}E{}: {}", season, episode, e);
            }
        }
    }
}

/// Match all unmatched groups
pub async fn match_all_groups() -> Result<MatchResult> {
    let unmatched: Vec<Group> = {
        let db = db::conn();
        let mut stmt = db.prepare("SELECT * FROM groups WHERE status = 'scanned'")?;
        let result: Vec<Group> = stmt.query_map([], db::row_to_group)?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    let mut matched = 0usize;
    let mut ambiguous = 0usize;

    for group in &unmatched {
        match match_group(group).await {
            Ok(()) => {
                let updated = db::get_group_by_id(group.id);
                if updated.map(|g| g.status == "matched").unwrap_or(false) {
                    matched += 1;
                } else {
                    ambiguous += 1;
                }
            }
            Err(e) => {
                warn!("Failed to match group {}: {}", group.id, e);
                ambiguous += 1;
            }
        }
    }

    info!("Matching complete: {} matched, {} ambiguous", matched, ambiguous);
    Ok(MatchResult { matched, ambiguous })
}
