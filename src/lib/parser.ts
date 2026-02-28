export interface ParsedFile {
  title: string;
  year?: number;
  season?: number;
  episode?: number;
  quality?: string;
  codec?: string;
  source?: string;
  audio?: string;
  mediaType: "movie" | "tv" | "unknown";
}

// Patterns to extract, applied in order. Each removes matched tokens from the string.
const SEASON_EPISODE_PATTERNS = [
  // S01E01, S01E01E02
  /[Ss](\d{1,2})[Ee](\d{1,3})(?:[Ee]\d{1,3})*/,
  // 1x01, 01x01
  /(\d{1,2})[xX](\d{2,3})/,
  // Season 1 Episode 1
  /[Ss]eason\s*(\d{1,2})\s*[Ee]pisode\s*(\d{1,3})/i,
  // c1_ep3, c01_ep03 (common rip naming)
  /[Cc](\d{1,2})[\s._-]*[Ee][Pp](\d{1,3})/,
  // E01, Ep01 (no season - assume S01)
  /(?:^|[\s._-])[Ee][Pp]?(\d{1,3})(?:[\s._-]|$)/,
];

const YEAR_PATTERN = /(?:^|[\s._(-])(\d{4})(?:[\s._)-]|$)/;

const QUALITY_PATTERNS = [
  /\b(2160p|4[Kk]|UHD)\b/,
  /\b(1080p|1080i)\b/,
  /\b(720p)\b/,
  /\b(480p|576p|SD)\b/,
];

const SOURCE_PATTERNS = [
  /\b(Blu-?[Rr]ay|BDRip|BRRip|BDREMUX)\b/i,
  /\b(WEB-?DL|WEBRip|WEBDL|AMZN|NF|DSNP|HMAX|ATVP|PCOK|PMTP)\b/i,
  /\b(DVDRip|DVDR|DVD9|DVD5)\b/i,
  /\b(HDRip|HDTV|PDTV)\b/i,
  /\b(CAM|TS|TC|HDCAM|SCR|SCREENER)\b/i,
  /\b(REMUX)\b/i,
];

const CODEC_PATTERNS = [
  /\b([Hh]\.?265|[Xx]\.?265|HEVC)\b/,
  /\b([Hh]\.?264|[Xx]\.?264|AVC)\b/,
  /\b(AV1)\b/i,
  /\b(XviD|DivX)\b/i,
  /\b(VP9)\b/i,
  /\b(MPEG-?[24])\b/i,
];

const AUDIO_PATTERNS = [
  /\b(DTS-?HD[\s._-]?MA|DTS-?HD|DTS-?X|DTS)\b/i,
  /\b(TrueHD[\s._-]?Atmos|TrueHD|Atmos)\b/i,
  /\b(DD[P+]?\s*5\.1|DDP?7\.1|Dolby\s*Digital|AC-?3|EAC-?3|E-AC-3)\b/i,
  /\b(FLAC|LPCM|PCM)\b/i,
  /\b(AAC[\s._-]?2\.0|AAC[\s._-]?5\.1|AAC)\b/i,
  /\b(MP3|OGG|OPUS)\b/i,
];

const RELEASE_GROUP_PATTERN = /-([A-Za-z0-9]+)$/;

const MISC_PATTERNS = [
  /\b(PROPER|REPACK|RERIP|REAL|INTERNAL|LIMITED|EXTENDED|UNRATED|DC|DIRECTORS[\s._-]?CUT)\b/i,
  /\b(HDR10\+?|HDR|DV|DoVi|Dolby[\s._-]?Vision|SDR|HLG)\b/i,
  /\b(10bit|8bit|12bit)\b/i,
  /\b(MULTI|MULTi|DUAL|DUBBED|SUBBED)\b/i,
  /\b(COMPLETE|PROPER|REMASTERED)\b/i,
];

function stripPattern(
  input: string,
  pattern: RegExp
): { cleaned: string; match: string | null; groups?: string[] } {
  const m = input.match(pattern);
  if (!m) return { cleaned: input, match: null };
  return {
    cleaned: input.slice(0, m.index) + " " + input.slice(m.index! + m[0].length),
    match: m[0],
    groups: m.slice(1),
  };
}

function stripPatternList(
  input: string,
  patterns: RegExp[]
): { cleaned: string; match: string | null; groups?: string[] } {
  for (const pattern of patterns) {
    const result = stripPattern(input, pattern);
    if (result.match) return result;
  }
  return { cleaned: input, match: null };
}

