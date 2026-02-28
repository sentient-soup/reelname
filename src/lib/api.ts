// Client-side API helpers — Tauri invoke() based
import { invoke } from "@tauri-apps/api/core";
import type { Job, MatchCandidate, Destination } from "@/lib/types";
import type { GroupWithJobs } from "@/lib/store";

// ── Groups ──────────────────────────────────────────────

export interface GroupsResponse {
  groups: GroupWithJobs[];
  total: number;
}

export async function fetchGroups(params?: Record<string, string>): Promise<GroupsResponse> {
  return invoke<GroupsResponse>("get_groups", {
    status: params?.status || null,
    mediaType: params?.mediaType || null,
    search: params?.search || null,
    sortBy: params?.sortBy || null,
    sortDir: params?.sortDir || null,
    page: params?.page ? parseInt(params.page, 10) : null,
    limit: params?.limit ? parseInt(params.limit, 10) : null,
  });
}

export async function fetchGroup(id: number): Promise<GroupWithJobs> {
  return invoke<GroupWithJobs>("get_group", { id });
}

export async function updateGroup(id: number, updates: Record<string, unknown>): Promise<void> {
  return invoke("update_group", { id, updates });
}

export async function deleteGroup(id: number): Promise<void> {
  return invoke("delete_group", { id });
}

// ── Jobs ────────────────────────────────────────────────

export interface JobsResponse {
  jobs: Job[];
  total: number;
}

export async function fetchJobs(params?: Record<string, string>): Promise<JobsResponse> {
  return invoke<JobsResponse>("get_jobs", {
    status: params?.status || null,
    mediaType: params?.mediaType || null,
    search: params?.search || null,
    sortBy: params?.sortBy || null,
    sortDir: params?.sortDir || null,
    page: params?.page ? parseInt(params.page, 10) : null,
    limit: params?.limit ? parseInt(params.limit, 10) : null,
  });
}

export async function fetchJob(id: number): Promise<Job> {
  return invoke<Job>("get_job", { id });
}

export async function updateJob(id: number, updates: Record<string, unknown>): Promise<void> {
  return invoke("update_job", { id, updates });
}

export async function deleteJob(id: number): Promise<void> {
  return invoke("delete_job", { id });
}

// ── Bulk actions ────────────────────────────────────────

export async function bulkAction(
  action: string,
  opts: { jobIds?: number[]; groupIds?: number[] }
): Promise<void> {
  return invoke("bulk_action", {
    action,
    jobIds: opts.jobIds || null,
    groupIds: opts.groupIds || null,
  });
}

// ── Scan ────────────────────────────────────────────────

export interface ScanResult {
  addedGroups?: number;
  addedFiles?: number;
  matched?: number;
  ambiguous?: number;
  error?: string;
  matchError?: string;
}

export async function triggerScan(path?: string): Promise<ScanResult> {
  return invoke<ScanResult>("scan_directory", { path: path || null });
}

// ── Match ───────────────────────────────────────────────

export interface MatchResult {
  matched?: number;
  ambiguous?: number;
  error?: string;
}

export async function triggerMatch(): Promise<MatchResult> {
  return invoke<MatchResult>("match_groups");
}

// ── Settings ────────────────────────────────────────────

export async function fetchSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_settings");
}

export async function updateSettings(updates: Record<string, string>): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("update_settings", { updates });
}

// ── TMDB Search ─────────────────────────────────────────

export interface TmdbSearchResponse {
  results?: MatchCandidate[];
}

export async function searchTmdb(query: string, mediaType?: string, year?: number): Promise<TmdbSearchResponse> {
  return invoke<TmdbSearchResponse>("search_tmdb", {
    query,
    mediaType: mediaType || null,
    year: year || null,
  });
}

// ── Seasons / Episodes ─────────────────────────────────

export interface Season {
  seasonNumber: number;
  name: string;
  episodeCount: number;
}

export interface Episode {
  episodeNumber: number;
  name: string;
  overview?: string;
}

export async function fetchSeasons(groupId: number): Promise<Season[]> {
  return invoke<Season[]>("get_seasons", { groupId });
}

export async function fetchSeasonEpisodes(groupId: number, season: number): Promise<Episode[]> {
  return invoke<Episode[]>("get_season_episodes", { groupId, season });
}

// ── Destinations ────────────────────────────────────────

export async function fetchDestinations(): Promise<Destination[]> {
  return invoke<Destination[]>("get_destinations");
}

export async function createDestination(data: Record<string, unknown>): Promise<Destination> {
  return invoke<Destination>("create_destination", { input: data });
}

export async function deleteDestination(id: number): Promise<void> {
  return invoke("delete_destination", { id });
}

export async function testSshConnection(data: {
  sshHost: string;
  sshPort: number;
  sshUser: string;
  sshKeyPath: string;
  sshKeyPassphrase?: string;
  basePath: string;
}): Promise<{ ok: boolean; error?: string }> {
  return invoke<{ ok: boolean; error?: string }>("test_ssh_connection", { input: data });
}

// ── Transfer ────────────────────────────────────────────

export async function startTransfer(
  opts: { jobIds?: number[]; groupIds?: number[] },
  destinationId: number
): Promise<void> {
  return invoke("start_transfer", {
    jobIds: opts.jobIds || null,
    groupIds: opts.groupIds || null,
    destinationId,
  });
}
