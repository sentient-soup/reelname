use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub title: String,
    pub year: Option<i64>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub quality: Option<String>,
    pub codec: Option<String>,
    pub source: Option<String>,
    pub audio: Option<String>,
    pub media_type: String, // "movie" | "tv" | "unknown"
}

#[derive(Debug, Clone)]
pub struct ParsedFolder {
    pub title: String,
    pub year: Option<i64>,
}

// Season/Episode patterns
static SE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"[Ss](\d{1,2})[Ee](\d{1,3})(?:[Ee]\d{1,3})*").unwrap(),
        Regex::new(r"(\d{1,2})[xX](\d{2,3})").unwrap(),
        Regex::new(r"(?i)[Ss]eason\s*(\d{1,2})\s*[Ee]pisode\s*(\d{1,3})").unwrap(),
        Regex::new(r"[Cc](\d{1,2})[\s._\-]*[Ee][Pp](\d{1,3})").unwrap(),
        // Episode-only pattern (E01) — special handling: season defaults to 1
        Regex::new(r"(?:^|[\s._\-])[Ee][Pp]?(\d{1,3})(?:[\s._\-]|$)").unwrap(),
    ]
});

static YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|[\s._(\-])(\d{4})(?:[\s._)\-]|$)").unwrap());

static QUALITY_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"\b(2160p|4[Kk]|UHD)\b").unwrap(),
        Regex::new(r"\b(1080p|1080i)\b").unwrap(),
        Regex::new(r"\b(720p)\b").unwrap(),
        Regex::new(r"\b(480p|576p|SD)\b").unwrap(),
    ]
});

static SOURCE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\b(Blu-?[Rr]ay|BDRip|BRRip|BDREMUX)\b").unwrap(),
        Regex::new(r"(?i)\b(WEB-?DL|WEBRip|WEBDL|AMZN|NF|DSNP|HMAX|ATVP|PCOK|PMTP)\b").unwrap(),
        Regex::new(r"(?i)\b(DVDRip|DVDR|DVD9|DVD5)\b").unwrap(),
        Regex::new(r"(?i)\b(HDRip|HDTV|PDTV)\b").unwrap(),
        Regex::new(r"(?i)\b(CAM|TS|TC|HDCAM|SCR|SCREENER)\b").unwrap(),
        Regex::new(r"(?i)\b(REMUX)\b").unwrap(),
    ]
});

static CODEC_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"\b([Hh]\.?265|[Xx]\.?265|HEVC)\b").unwrap(),
        Regex::new(r"\b([Hh]\.?264|[Xx]\.?264|AVC)\b").unwrap(),
        Regex::new(r"(?i)\b(AV1)\b").unwrap(),
        Regex::new(r"(?i)\b(XviD|DivX)\b").unwrap(),
        Regex::new(r"(?i)\b(VP9)\b").unwrap(),
        Regex::new(r"(?i)\b(MPEG-?[24])\b").unwrap(),
    ]
});

static AUDIO_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\b(DTS-?HD[\s._\-]?MA|DTS-?HD|DTS-?X|DTS)\b").unwrap(),
        Regex::new(r"(?i)\b(TrueHD[\s._\-]?Atmos|TrueHD|Atmos)\b").unwrap(),
        Regex::new(r"(?i)\b(DD[P+]?\s*5\.1|DDP?7\.1|Dolby\s*Digital|AC-?3|EAC-?3|E-AC-3)\b")
            .unwrap(),
        Regex::new(r"(?i)\b(FLAC|LPCM|PCM)\b").unwrap(),
        Regex::new(r"(?i)\b(AAC[\s._\-]?2\.0|AAC[\s._\-]?5\.1|AAC)\b").unwrap(),
        Regex::new(r"(?i)\b(MP3|OGG|OPUS)\b").unwrap(),
    ]
});

static RELEASE_GROUP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"-([A-Za-z0-9]+)$").unwrap());

static MISC_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\b(PROPER|REPACK|RERIP|REAL|INTERNAL|LIMITED|EXTENDED|UNRATED|DC|DIRECTORS[\s._\-]?CUT)\b").unwrap(),
        Regex::new(r"(?i)\b(HDR10\+?|HDR|DV|DoVi|Dolby[\s._\-]?Vision|SDR|HLG)\b").unwrap(),
        Regex::new(r"(?i)\b(10bit|8bit|12bit)\b").unwrap(),
        Regex::new(r"(?i)\b(MULTI|MULTi|DUAL|DUBBED|SUBBED)\b").unwrap(),
        Regex::new(r"(?i)\b(COMPLETE|PROPER|REMASTERED)\b").unwrap(),
    ]
});

static BRACKET_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[[^\]]*\]").unwrap());
static PAREN_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\((?!\d{4}\))[^)]*\)").unwrap());
static SEPARATOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[._]").unwrap());
static DASH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[–—\-]").unwrap());
static BRACKET_CHARS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\[\](){}]").unwrap());
static MULTI_SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

// Folder parsing
static PAREN_YEAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((\d{4})\)").unwrap());
static TRAILING_YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\s._\-](\d{4})(?:[\s._\-]|$)").unwrap());
static ALL_PARENS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\([^)]*\)").unwrap());

