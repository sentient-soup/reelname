import { db } from "@/lib/db";
import { settings } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

const TMDB_BASE = "https://api.themoviedb.org/3";

// Simple rate limiter: max 40 requests per 10 seconds (TMDB limit)
let requestTimestamps: number[] = [];
const RATE_LIMIT = 35; // stay slightly under
const RATE_WINDOW = 10000;

async function rateLimitedFetch(url: string, init?: RequestInit): Promise<Response> {
  const now = Date.now();
  requestTimestamps = requestTimestamps.filter((t) => now - t < RATE_WINDOW);

  if (requestTimestamps.length >= RATE_LIMIT) {
    const waitTime = RATE_WINDOW - (now - requestTimestamps[0]);
    await new Promise((resolve) => setTimeout(resolve, waitTime));
  }

  requestTimestamps.push(Date.now());
  return fetch(url, init);
}

function getApiKey(): string {
  const setting = db
    .select()
    .from(settings)
    .where(eq(settings.key, "tmdb_api_key"))
    .get();
  return setting?.value || "";
}

export interface TmdbSearchResult {
  id: number;
  title?: string;
  name?: string;
  release_date?: string;
  first_air_date?: string;
  poster_path: string | null;
  overview: string;
  popularity: number;
  media_type?: string;
  vote_average: number;
}

export interface TmdbSearchResponse {
  results: TmdbSearchResult[];
  total_results: number;
}

export async function searchMulti(
  query: string,
  year?: number
): Promise<TmdbSearchResult[]> {
  const apiKey = getApiKey();
  if (!apiKey) throw new Error("TMDB API key not configured");

  const params = new URLSearchParams({
    api_key: apiKey,
    query,
    include_adult: "false",
  });
  if (year) params.set("year", String(year));

  const res = await rateLimitedFetch(`${TMDB_BASE}/search/multi?${params}`);
  if (!res.ok) throw new Error(`TMDB API error: ${res.status}`);

  const data: TmdbSearchResponse = await res.json();
  // Filter to only movies and TV shows
  return data.results.filter(
    (r) => r.media_type === "movie" || r.media_type === "tv"
  );
}

export async function searchMovies(
  query: string,
  year?: number
): Promise<TmdbSearchResult[]> {
  const apiKey = getApiKey();
  if (!apiKey) throw new Error("TMDB API key not configured");

  const params = new URLSearchParams({
    api_key: apiKey,
    query,
    include_adult: "false",
  });
  if (year) params.set("year", String(year));

  const res = await rateLimitedFetch(`${TMDB_BASE}/search/movie?${params}`);
  if (!res.ok) throw new Error(`TMDB API error: ${res.status}`);

  const data: TmdbSearchResponse = await res.json();
  return data.results.map((r) => ({ ...r, media_type: "movie" }));
}

export async function searchTV(
  query: string,
  year?: number
): Promise<TmdbSearchResult[]> {
  const apiKey = getApiKey();
  if (!apiKey) throw new Error("TMDB API key not configured");

  const params = new URLSearchParams({
    api_key: apiKey,
    query,
    include_adult: "false",
  });
  if (year) params.set("first_air_date_year", String(year));

  const res = await rateLimitedFetch(`${TMDB_BASE}/search/tv?${params}`);
  if (!res.ok) throw new Error(`TMDB API error: ${res.status}`);

  const data: TmdbSearchResponse = await res.json();
  return data.results.map((r) => ({ ...r, media_type: "tv" }));
}

export interface TmdbSeason {
  id: number;
  name: string;
  season_number: number;
  episode_count: number;
  air_date: string | null;
  overview: string;
  poster_path: string | null;
}

export interface TmdbSeasonDetail {
  id: number;
  name: string;
  season_number: number;
  episodes: TmdbEpisode[];
}

export async function getShowSeasons(tvId: number): Promise<TmdbSeason[]> {
  const apiKey = getApiKey();
  if (!apiKey) throw new Error("TMDB API key not configured");

  const res = await rateLimitedFetch(
    `${TMDB_BASE}/tv/${tvId}?api_key=${apiKey}`
  );
  if (!res.ok) throw new Error(`TMDB API error: ${res.status}`);

  const data = await res.json();
  return data.seasons || [];
}

export async function getSeason(
  tvId: number,
  seasonNumber: number
): Promise<TmdbSeasonDetail | null> {
  const apiKey = getApiKey();
  if (!apiKey) throw new Error("TMDB API key not configured");

  const res = await rateLimitedFetch(
    `${TMDB_BASE}/tv/${tvId}/season/${seasonNumber}?api_key=${apiKey}`
  );
  if (!res.ok) return null;

  return res.json();
}

export interface TmdbEpisode {
  id: number;
  name: string;
  episode_number: number;
  season_number: number;
  overview: string;
  still_path: string | null;
}

export async function getEpisode(
  tvId: number,
  season: number,
  episode: number
): Promise<TmdbEpisode | null> {
  const apiKey = getApiKey();
  if (!apiKey) return null;

  const res = await rateLimitedFetch(
    `${TMDB_BASE}/tv/${tvId}/season/${season}/episode/${episode}?api_key=${apiKey}`
  );
  if (!res.ok) return null;

  return res.json();
}
