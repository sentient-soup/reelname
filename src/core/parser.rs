use regex::Regex;
use std::sync::LazyLock;

use crate::db::schema::MediaType;

#[derive(Debug, Clone, Default)]
pub struct ParsedFile {
    pub title: Option<String>,
    pub year: Option<i64>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub quality: Option<String>,
    pub codec: Option<String>,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedFolder {
    pub title: Option<String>,
    pub year: Option<i64>,
}

// ── Compiled regex patterns ──

struct Patterns {
    // Season/episode (tried in order, first match wins)
    se_patterns: Vec<(Regex, bool)>, // (pattern, has_season)
    year: Regex,
    quality: Vec<Regex>,
    source: Vec<Regex>,
    codec: Vec<Regex>,
    audio: Vec<Regex>,
    misc: Vec<Regex>,
    release_group: Regex,
    bracketed_tags: Regex,
    non_year_parens: Regex,
    dots_underscores: Regex,
    dashes: Regex,
    brackets: Regex,
    multi_spaces: Regex,
    // Folder-specific
    year_in_parens: Regex,
    trailing_year: Regex,
    all_parens: Regex,
}

static PATTERNS: LazyLock<Patterns> = LazyLock::new(|| {
    Patterns {
        se_patterns: vec![
            // 1. S01E01, S01E01E02
            (Regex::new(r"[Ss](\d{1,2})[Ee](\d{1,3})(?:[Ee]\d{1,3})*").unwrap(), true),
            // 2. 1x01, 01x01
            (Regex::new(r"(\d{1,2})[xX](\d{2,3})").unwrap(), true),
            // 3. Season 1 Episode 1
            (Regex::new(r"(?i)[Ss]eason\s*(\d{1,2})\s*[Ee]pisode\s*(\d{1,3})").unwrap(), true),
            // 4. c1_ep3, c01_ep03
            (Regex::new(r"[Cc](\d{1,2})[\s._\-]*[Ee][Pp](\d{1,3})").unwrap(), true),
            // 5. E01, Ep01 (no season -> assume S01)
            (Regex::new(r"(?:^|[\s._\-])[Ee][Pp]?(\d{1,3})(?:[\s._\-]|$)").unwrap(), false),
        ],
        year: Regex::new(r"(?:^|[\s._(\-])(\d{4})(?:[\s._)\-]|$)").unwrap(),
        quality: vec![
            Regex::new(r"\b(2160p|4[Kk]|UHD)\b").unwrap(),
            Regex::new(r"\b(1080p|1080i)\b").unwrap(),
            Regex::new(r"\b(720p)\b").unwrap(),
            Regex::new(r"\b(480p|576p|SD)\b").unwrap(),
        ],
        source: vec![
            Regex::new(r"(?i)\b(Blu-?[Rr]ay|BDRip|BRRip|BDREMUX)\b").unwrap(),
            Regex::new(r"(?i)\b(WEB-?DL|WEBRip|WEBDL|AMZN|NF|DSNP|HMAX|ATVP|PCOK|PMTP)\b").unwrap(),
            Regex::new(r"(?i)\b(DVDRip|DVDR|DVD9|DVD5)\b").unwrap(),
            Regex::new(r"(?i)\b(HDRip|HDTV|PDTV)\b").unwrap(),
            Regex::new(r"(?i)\b(CAM|TS|TC|HDCAM|SCR|SCREENER)\b").unwrap(),
            Regex::new(r"(?i)\b(REMUX)\b").unwrap(),
        ],
        codec: vec![
            Regex::new(r"\b([Hh]\.?265|[Xx]\.?265|HEVC)\b").unwrap(),
            Regex::new(r"\b([Hh]\.?264|[Xx]\.?264|AVC)\b").unwrap(),
            Regex::new(r"(?i)\b(AV1)\b").unwrap(),
            Regex::new(r"(?i)\b(XviD|DivX)\b").unwrap(),
            Regex::new(r"(?i)\b(VP9)\b").unwrap(),
            Regex::new(r"(?i)\b(MPEG-?[24])\b").unwrap(),
        ],
        audio: vec![
            Regex::new(r"(?i)\b(DTS-?HD[\s._\-]?MA|DTS-?HD|DTS-?X|DTS)\b").unwrap(),
            Regex::new(r"(?i)\b(TrueHD[\s._\-]?Atmos|TrueHD|Atmos)\b").unwrap(),
            Regex::new(r"(?i)\b(DD[P+]?\s*5\.1|DDP?7\.1|Dolby\s*Digital|AC-?3|EAC-?3|E-AC-3)\b").unwrap(),
            Regex::new(r"(?i)\b(FLAC|LPCM|PCM)\b").unwrap(),
            Regex::new(r"(?i)\b(AAC[\s._\-]?2\.0|AAC[\s._\-]?5\.1|AAC)\b").unwrap(),
            Regex::new(r"(?i)\b(MP3|OGG|OPUS)\b").unwrap(),
        ],
        misc: vec![
            Regex::new(r"(?i)\b(PROPER|REPACK|RERIP|REAL|INTERNAL|LIMITED|EXTENDED|UNRATED|DC|DIRECTORS[\s._\-]?CUT)\b").unwrap(),
            Regex::new(r"(?i)\b(HDR10\+?|HDR|DV|DoVi|Dolby[\s._\-]?Vision|SDR|HLG)\b").unwrap(),
            Regex::new(r"(?i)\b(10bit|8bit|12bit)\b").unwrap(),
            Regex::new(r"(?i)\b(MULTI|MULTi|DUAL|DUBBED|SUBBED)\b").unwrap(),
            Regex::new(r"(?i)\b(COMPLETE|PROPER|REMASTERED)\b").unwrap(),
        ],
        release_group: Regex::new(r"-([A-Za-z0-9]+)$").unwrap(),
        bracketed_tags: Regex::new(r"\[[^\]]*\]").unwrap(),
        non_year_parens: Regex::new(r"\((?!\d{4}\))[^)]*\)").unwrap(),
        dots_underscores: Regex::new(r"[._]").unwrap(),
        dashes: Regex::new(r"[-–—]").unwrap(),
        brackets: Regex::new(r"[[\](){}]").unwrap(),
        multi_spaces: Regex::new(r"\s+").unwrap(),
        // Folder-specific
        year_in_parens: Regex::new(r"\((\d{4})\)").unwrap(),
        trailing_year: Regex::new(r"[\s._\-](\d{4})(?:[\s._\-]|$)").unwrap(),
        all_parens: Regex::new(r"\([^)]*\)").unwrap(),
    }
});

fn current_year() -> i64 {
    chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026)
}