export function parseFileName(fileName: string): ParsedFile {
  // Remove file extension
  let working = fileName.replace(/\.[^.]+$/, "");

  // Strip bracketed tags like [DTA], [SubGroup], [1080p], [HEVC] etc.
  // These are common in anime/scene releases and should not pollute the title.
  working = working.replace(/\[[^\]]*\]/g, " ");

  // Strip parenthesized tags like (Batch), (BD), (Dual Audio) but keep year patterns (2020)
  working = working.replace(/\((?!\d{4}\))[^)]*\)/g, " ");

  // Replace common separators with spaces
  working = working.replace(/[._]/g, " ");

  // Strip release group (typically last token after a dash)
  const releaseResult = stripPattern(working, RELEASE_GROUP_PATTERN);
  working = releaseResult.cleaned;

  // Extract season/episode
  let season: number | undefined;
  let episode: number | undefined;

  for (const pattern of SEASON_EPISODE_PATTERNS) {
    const m = working.match(pattern);
    if (m) {
      if (pattern === SEASON_EPISODE_PATTERNS[4]) {
        // Episode-only pattern (E01)
        season = 1;
        episode = parseInt(m[1], 10);
      } else {
        season = parseInt(m[1], 10);
        episode = parseInt(m[2], 10);
      }
      working = working.slice(0, m.index) + " " + working.slice(m.index! + m[0].length);
      break;
    }
  }

  // Extract year
  let year: number | undefined;
  const yearResult = stripPattern(working, YEAR_PATTERN);
  if (yearResult.groups) {
    const y = parseInt(yearResult.groups[0], 10);
    if (y >= 1900 && y <= new Date().getFullYear() + 1) {
      year = y;
      working = yearResult.cleaned;
    }
  }

  // Extract quality
  const qualityResult = stripPatternList(working, QUALITY_PATTERNS);
  const quality = qualityResult.match || undefined;
  working = qualityResult.cleaned;

  // Extract source
  const sourceResult = stripPatternList(working, SOURCE_PATTERNS);
  working = sourceResult.cleaned;

  // Extract codec
  const codecResult = stripPatternList(working, CODEC_PATTERNS);
  const codec = codecResult.match || undefined;
  working = codecResult.cleaned;

  // Extract audio
  const audioResult = stripPatternList(working, AUDIO_PATTERNS);
  const audio = audioResult.match || undefined;
  working = audioResult.cleaned;

  // Strip misc tags
  for (const pattern of MISC_PATTERNS) {
    working = working.replace(pattern, " ");
  }

  // Clean up remaining string to get title
  let title = working
    .replace(/[-–—]/g, " ")
    .replace(/[[\](){}]/g, " ")
    .replace(/\s+/g, " ")
    .trim();

  // Media type heuristic
  let mediaType: "movie" | "tv" | "unknown" = "unknown";
  if (season !== undefined || episode !== undefined) {
    mediaType = "tv";
  } else if (year !== undefined) {
    mediaType = "movie";
  }

  return {
    title,
    year,
    season,
    episode,
    quality,
    codec,
    source: sourceResult.match || undefined,
    audio,
    mediaType,
  };
}

export interface ParsedFolder {
  title: string;
  year?: number;
}

/**
 * Parse a folder name to extract title and optional year.
 * Simpler than filename parsing — just handles year in parens and common separators.
 */
export function parseFolderName(folderName: string): ParsedFolder {
  let working = folderName;

  // Strip bracketed tags like [DTA], [SubGroup], [1080p] etc.
  working = working.replace(/\[[^\]]*\]/g, " ");

  // Extract year in parentheses: "Show Name (2020)" or "Show Name (2020) [extras]"
  let year: number | undefined;
  const parenYearMatch = working.match(/\((\d{4})\)/);
  if (parenYearMatch) {
    const y = parseInt(parenYearMatch[1], 10);
    if (y >= 1900 && y <= new Date().getFullYear() + 1) {
      year = y;
      working = working.replace(parenYearMatch[0], "");
    }
  }

  // If no year in parens, try trailing year: "Show.Name.2020" or "Show Name 2020"
  if (!year) {
    const trailingYearMatch = working.match(/[\s._-](\d{4})(?:[\s._-]|$)/);
    if (trailingYearMatch) {
      const y = parseInt(trailingYearMatch[1], 10);
      if (y >= 1900 && y <= new Date().getFullYear() + 1) {
        year = y;
        working = working.slice(0, trailingYearMatch.index);
      }
    }
  }

  // Strip remaining parenthesized tags like (Batch), (BD), (Complete) — year already extracted
  working = working.replace(/\([^)]*\)/g, " ");

  // Replace dots and underscores with spaces
  let title = working
    .replace(/[._]/g, " ")
    .replace(/[-–—]/g, " ")
    .replace(/[[\](){}]/g, " ")
    .replace(/\s+/g, " ")
    .trim();

  return { title, year };
}
