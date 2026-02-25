import {
  searchMulti,
  searchMovies,
  searchTV,
  getEpisode,
  type TmdbSearchResult,
} from "./tmdb";
import { db } from "./db";
import { groups, jobs, matchCandidates, settings } from "./db/schema";
import { eq } from "drizzle-orm";
import type { Group, NewMatchCandidate } from "./db/schema";

/**
 * Levenshtein distance normalized to 0-1 similarity
 */
function titleSimilarity(a: string, b: string): number {
  const s1 = a.toLowerCase().trim();
  const s2 = b.toLowerCase().trim();

  if (s1 === s2) return 1;
  if (s1.length === 0 || s2.length === 0) return 0;

  const len1 = s1.length;
  const len2 = s2.length;
  const matrix: number[][] = [];

  for (let i = 0; i <= len1; i++) {
    matrix[i] = [i];
  }
  for (let j = 0; j <= len2; j++) {
    matrix[0][j] = j;
  }

  for (let i = 1; i <= len1; i++) {
    for (let j = 1; j <= len2; j++) {
      const cost = s1[i - 1] === s2[j - 1] ? 0 : 1;
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1,
        matrix[i][j - 1] + 1,
        matrix[i - 1][j - 1] + cost
      );
    }
  }

  const maxLen = Math.max(len1, len2);
  return 1 - matrix[len1][len2] / maxLen;
}

/**
 * Calculate confidence score for a match
 */
function calculateConfidence(
  parsedTitle: string,
  parsedYear: number | null,
  parsedMediaType: string,
  result: TmdbSearchResult
): number {
  const tmdbTitle = result.title || result.name || "";
  const tmdbYear = parseInt(
    (result.release_date || result.first_air_date || "").slice(0, 4),
    10
  );
  const tmdbMediaType = result.media_type || "unknown";

  // Title similarity: 60% weight
  const titleScore = titleSimilarity(parsedTitle, tmdbTitle) * 0.6;

  // Year match: 25% weight
  let yearScore = 0;
  if (parsedYear && !isNaN(tmdbYear)) {
    const yearDiff = Math.abs(parsedYear - tmdbYear);
    if (yearDiff === 0) yearScore = 0.25;
    else if (yearDiff === 1) yearScore = 0.15;
    else if (yearDiff === 2) yearScore = 0.05;
  } else if (!parsedYear) {
    yearScore = 0.1; // neutral when we don't have a year
  }

  // Media type consistency: 10% weight
  let typeScore = 0.05;
  if (parsedMediaType !== "unknown") {
    if (
      (parsedMediaType === "tv" && tmdbMediaType === "tv") ||
      (parsedMediaType === "movie" && tmdbMediaType === "movie")
    ) {
      typeScore = 0.1;
    } else {
      typeScore = 0;
    }
  }

  // Popularity tiebreaker: 5% weight
  const popScore = Math.min(result.popularity / 100, 1) * 0.05;

  return titleScore + yearScore + typeScore + popScore;
}

/**
 * Match a group against TMDB using folder name
 */
