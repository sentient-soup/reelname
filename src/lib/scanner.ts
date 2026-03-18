import fs from "fs";
import path from "path";

const VIDEO_EXTENSIONS = new Set([
  ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".m4v",
  ".mpg", ".mpeg", ".ts", ".m2ts", ".vob", ".iso", ".webm",
]);

const DEFAULT_SUBTITLE_EXTENSIONS = new Set([".srt"]);

const SUBTITLE_FOLDER_NAMES = new Set([
  "subs", "sub", "subtitles", "subtitle",
]);

/** Quick check for season/episode indicators in a filename */
const HAS_EPISODE_INFO = /[Ss]\d{1,2}[Ee]\d{1,3}|\d{1,2}[xX]\d{2,3}|[Ss]eason\s*\d{1,2}\s*[Ee]pisode/i;

const SEASON_FOLDER_PATTERN = /^(?:Season\s*|S)(\d+)$/i;
const SPECIALS_FOLDER_NAMES = new Set(["specials", "season 0", "season 00", "season0", "season00"]);

const EXTRA_FOLDER_MAP: Record<string, string> = {
  "extras": "other",
  "extra": "other",
  "behind the scenes": "behind_the_scenes",
  "behindthescenes": "behind_the_scenes",
  "deleted scenes": "deleted_scenes",
  "deletedscenes": "deleted_scenes",
  "featurettes": "featurettes",
  "featurette": "featurettes",
  "interviews": "interviews",
  "interview": "interviews",
  "scenes": "scenes",
  "scene": "scenes",
  "shorts": "shorts",
  "short": "shorts",
  "trailers": "trailers",
  "trailer": "trailers",
  "other": "other",
};

export interface ScannedFile {
  sourcePath: string;
  fileName: string;
  fileSize: number;
  fileExtension: string;
}

export type FileCategory = "episode" | "movie" | "special" | "extra";

export interface ScannedGroupFile {
  sourcePath: string;
  fileName: string;
  fileSize: number;
  fileExtension: string;
  detectedSeason: number | null;
  fileCategory: FileCategory;
  extraType: string | null;
}

export interface ScannedSubtitleFile {
  sourcePath: string;
  fileName: string;
  fileExtension: string;
  languageCode: string | null;
  /** The base name used for matching to a video file (without language code and subtitle extension) */
  matchBase: string;
}

export interface ScannedGroup {
  folderPath: string;
  folderName: string;
  files: ScannedGroupFile[];
  subtitles: ScannedSubtitleFile[];
}

/**
 * Parse a subtitle filename to extract the language code and match base.
 * E.g. "Show.S01E01.en.srt" → { languageCode: "en", matchBase: "show.s01e01" }
 *      "Show.S01E01.srt"    → { languageCode: null, matchBase: "show.s01e01" }
 */
function parseSubtitleName(
  fileName: string,
  subtitleExts: Set<string>
): { languageCode: string | null; matchBase: string } | null {
  const ext = path.extname(fileName).toLowerCase();
  if (!subtitleExts.has(ext)) return null;

  const withoutExt = fileName.slice(0, -ext.length);
  // Check if the part before .srt is a 2-3 char language code
  const lastDot = withoutExt.lastIndexOf(".");
  if (lastDot !== -1) {
    const possibleLang = withoutExt.slice(lastDot + 1);
    if (/^[a-z]{2,3}$/i.test(possibleLang)) {
      return {
        languageCode: possibleLang.toLowerCase(),
        matchBase: withoutExt.slice(0, lastDot).toLowerCase(),
      };
    }
  }
  return { languageCode: null, matchBase: withoutExt.toLowerCase() };
}

function collectSubtitleFiles(
  dir: string,
  subtitleExts: Set<string>,
  languageFilter: Set<string> | null
): ScannedSubtitleFile[] {
  const results: ScannedSubtitleFile[] = [];
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return results;
  }
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectSubtitleFiles(fullPath, subtitleExts, languageFilter));
    } else if (entry.isFile()) {
      const ext = path.extname(entry.name).toLowerCase();
      if (subtitleExts.has(ext)) {
        const parsed = parseSubtitleName(entry.name, subtitleExts);
        if (parsed) {
          // Language filter: if filter is set and subtitle has a language code, it must match
          if (languageFilter && parsed.languageCode && !languageFilter.has(parsed.languageCode)) {
            continue;
          }
          results.push({
            sourcePath: fullPath,
            fileName: entry.name,
            fileExtension: ext,
            languageCode: parsed.languageCode,
            matchBase: parsed.matchBase,
          });
        }
      }
    }
  }
  return results;
}

function collectVideoFiles(dir: string): ScannedFile[] {
  const results: ScannedFile[] = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectVideoFiles(fullPath));
    } else if (entry.isFile()) {
      const ext = path.extname(entry.name).toLowerCase();
      if (VIDEO_EXTENSIONS.has(ext)) {
        const stat = fs.statSync(fullPath);
        results.push({
          sourcePath: fullPath,
          fileName: entry.name,
          fileSize: stat.size,
          fileExtension: ext,
        });
      }
    }
  }
  return results;
}

