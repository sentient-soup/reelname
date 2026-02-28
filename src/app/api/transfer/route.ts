import { NextResponse } from "next/server";
import { queueTransfers } from "@/lib/transfer";
import { db } from "@/lib/db";
import { jobs } from "@/lib/db/schema";
import { eq, inArray } from "drizzle-orm";

export async function POST(request: Request) {
  try {
    const body = await request.json();
    const { jobIds, groupIds, destinationId } = body as {
      jobIds?: number[];
      groupIds?: number[];
      destinationId: number;
    };

    if ((!jobIds?.length && !groupIds?.length) || !destinationId) {
      return NextResponse.json(
        { error: "jobIds or groupIds and destinationId are required" },
        { status: 400 }
      );
    }

    // Collect all job IDs to transfer
    const allJobIds = new Set<number>(jobIds || []);

    // Expand groupIds to their confirmed child jobs
    if (groupIds?.length) {
      for (const gid of groupIds) {
        const groupJobs = db
          .select()
          .from(jobs)
          .where(eq(jobs.groupId, gid))
          .all();
        for (const j of groupJobs) {
          if (j.status === "confirmed") {
            allJobIds.add(j.id);
          }
        }
      }
    }

    if (allJobIds.size === 0) {
      return NextResponse.json(
        { error: "No confirmed jobs to transfer" },
        { status: 400 }
      );
    }

    const jobIdArray = [...allJobIds];

    // Reset any old completed/failed jobs so they don't pollute progress totals
    db.update(jobs)
      .set({
        status: "confirmed",
        transferProgress: null,
        transferError: null,
        destinationPath: null,
        updatedAt: new Date().toISOString(),
      })
      .where(inArray(jobs.status, ["completed", "failed"]))
      .run();

    // Mark all new jobs as queued so they're immediately visible to SSE/status
    db.update(jobs)
      .set({
        status: "queued",
        transferProgress: null,
        transferError: null,
        updatedAt: new Date().toISOString(),
      })
      .where(inArray(jobs.id, jobIdArray))
      .run();

    const result = queueTransfers(jobIdArray, destinationId);
    return NextResponse.json(result);
  } catch (error) {
    const message = error instanceof Error ? error.message : "Transfer failed";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