fn valid_year(y: i64) -> bool {
    (1900..=current_year() + 1).contains(&y)
}

/// Strip the first matching pattern from a list and return the matched text.
fn strip_first_match(text: &mut String, patterns: &[Regex]) -> Option<String> {
    for pat in patterns {
        if let Some(m) = pat.find(text) {
            let matched = if let Some(caps) = pat.captures(text) {
                caps.get(1).map(|c| c.as_str().to_string())
            } else {
                Some(m.as_str().to_string())
            };
            *text = format!("{} {}", &text[..m.start()], &text[m.end()..]);
            return matched;
        }
    }
    None
}

/// Strip all matches of patterns from text (used for misc patterns).
fn strip_all_matches(text: &mut String, patterns: &[Regex]) {
    for pat in patterns {
        *text = pat.replace_all(text, " ").to_string();
    }
}

pub fn parse_file_name(file_name: &str) -> ParsedFile {
    let p = &*PATTERNS;
    let mut result = ParsedFile::default();

    // 1. Strip file extension
    let mut work = if let Some(pos) = file_name.rfind('.') {
        file_name[..pos].to_string()
    } else {
        file_name.to_string()
    };

    // 2. Strip bracketed tags [...]
    work = p.bracketed_tags.replace_all(&work, " ").to_string();

    // 3. Strip non-year parenthesized tags
    work = p.non_year_parens.replace_all(&work, " ").to_string();

    // 4. Replace dots and underscores with spaces
    work = p.dots_underscores.replace_all(&work, " ").to_string();

    // 5. Strip release group
    work = p.release_group.replace(&work, "").to_string();

    // 6. Extract season/episode
    for (pat, has_season) in &p.se_patterns {
        if let Some(caps) = pat.captures(&work) {
            if *has_season {
                result.season = caps.get(1).and_then(|m| m.as_str().parse().ok());
                result.episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
            } else {
                result.season = Some(1);
                result.episode = caps.get(1).and_then(|m| m.as_str().parse().ok());
            }
            // Remove the matched pattern from work
            if let Some(m) = pat.find(&work) {
                work = format!("{}{}", &work[..m.start()], &work[m.end()..]);
            }
            break;
        }
    }

    // 7. Extract year
    if let Some(caps) = p.year.captures(&work) {
        if let Some(year_str) = caps.get(1) {
            if let Ok(y) = year_str.as_str().parse::<i64>() {
                if valid_year(y) {
                    result.year = Some(y);
                    // Remove year from work
                    if let Some(m) = p.year.find(&work) {
                        work = format!("{}{}", &work[..m.start()], &work[m.end()..]);
                    }
                }
            }
        }
    }

    // 8. Extract quality
    result.quality = strip_first_match(&mut work, &p.quality);

    // 9. Extract source (stripped but not stored)
    let _ = strip_first_match(&mut work, &p.source);

    // 10. Extract codec
    result.codec = strip_first_match(&mut work, &p.codec);

    // 11. Extract audio (stripped but not stored)
    let _ = strip_first_match(&mut work, &p.audio);

    // 12. Strip misc patterns
    strip_all_matches(&mut work, &p.misc);

    // 13. Clean title
    work = p.dashes.replace_all(&work, " ").to_string();
    work = p.brackets.replace_all(&work, "").to_string();
    work = p.multi_spaces.replace_all(&work, " ").to_string();
    let title = work.trim().to_string();
    if !title.is_empty() {
        result.title = Some(title);
    }

    // 14. Media type heuristic
    result.media_type = if result.season.is_some() || result.episode.is_some() {
        MediaType::Tv
    } else if result.year.is_some() {
        MediaType::Movie
    } else {
        MediaType::Unknown
    };

    result
}

