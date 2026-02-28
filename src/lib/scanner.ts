import fs from "fs";
import path from "path";

const VIDEO_EXTENSIONS = new Set([
  ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".m4v",
  ".mpg", ".mpeg", ".ts", ".m2ts", ".vob", ".iso", ".webm",
]);

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

export interface ScannedGroup {
  folderPath: string;
  folderName: string;
  files: ScannedGroupFile[];
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

export function scanDirectoryGrouped(dirPath: string): ScannedGroup[] {
  const groups: ScannedGroup[] = [];
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      const group: ScannedGroup = {
        folderPath: fullPath,
        folderName: entry.name,
        files: [],
      };

      // Walk the group folder
      const subEntries = fs.readdirSync(fullPath, { withFileTypes: true });
      let hasSeasonFolders = false;

      for (const sub of subEntries) {
        const subPath = path.join(fullPath, sub.name);

        if (sub.isDirectory()) {
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
          }
        }
      }

      if (group.files.length > 0) {
        // Media type heuristic: if season folders exist or multiple files → TV
        // Single file with no season folders → movie
        if (!hasSeasonFolders && group.files.length === 1 &&
            group.files.every((f) => f.fileCategory === "episode")) {
          // Single file, no season structure → likely a movie
          group.files[0].fileCategory = "movie";
        }

        groups.push(group);
      }
    } else if (entry.isFile()) {
      // Loose file in scan root → single-file group (movie)
      const ext = path.extname(entry.name).toLowerCase();
      if (VIDEO_EXTENSIONS.has(ext)) {
        const stat = fs.statSync(fullPath);
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
              fileCategory: "movie",
              extraType: null,
            },
          ],
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
