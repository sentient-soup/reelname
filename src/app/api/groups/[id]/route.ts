import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { groups, jobs, matchCandidates, settings } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { getEpisode } from "@/lib/tmdb";
import { formatGroupedPath } from "@/lib/naming";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const groupId = parseInt(id, 10);

  const group = db.select().from(groups).where(eq(groups.id, groupId)).get();
  if (!group) {
    return NextResponse.json({ error: "Group not found" }, { status: 404 });
  }

  const groupJobs = db
    .select()
    .from(jobs)
    .where(eq(jobs.groupId, groupId))
    .all();

  const candidates = db
    .select()
    .from(matchCandidates)
    .where(eq(matchCandidates.groupId, groupId))
    .all();

  // Compute preview names if group has a TMDB match
  let jobsWithPreview = groupJobs;
  if (group.tmdbId) {
    const allSettings = db.select().from(settings).all();
    const settingsMap: Record<string, string> = {};
    for (const s of allSettings) settingsMap[s.key] = s.value;
    const namingSettings = {
      naming_preset: settingsMap["naming_preset"] || "jellyfin",
      specials_folder_name: settingsMap["specials_folder_name"] || "Specials",
      extras_folder_name: settingsMap["extras_folder_name"] || "Extras",
    };
    jobsWithPreview = groupJobs.map((job) => ({
      ...job,
      previewName: formatGroupedPath(job, group, namingSettings),
    }));
  }

  return NextResponse.json({ ...group, jobs: jobsWithPreview, candidates });
}

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const groupId = parseInt(id, 10);
  const body = await request.json();

  const group = db.select().from(groups).where(eq(groups.id, groupId)).get();
  if (!group) {
    return NextResponse.json({ error: "Group not found" }, { status: 404 });
  }

  const now = new Date().toISOString();

  const allowedFields = [
    "status", "mediaType", "parsedTitle", "parsedYear",
    "tmdbId", "tmdbTitle", "tmdbYear", "tmdbPosterPath",
    "matchConfidence", "destinationId",
  ];

  // Use camelCase keys directly — Drizzle .set() expects JS property names, not SQL column names
  const updates: Record<string, unknown> = { updatedAt: now };
  for (const field of allowedFields) {
    if (field in body) {
      updates[field] = body[field];
    }
  }

  const updated = db
    .update(groups)
    .set(updates)
    .where(eq(groups.id, groupId))
    .returning()
    .get();

  // Cascade status changes to child jobs
  if ("status" in body) {
    db.update(jobs)
      .set({ status: body.status, updatedAt: now })
      .where(eq(jobs.groupId, groupId))
      .run();
  }

  // Cascade TMDB info to child jobs
  if ("tmdbId" in body) {
    const tmdbUpdates: Record<string, unknown> = { updatedAt: now };
    if ("tmdbId" in body) tmdbUpdates.tmdbId = body.tmdbId;
    if ("tmdbTitle" in body) tmdbUpdates.tmdbTitle = body.tmdbTitle;
    if ("tmdbYear" in body) tmdbUpdates.tmdbYear = body.tmdbYear;
    if ("tmdbPosterPath" in body) tmdbUpdates.tmdbPosterPath = body.tmdbPosterPath;
    if ("matchConfidence" in body) tmdbUpdates.matchConfidence = body.matchConfidence;

    db.update(jobs)
      .set(tmdbUpdates)
      .where(eq(jobs.groupId, groupId))
      .run();

    // Fetch episode titles for TV groups when a TMDB match is confirmed
    const resolvedMediaType = body.mediaType || updated.mediaType;
    if (resolvedMediaType === "tv" && body.tmdbId) {
      await fetchEpisodeTitlesForGroup(groupId, body.tmdbId);
    }
  }

  const groupJobs = db
    .select()
    .from(jobs)
    .where(eq(jobs.groupId, groupId))
    .all();

  // Compute preview names if group has a TMDB match
  let jobsWithPreview = groupJobs;
  if (updated.tmdbId) {
    const allSettings = db.select().from(settings).all();
    const settingsMap: Record<string, string> = {};
    for (const s of allSettings) settingsMap[s.key] = s.value;
    const namingSettings = {
      naming_preset: settingsMap["naming_preset"] || "jellyfin",
      specials_folder_name: settingsMap["specials_folder_name"] || "Specials",
      extras_folder_name: settingsMap["extras_folder_name"] || "Extras",
    };
    jobsWithPreview = groupJobs.map((job) => ({
      ...job,
      previewName: formatGroupedPath(job, updated, namingSettings),
    }));
  }

  return NextResponse.json({ ...updated, jobs: jobsWithPreview });
}

async function fetchEpisodeTitlesForGroup(
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

export async function DELETE(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const groupId = parseInt(id, 10);

  // Cascade delete handles jobs and matchCandidates
  db.delete(groups).where(eq(groups.id, groupId)).run();

  return NextResponse.json({ success: true });
}