function classifySubfolder(
  folderName: string
): {
  detectedSeason: number | null;
  fileCategory: FileCategory;
  extraType: string | null;
} {
  const lower = folderName.toLowerCase().trim();

  // Check specials
  if (SPECIALS_FOLDER_NAMES.has(lower)) {
    return { detectedSeason: 0, fileCategory: "special", extraType: null };
  }

  // Check season pattern
  const seasonMatch = folderName.match(SEASON_FOLDER_PATTERN);
  if (seasonMatch) {
    const seasonNum = parseInt(seasonMatch[1], 10);
    if (seasonNum === 0) {
      return { detectedSeason: 0, fileCategory: "special", extraType: null };
    }
    return { detectedSeason: seasonNum, fileCategory: "episode", extraType: null };
  }

  // Check extras
  const extraType = EXTRA_FOLDER_MAP[lower];
  if (extraType) {
    return { detectedSeason: null, fileCategory: "extra", extraType };
  }

  return { detectedSeason: null, fileCategory: "episode", extraType: null };
}

export function scanDirectoryGrouped(
  dirPath: string,
  subtitleExtensions?: Set<string>,
  subtitleLanguages?: Set<string> | null
): ScannedGroup[] {
  const subExts = subtitleExtensions ?? DEFAULT_SUBTITLE_EXTENSIONS;
  const langFilter = subtitleLanguages && subtitleLanguages.size > 0 ? subtitleLanguages : null;
  const groups: ScannedGroup[] = [];
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      const group: ScannedGroup = {
        folderPath: fullPath,
        folderName: entry.name,
        files: [],
        subtitles: [],
      };

      // Walk the group folder
      const subEntries = fs.readdirSync(fullPath, { withFileTypes: true });
      let hasSeasonFolders = false;

      for (const sub of subEntries) {
        const subPath = path.join(fullPath, sub.name);

        if (sub.isDirectory()) {
          // Check if this is a subtitle folder
          if (SUBTITLE_FOLDER_NAMES.has(sub.name.toLowerCase().trim())) {
            group.subtitles.push(...collectSubtitleFiles(subPath, subExts, langFilter));
            continue;
          }

          const classification = classifySubfolder(sub.name);
          if (classification.fileCategory === "episode" && classification.detectedSeason !== null) {
            hasSeasonFolders = true;
          }

          // Collect all video files in this subfolder
          const files = collectVideoFiles(subPath);
          for (const file of files) {
            group.files.push({
              ...file,
              detectedSeason: classification.detectedSeason,
              fileCategory: classification.fileCategory,
              extraType: classification.extraType,
            });
          }

          // Also collect subtitles from season/extra subfolders and their sub dirs
          group.subtitles.push(...collectSubtitleFiles(subPath, subExts, langFilter));
        } else if (sub.isFile()) {
          const ext = path.extname(sub.name).toLowerCase();
          if (VIDEO_EXTENSIONS.has(ext)) {
            const stat = fs.statSync(subPath);
            group.files.push({
              sourcePath: subPath,
              fileName: sub.name,
              fileSize: stat.size,
              fileExtension: ext,
              detectedSeason: null, // will be inferred from filename later
              fileCategory: "episode", // default, may be reclassified
              extraType: null,
            });
          } else if (subExts.has(ext)) {
            // Subtitle file at group root level
            const parsed = parseSubtitleName(sub.name, subExts);
            if (parsed) {
              // Language filter: if filter is set and subtitle has a language code, it must match
              if (!langFilter || !parsed.languageCode || langFilter.has(parsed.languageCode)) {
                group.subtitles.push({
                  sourcePath: subPath,
                  fileName: sub.name,
                  fileExtension: ext,
                  languageCode: parsed.languageCode,
                  matchBase: parsed.matchBase,
                });
              }
            }
          }
        }
      }

      if (group.files.length > 0) {
        // Media type heuristic: if season folders exist or multiple files → TV
        // Single file with no season folders and no episode info in filename → movie
        if (!hasSeasonFolders && group.files.length === 1 &&
            group.files.every((f) => f.fileCategory === "episode") &&
            !HAS_EPISODE_INFO.test(group.files[0].fileName)) {
          group.files[0].fileCategory = "movie";
        }

        groups.push(group);
      }
    } else if (entry.isFile()) {
      // Loose file in scan root → single-file group
      const ext = path.extname(entry.name).toLowerCase();
      if (VIDEO_EXTENSIONS.has(ext)) {
        const stat = fs.statSync(fullPath);
        // Detect if the filename contains season/episode info → TV, otherwise movie
        const hasEpisodeInfo = HAS_EPISODE_INFO.test(entry.name);
        groups.push({
          folderPath: dirPath,
          folderName: entry.name.replace(/\.[^.]+$/, ""),
          files: [
            {
              sourcePath: fullPath,
              fileName: entry.name,
              fileSize: stat.size,
              fileExtension: ext,
              detectedSeason: null,
              fileCategory: hasEpisodeInfo ? "episode" : "movie",
              extraType: null,
            },
          ],
          subtitles: [],
        });
      }
    }
  }

  return groups;
}

// Keep legacy flat scanner for backward compatibility
export function scanDirectory(dirPath: string): ScannedFile[] {
  return collectVideoFiles(dirPath);
}
