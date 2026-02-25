import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { jobs, groups, matchCandidates } from "@/lib/db/schema";
import { eq, inArray } from "drizzle-orm";

export async function POST(request: Request) {
  const body = await request.json();
  const { action, jobIds, groupIds } = body as {
    action: "confirm" | "skip" | "delete" | "rematch";
    jobIds?: number[];
    groupIds?: number[];
  };

  if (!action || (!jobIds?.length && !groupIds?.length)) {
    return NextResponse.json(
      { error: "action and jobIds or groupIds are required" },
      { status: 400 }
    );
  }

  const now = new Date().toISOString();
  let affected = 0;

  // Handle group-level actions
  if (groupIds?.length) {
    switch (action) {
      case "confirm":
        db.update(groups)
          .set({ status: "confirmed", updatedAt: now })
          .where(inArray(groups.id, groupIds))
          .run();
        for (const gid of groupIds) {
          db.update(jobs)
            .set({ status: "confirmed", updatedAt: now })
            .where(eq(jobs.groupId, gid))
            .run();
        }
        break;

      case "skip":
        db.update(groups)
          .set({ status: "skipped", updatedAt: now })
          .where(inArray(groups.id, groupIds))
          .run();
        for (const gid of groupIds) {
          db.update(jobs)
            .set({ status: "skipped", updatedAt: now })
            .where(eq(jobs.groupId, gid))
            .run();
        }
        break;

      case "delete":
        // Cascade delete handles child jobs and candidates
        db.delete(groups).where(inArray(groups.id, groupIds)).run();
        break;

      case "rematch":
        db.update(groups)
          .set({
            status: "scanned",
            tmdbId: null,
            tmdbTitle: null,
            tmdbYear: null,
            tmdbPosterPath: null,
            matchConfidence: null,
            updatedAt: now,
          })
          .where(inArray(groups.id, groupIds))
          .run();
        for (const gid of groupIds) {
          db.update(jobs)
            .set({
              status: "scanned",
              tmdbEpisodeTitle: null,
              updatedAt: now,
            })
            .where(eq(jobs.groupId, gid))
            .run();
          db.delete(matchCandidates)
            .where(eq(matchCandidates.groupId, gid))
            .run();
        }
        break;
    }
    affected += groupIds.length;
  }

  // Handle job-level actions (unchanged)
  if (jobIds?.length) {
    switch (action) {
      case "confirm":
        db.update(jobs)
          .set({ status: "confirmed", updatedAt: now })
          .where(inArray(jobs.id, jobIds))
          .run();
        break;

      case "skip":
        db.update(jobs)
          .set({ status: "skipped", updatedAt: now })
          .where(inArray(jobs.id, jobIds))
          .run();
        break;

      case "delete":
        for (const id of jobIds) {
          db.delete(matchCandidates).where(eq(matchCandidates.jobId, id)).run();
        }
        db.delete(jobs).where(inArray(jobs.id, jobIds)).run();
        break;

      case "rematch":
        db.update(jobs)
          .set({
            status: "scanned",
            tmdbId: null,
            tmdbTitle: null,
            tmdbYear: null,
            tmdbPosterPath: null,
            tmdbEpisodeTitle: null,
            matchConfidence: null,
            updatedAt: now,
          })
          .where(inArray(jobs.id, jobIds))
          .run();
        for (const id of jobIds) {
          db.delete(matchCandidates).where(eq(matchCandidates.jobId, id)).run();
        }
        break;
    }
    affected += jobIds.length;
  }

  return NextResponse.json({ success: true, affected });
}
