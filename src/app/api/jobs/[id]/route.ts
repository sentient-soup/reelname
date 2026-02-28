import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { jobs, matchCandidates } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const jobId = parseInt(id, 10);

  const job = db.select().from(jobs).where(eq(jobs.id, jobId)).get();
  if (!job) {
    return NextResponse.json({ error: "Job not found" }, { status: 404 });
  }

  const candidates = db
    .select()
    .from(matchCandidates)
    .where(eq(matchCandidates.jobId, jobId))
    .all();

  return NextResponse.json({ ...job, candidates });
}

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const jobId = parseInt(id, 10);
  const body = await request.json();

  const job = db.select().from(jobs).where(eq(jobs.id, jobId)).get();
  if (!job) {
    return NextResponse.json({ error: "Job not found" }, { status: 404 });
  }

  const allowedFields = [
    "status", "mediaType", "parsedTitle", "parsedYear", "parsedSeason",
    "parsedEpisode", "parsedQuality", "parsedCodec", "tmdbId", "tmdbTitle",
    "tmdbYear", "tmdbPosterPath", "tmdbEpisodeTitle", "matchConfidence",
    "destinationId", "destinationPath", "transferProgress", "transferError",
    "fileCategory",
  ];

  // Use camelCase keys directly — Drizzle .set() expects JS property names, not SQL column names
  const updates: Record<string, unknown> = { updatedAt: new Date().toISOString() };
  for (const field of allowedFields) {
    if (field in body) {
      updates[field] = body[field];
    }
  }

  const updated = db
    .update(jobs)
    .set(updates)
    .where(eq(jobs.id, jobId))
    .returning()
    .get();

  return NextResponse.json(updated);
}

export async function DELETE(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const jobId = parseInt(id, 10);

  db.delete(matchCandidates).where(eq(matchCandidates.jobId, jobId)).run();
  db.delete(jobs).where(eq(jobs.id, jobId)).run();

  return NextResponse.json({ success: true });
}
