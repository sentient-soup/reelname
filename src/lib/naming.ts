import type { Job, Group } from "./db/schema";

export type NamingPreset = "jellyfin" | "plex";

/**
 * Naming presets based on official documentation:
 *
 * Jellyfin: https://jellyfin.org/docs/general/server/media/shows/
 *   Series folder:  "Series Name (year)/Season XX/Series Name SxxExx - Episode Title.ext"
 *   Movie folder:   "Movie Name (year)/Movie Name (year).ext"
 *   Specials:       "Series Name (year)/Season 00/Series Name S00Exx - Episode Title.ext"
 *   Extras:         "Series Name (year)/behind the scenes/filename.ext" (lowercase folder names)
 *
 * Plex: https://support.plex.tv/articles/naming-and-organizing-your-tv-show-files/
 *   Series folder:  "Series Name (year)/Season XX/Series Name (year) - sXXeXX - Episode Title.ext"
 *   Movie folder:   "Movie Name (year)/Movie Name (year).ext"
 *   Specials:       "Series Name (year)/Specials/Series Name (year) - s00eXX - Episode Title.ext"
 *   Extras:         "Series Name (year)/Behind The Scenes/filename.ext" (title case folder names)
 */
export const NAMING_PRESETS: Record<
  NamingPreset,
  {
    movie: string;
    tv: string;
    special: string;
    extra: string;
  }
> = {
  jellyfin: {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} S{season:2}E{episode:2} - {episodeTitle}.{ext}",
    special:
      "{title} ({year})/Season 00/{title} S00E{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
  },
  plex: {
    movie: "{title} ({year})/{title} ({year}).{ext}",
    tv: "{title} ({year})/Season {season:2}/{title} ({year}) - s{season:2}e{episode:2} - {episodeTitle}.{ext}",
    special:
      "{title} ({year})/Specials/{title} ({year}) - s00e{episode:2} - {episodeTitle}.{ext}",
    extra: "{title} ({year})/{extraType}/{fileName}.{ext}",
  },
};

// Map extra_type DB values to Jellyfin folder names (lowercase per Jellyfin docs)
const JELLYFIN_EXTRA_FOLDER_NAMES: Record<string, string> = {
  behind_the_scenes: "behind the scenes",
  deleted_scenes: "deleted scenes",
  featurettes: "featurettes",
  interviews: "interviews",
  scenes: "clips",
  shorts: "shorts",
  trailers: "trailers",
  other: "extras",
};

// Map extra_type DB values to Plex folder names (title case per Plex convention)
const PLEX_EXTRA_FOLDER_NAMES: Record<string, string> = {
  behind_the_scenes: "Behind The Scenes",
  deleted_scenes: "Deleted Scenes",
  featurettes: "Featurettes",
  interviews: "Interviews",
  scenes: "Scenes",
  shorts: "Shorts",
  trailers: "Trailers",
  other: "Other",
};

function sanitize(str: string): string {
  return str
    .replace(/[<>:"/\\|?*]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function padNum(val: number | null | undefined, width: number): string {
  if (val == null) return "00".padStart(width, "0");
  return String(val).padStart(width, "0");
}

interface NamingSettings {
  naming_preset: string;
  specials_folder_name: string;
  extras_folder_name: string;
}

/**
 * Format a destination path using group context and naming presets.
 */
export function formatGroupedPath(
  job: Job,
  group: Group,
  namingSettings: NamingSettings
): string {
  const preset = (namingSettings.naming_preset || "jellyfin") as NamingPreset;
  const presetTemplates = NAMING_PRESETS[preset] || NAMING_PRESETS.jellyfin;

  // Select template by file category
  let template: string;
  switch (job.fileCategory) {
    case "movie":
      template = presetTemplates.movie;
      break;
    case "special":
      template = presetTemplates.special;
      break;
    case "extra":
      template = presetTemplates.extra;
      break;
    default:
      template = presetTemplates.tv;
  }

  // Use group-level TMDB info for title/year, fallback to job-level
  const title = sanitize(group.tmdbTitle || group.parsedTitle || "Unknown");
  const year = group.tmdbYear || group.parsedYear || job.tmdbYear || job.parsedYear || "";
  const ext = job.fileExtension.replace(/^\./, "");
  const episodeTitle = sanitize(job.tmdbEpisodeTitle || "");
  const quality = job.parsedQuality || "";
  const fileName = sanitize(job.fileName.replace(/\.[^.]+$/, ""));

  // Select extra type folder name based on preset
  const extraFolderMap =
    preset === "plex" ? PLEX_EXTRA_FOLDER_NAMES : JELLYFIN_EXTRA_FOLDER_NAMES;
  const extraTypeName =
    extraFolderMap[job.extraType || ""] ||
    (preset === "plex" ? "Other" : "extras");

  let result = template;

  result = result.replace(/\{title\}/g, title);
  result = result.replace(/\{year\}/g, String(year));
  result = result.replace(/\{ext\}/g, ext);
  result = result.replace(/\{episodeTitle\}/g, episodeTitle || "Episode");
  result = result.replace(/\{quality\}/g, quality);
  result = result.replace(/\{fileName\}/g, fileName);
  result = result.replace(/\{extraType\}/g, extraTypeName);

  // Season/episode with padding
  result = result.replace(/\{season:(\d+)\}/g, (_, width) =>
    padNum(job.parsedSeason, parseInt(width, 10))
  );
  result = result.replace(/\{episode:(\d+)\}/g, (_, width) =>
    padNum(job.parsedEpisode, parseInt(width, 10))
  );

  result = result.replace(/\{season\}/g, String(job.parsedSeason ?? 0));
  result = result.replace(/\{episode\}/g, String(job.parsedEpisode ?? 0));

  // Clean up empty episode titles leaving trailing " - "
  result = result.replace(/ - \./, ".");
  result = result.replace(/ - Episode\./, ".");

  // Clean up empty year leaving "()" in path
  result = result.replace(/ \(\)/g, "");

  return result;
}

/**
 * Legacy: format path for jobs without a group (backward compat)
 */
export function formatPath(template: string, job: Job): string {
  const title = sanitize(job.tmdbTitle || job.parsedTitle || "Unknown");
  const year = job.tmdbYear || job.parsedYear || "";
  const ext = job.fileExtension.replace(/^\./, "");
  const episodeTitle = sanitize(job.tmdbEpisodeTitle || "");
  const quality = job.parsedQuality || "";

  let result = template;
  result = result.replace(/\{title\}/g, title);
  result = result.replace(/\{year\}/g, String(year));
  result = result.replace(/\{ext\}/g, ext);
  result = result.replace(/\{episodeTitle\}/g, episodeTitle);
  result = result.replace(/\{quality\}/g, quality);
  result = result.replace(/\{season:(\d+)\}/g, (_, width) =>
    padNum(job.parsedSeason, parseInt(width, 10))
  );
  result = result.replace(/\{episode:(\d+)\}/g, (_, width) =>
    padNum(job.parsedEpisode, parseInt(width, 10))
  );
  result = result.replace(/\{season\}/g, String(job.parsedSeason ?? 0));
  result = result.replace(/\{episode\}/g, String(job.parsedEpisode ?? 0));
  result = result.replace(/ \(\)/g, "");

  return result;
}