fn strip_pattern(input: &str, pattern: &Regex) -> (String, Option<String>, Vec<String>) {
    if let Some(m) = pattern.find(input) {
        let cleaned = format!("{} {}", &input[..m.start()], &input[m.end()..]);
        let captures: Vec<String> = pattern
            .captures(input)
            .map(|c| {
                (1..c.len())
                    .filter_map(|i| c.get(i).map(|m| m.as_str().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        (cleaned, Some(m.as_str().to_string()), captures)
    } else {
        (input.to_string(), None, Vec::new())
    }
}

fn strip_pattern_list(input: &str, patterns: &[Regex]) -> (String, Option<String>, Vec<String>) {
    for pattern in patterns {
        let (cleaned, matched, groups) = strip_pattern(input, pattern);
        if matched.is_some() {
            return (cleaned, matched, groups);
        }
    }
    (input.to_string(), None, Vec::new())
}

pub fn parse_file_name(file_name: &str) -> ParsedFile {
    // Remove file extension
    let working = if let Some(idx) = file_name.rfind('.') {
        &file_name[..idx]
    } else {
        file_name
    };

    // Strip bracketed tags
    let mut working = BRACKET_TAG_RE.replace_all(working, " ").to_string();

    // Strip parenthesized tags (keep year patterns)
    working = PAREN_TAG_RE.replace_all(&working, " ").to_string();

    // Replace common separators with spaces
    working = SEPARATOR_RE.replace_all(&working, " ").to_string();

    // Strip release group
    let (w, _, _) = strip_pattern(&working, &RELEASE_GROUP_RE);
    working = w;

    // Extract season/episode
    let mut season: Option<i64> = None;
    let mut episode: Option<i64> = None;

    for (i, pattern) in SE_PATTERNS.iter().enumerate() {
        if let Some(caps) = pattern.captures(&working) {
            if i == 4 {
                // Episode-only pattern
                season = Some(1);
                episode = caps.get(1).and_then(|m| m.as_str().parse().ok());
            } else {
                season = caps.get(1).and_then(|m| m.as_str().parse().ok());
                episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }
            if let Some(m) = caps.get(0) {
                working = format!("{} {}", &working[..m.start()], &working[m.end()..]);
            }
            break;
        }
    }

    // Extract year
    let mut year: Option<i64> = None;
    let (w, _, groups) = strip_pattern(&working, &YEAR_RE);
    if let Some(y_str) = groups.first() {
        if let Ok(y) = y_str.parse::<i64>() {
            if (1900..=2028).contains(&y) {
                year = Some(y);
                working = w;
            }
        }
    }

    // Extract quality
    let (w, quality, _) = strip_pattern_list(&working, &QUALITY_PATTERNS);
    working = w;

    // Extract source
    let (w, source, _) = strip_pattern_list(&working, &SOURCE_PATTERNS);
    working = w;

    // Extract codec
    let (w, codec, _) = strip_pattern_list(&working, &CODEC_PATTERNS);
    working = w;

    // Extract audio
    let (w, audio, _) = strip_pattern_list(&working, &AUDIO_PATTERNS);
    working = w;

    // Strip misc tags
    for pattern in MISC_PATTERNS.iter() {
        working = pattern.replace_all(&working, " ").to_string();
    }

    // Clean up title
    let title = DASH_RE.replace_all(&working, " ").to_string();
    let title = BRACKET_CHARS_RE.replace_all(&title, " ").to_string();
    let title = MULTI_SPACE_RE.replace_all(&title, " ").to_string();
    let title = title.trim().to_string();

    // Media type heuristic
    let media_type = if season.is_some() || episode.is_some() {
        "tv"
    } else if year.is_some() {
        "movie"
    } else {
        "unknown"
    };

    ParsedFile {
        title,
        year,
        season,
        episode,
        quality,
        codec,
        source,
        audio,
        media_type: media_type.to_string(),
    }
}

pub fn parse_folder_name(folder_name: &str) -> ParsedFolder {
    let mut working = folder_name.to_string();

    // Strip bracketed tags
    working = BRACKET_TAG_RE.replace_all(&working, " ").to_string();

    // Extract year in parentheses
    let mut year: Option<i64> = None;
    if let Some(caps) = PAREN_YEAR_RE.captures(&working) {
        if let Ok(y) = caps[1].parse::<i64>() {
            if (1900..=2028).contains(&y) {
                year = Some(y);
                working = working.replace(caps.get(0).unwrap().as_str(), "");
            }
        }
    }

    // If no year in parens, try trailing year
    if year.is_none() {
        if let Some(caps) = TRAILING_YEAR_RE.captures(&working) {
            if let Ok(y) = caps[1].parse::<i64>() {
                if (1900..=2028).contains(&y) {
                    year = Some(y);
                    working = working[..caps.get(0).unwrap().start()].to_string();
                }
            }
        }
    }

    // Strip remaining parenthesized tags
    working = ALL_PARENS_RE.replace_all(&working, " ").to_string();

    // Clean up
    let title = SEPARATOR_RE.replace_all(&working, " ").to_string();
    let title = DASH_RE.replace_all(&title, " ").to_string();
    let title = BRACKET_CHARS_RE.replace_all(&title, " ").to_string();
    let title = MULTI_SPACE_RE.replace_all(&title, " ").to_string();
    let title = title.trim().to_string();

    ParsedFolder { title, year }
}