pub fn parse_folder_name(folder_name: &str) -> ParsedFolder {
    let p = &*PATTERNS;
    let mut result = ParsedFolder::default();

    // 1. Strip bracketed tags
    let mut work = p.bracketed_tags.replace_all(folder_name, "").to_string();

    // 2. Extract year in parens first (priority)
    if let Some(caps) = p.year_in_parens.captures(&work) {
        if let Some(year_str) = caps.get(1) {
            if let Ok(y) = year_str.as_str().parse::<i64>() {
                if valid_year(y) {
                    result.year = Some(y);
                }
            }
        }
    }

    // 3. If no year, try trailing year
    if result.year.is_none() {
        if let Some(caps) = p.trailing_year.captures(&work) {
            if let Some(year_str) = caps.get(1) {
                if let Ok(y) = year_str.as_str().parse::<i64>() {
                    if valid_year(y) {
                        result.year = Some(y);
                    }
                }
            }
        }
    }

    // 4. Strip remaining parens
    work = p.all_parens.replace_all(&work, "").to_string();

    // 5. Clean title
    work = p.dots_underscores.replace_all(&work, " ").to_string();
    work = p.dashes.replace_all(&work, " ").to_string();
    work = p.brackets.replace_all(&work, "").to_string();
    work = p.multi_spaces.replace_all(&work, " ").to_string();

    let title = work.trim().to_string();
    if !title.is_empty() {
        result.title = Some(title);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_tv() {
        let r = parse_file_name("Breaking.Bad.S01E01.720p.BluRay.x264-DEMAND.mkv");
        assert_eq!(r.season, Some(1));
        assert_eq!(r.episode, Some(1));
        assert_eq!(r.quality, Some("720p".to_string()));
        assert_eq!(r.codec, Some("x264".to_string()));
        assert_eq!(r.media_type, MediaType::Tv);
        assert!(r.title.as_ref().unwrap().contains("Breaking Bad"));
    }

    #[test]
    fn test_movie_with_year() {
        let r = parse_file_name("The.Matrix.1999.1080p.BluRay.x264.mkv");
        assert_eq!(r.year, Some(1999));
        assert_eq!(r.quality, Some("1080p".to_string()));
        assert_eq!(r.media_type, MediaType::Movie);
        assert!(r.title.as_ref().unwrap().contains("The Matrix"));
    }

    #[test]
    fn test_episode_only() {
        let r = parse_file_name("Show.Name.E05.720p.mkv");
        assert_eq!(r.season, Some(1));
        assert_eq!(r.episode, Some(5));
        assert_eq!(r.media_type, MediaType::Tv);
    }

    #[test]
    fn test_2x03_format() {
        let r = parse_file_name("Show.Name.2x03.mkv");
        assert_eq!(r.season, Some(2));
        assert_eq!(r.episode, Some(3));
    }

    #[test]
    fn test_4k_quality() {
        let r = parse_file_name("Movie.2020.2160p.WEB-DL.x265.mkv");
        assert_eq!(r.quality, Some("2160p".to_string()));
        assert_eq!(r.year, Some(2020));
    }

    #[test]
    fn test_folder_with_year_parens() {
        let r = parse_folder_name("The Matrix (1999)");
        assert_eq!(r.title, Some("The Matrix".to_string()));
        assert_eq!(r.year, Some(1999));
    }

    #[test]
    fn test_folder_dots() {
        let r = parse_folder_name("Breaking.Bad.2008");
        assert_eq!(r.title, Some("Breaking Bad".to_string()));
        assert_eq!(r.year, Some(2008));
    }
}