export async function matchGroup(group: Group): Promise<void> {
  if (!group.parsedTitle) {
    db.update(groups)
      .set({ status: "ambiguous", updatedAt: new Date().toISOString() })
      .where(eq(groups.id, group.id))
      .run();
    return;
  }

  // Search TMDB based on media type
  let results: TmdbSearchResult[];
  if (group.mediaType === "tv") {
    results = await searchTV(group.parsedTitle, group.parsedYear ?? undefined);
  } else if (group.mediaType === "movie") {
    results = await searchMovies(group.parsedTitle, group.parsedYear ?? undefined);
  } else {
    results = await searchMulti(group.parsedTitle, group.parsedYear ?? undefined);
  }

  if (results.length === 0) {
    db.update(groups)
      .set({ status: "ambiguous", updatedAt: new Date().toISOString() })
      .where(eq(groups.id, group.id))
      .run();
    return;
  }

  // Score all results
  const scored = results.slice(0, 10).map((r) => ({
    result: r,
    confidence: calculateConfidence(
      group.parsedTitle!,
      group.parsedYear,
      group.mediaType,
      r
    ),
  }));

  scored.sort((a, b) => b.confidence - a.confidence);

  // Save candidates at group level
  db.delete(matchCandidates).where(eq(matchCandidates.groupId, group.id)).run();

  for (const { result, confidence } of scored) {
    const tmdbTitle = result.title || result.name || "";
    const tmdbYear = parseInt(
      (result.release_date || result.first_air_date || "").slice(0, 4),
      10
    );

    const candidate: NewMatchCandidate = {
      groupId: group.id,
      jobId: null,
      tmdbId: result.id,
      mediaType: (result.media_type as "movie" | "tv") || "movie",
      title: tmdbTitle,
      year: isNaN(tmdbYear) ? null : tmdbYear,
      posterPath: result.poster_path,
      overview: result.overview?.slice(0, 500) || null,
      confidence,
    };

    db.insert(matchCandidates).values(candidate).run();
  }

  // Auto-match logic
  const threshold = parseFloat(
    db.select().from(settings).where(eq(settings.key, "auto_match_threshold")).get()
      ?.value || "0.85"
  );
  const top = scored[0];
  const second = scored[1];
  const gap = second ? top.confidence - second.confidence : 1;

  const now = new Date().toISOString();

  if (top.confidence >= threshold && gap >= 0.15) {
    const tmdbTitle = top.result.title || top.result.name || "";
    const tmdbYear = parseInt(
      (top.result.release_date || top.result.first_air_date || "").slice(0, 4),
      10
    );

    // Update group with match
    db.update(groups)
      .set({
        status: "matched",
        tmdbId: top.result.id,
        tmdbTitle,
        tmdbYear: isNaN(tmdbYear) ? null : tmdbYear,
        tmdbPosterPath: top.result.poster_path,
        matchConfidence: top.confidence,
        mediaType: (top.result.media_type as "movie" | "tv") || group.mediaType,
        updatedAt: now,
      })
      .where(eq(groups.id, group.id))
      .run();

    // Update child jobs with status and TMDB info
    db.update(jobs)
      .set({
        status: "matched",
        tmdbId: top.result.id,
        tmdbTitle: tmdbTitle,
        tmdbYear: isNaN(tmdbYear) ? null : tmdbYear,
        tmdbPosterPath: top.result.poster_path,
        matchConfidence: top.confidence,
        updatedAt: now,
      })
      .where(eq(jobs.groupId, group.id))
      .run();

    // For TV groups, fetch episode titles
    if (top.result.media_type === "tv") {
      await fetchEpisodeTitles(group.id, top.result.id);
    }
  } else {
    db.update(groups)
      .set({
        status: "ambiguous",
        matchConfidence: top.confidence,
        updatedAt: now,
      })
      .where(eq(groups.id, group.id))
      .run();
  }
}

/**
 * Fetch episode titles from TMDB for all jobs in a TV group
 */
async function fetchEpisodeTitles(
  groupId: number,
  tmdbId: number
): Promise<void> {
  const groupJobs = db
    .select()
    .from(jobs)
    .where(eq(jobs.groupId, groupId))
    .all();

  for (const job of groupJobs) {
    if (
      job.fileCategory === "extra" ||
      job.parsedSeason == null ||
      job.parsedEpisode == null
    ) {
      continue;
    }

    try {
      const ep = await getEpisode(tmdbId, job.parsedSeason, job.parsedEpisode);
      if (ep) {
        db.update(jobs)
          .set({
            tmdbEpisodeTitle: ep.name,
            updatedAt: new Date().toISOString(),
          })
          .where(eq(jobs.id, job.id))
          .run();
      }
    } catch {
      // Episode not found, skip
    }
  }
}

/**
 * Match all unmatched groups
 */
export async function matchAllGroups(): Promise<{
  matched: number;
  ambiguous: number;
}> {
  const unmatched = db
    .select()
    .from(groups)
    .where(eq(groups.status, "scanned"))
    .all();

  let matched = 0;
  let ambiguous = 0;

  for (const group of unmatched) {
    try {
      await matchGroup(group);
      const updated = db
        .select()
        .from(groups)
        .where(eq(groups.id, group.id))
        .get();
      if (updated?.status === "matched") matched++;
      else ambiguous++;
    } catch (err) {
      console.error(`Failed to match group ${group.id}:`, err);
      ambiguous++;
    }
  }

  return { matched, ambiguous };
}
