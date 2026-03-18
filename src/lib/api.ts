// Client-side API helpers

// ── Groups ──────────────────────────────────────────────

export async function fetchGroups(params?: Record<string, string>) {
  const searchParams = new URLSearchParams();
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value) searchParams.set(key, value);
    }
  }
  const res = await fetch(`/api/groups?${searchParams}`);
  return res.json();
}

export async function fetchGroup(id: number) {
  const res = await fetch(`/api/groups/${id}`);
  return res.json();
}

export async function updateGroup(id: number, updates: Record<string, unknown>) {
  const res = await fetch(`/api/groups/${id}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteGroup(id: number) {
  const res = await fetch(`/api/groups/${id}`, { method: "DELETE" });
  return res.json();
}

// ── Jobs (kept for backward compat) ─────────────────────

export async function fetchJobs(params?: Record<string, string>) {
  const searchParams = new URLSearchParams();
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value) searchParams.set(key, value);
    }
  }
  const res = await fetch(`/api/jobs?${searchParams}`);
  return res.json();
}

export async function fetchJob(id: number) {
  const res = await fetch(`/api/jobs/${id}`);
  return res.json();
}

export async function updateJob(id: number, updates: Record<string, unknown>) {
  const res = await fetch(`/api/jobs/${id}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteJob(id: number) {
  const res = await fetch(`/api/jobs/${id}`, { method: "DELETE" });
  return res.json();
}

// ── Bulk actions ────────────────────────────────────────

export async function bulkAction(
  action: string,
  opts: { jobIds?: number[]; groupIds?: number[] }
) {
  const res = await fetch("/api/jobs/bulk", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action, ...opts }),
  });
  return res.json();
}

// ── Scan ────────────────────────────────────────────────

export interface ScanProgress {
  phase: "discovering" | "scanning" | "matching" | "complete" | "error";
  total?: number;
  processed?: number;
  added?: number;
  skipped?: number;
  name?: string;
  // complete event fields
  addedGroups?: number;
  addedFiles?: number;
  matched?: number;
  ambiguous?: number;
  matchError?: string | null;
  error?: string;
}

export async function triggerScan(
  path?: string,
  onProgress?: (progress: ScanProgress) => void
) {
  const res = await fetch("/api/scan", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(path ? { path } : {}),
  });

  // If the response is not SSE (e.g. an error JSON), handle it the old way
  const contentType = res.headers.get("Content-Type") || "";
  if (!contentType.includes("text/event-stream")) {
    return res.json();
  }

  const reader = res.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let result: any = {};

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() || "";

    let currentEvent = "";
    for (const line of lines) {
      if (line.startsWith("event: ")) {
        currentEvent = line.slice(7).trim();
      } else if (line.startsWith("data: ")) {
        const data = JSON.parse(line.slice(6));

        if (currentEvent === "phase") {
          onProgress?.({ phase: data.phase });
        } else if (currentEvent === "discovered") {
          onProgress?.({ phase: "scanning", total: data.total, processed: 0 });
        } else if (currentEvent === "progress") {
          onProgress?.({
            phase: "scanning",
            total: data.total,
            processed: data.processed,
            added: data.added,
            skipped: data.skipped,
            name: data.name,
          });
        } else if (currentEvent === "matchProgress") {
          onProgress?.({
            phase: "matching",
            processed: data.processed,
            total: data.total,
          });
        } else if (currentEvent === "complete") {
          result = data;
          onProgress?.({ phase: "complete", ...data });
        } else if (currentEvent === "error") {
          result = data;
          onProgress?.({ phase: "error", error: data.error });
        }
      }
    }
  }

  return result;
}

// ── Match ───────────────────────────────────────────────

export async function triggerMatch() {
  const res = await fetch("/api/match", { method: "POST" });
  return res.json();
}

// ── Settings ────────────────────────────────────────────

export async function fetchSettings() {
  const res = await fetch("/api/settings");
  return res.json();
}

export async function updateSettings(updates: Record<string, string>) {
  const res = await fetch("/api/settings", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(updates),
  });
  return res.json();
}

// ── TMDB Search ─────────────────────────────────────────

export async function searchTmdb(query: string, mediaType?: string, year?: number) {
  const params = new URLSearchParams({ query });
  if (mediaType) params.set("mediaType", mediaType);
  if (year) params.set("year", String(year));
  const res = await fetch(`/api/search?${params}`);
  return res.json();
}

// ── Seasons / Episodes ─────────────────────────────────

export async function fetchSeasons(groupId: number) {
  const res = await fetch(`/api/groups/${groupId}/seasons`);
  return res.json();
}

export async function fetchSeasonEpisodes(groupId: number, season: number) {
  const res = await fetch(`/api/groups/${groupId}/seasons?season=${season}`);
  return res.json();
}

// ── Destinations ────────────────────────────────────────

export async function fetchDestinations() {
  const res = await fetch("/api/destinations");
  return res.json();
}

export async function createDestination(data: Record<string, unknown>) {
  const res = await fetch("/api/destinations", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  return res.json();
}

export async function deleteDestination(id: number) {
  const res = await fetch(`/api/destinations/${id}`, { method: "DELETE" });
  return res.json();
}

export async function testSshConnection(data: {
  sshHost: string;
  sshPort: number;
  sshUser: string;
  sshKeyPath: string;
  sshKeyPassphrase?: string;
  basePath: string;
}): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch("/api/destinations/test-connection", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  return res.json();
}

// ── Transfer ────────────────────────────────────────────

export async function startTransfer(
  opts: { jobIds?: number[]; groupIds?: number[] },
  destinationId: number
) {
  const res = await fetch("/api/transfer", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ...opts, destinationId }),
  });
  return res.json();
}

export async function fetchTransferStatus(): Promise<{
  active: boolean;
  jobs: Array<{
    id: number;
    status: string;
    fileName: string;
    fileSize: number;
    transferProgress: number | null;
    transferError: string | null;
    destinationPath: string | null;
  }>;
}> {
  const res = await fetch("/api/transfer/status");
  return res.json();
}
