import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { jobs } from "@/lib/db/schema";
import { inArray } from "drizzle-orm";

export async function GET() {
  const transferJobs = db
    .select({
      id: jobs.id,
      status: jobs.status,
      fileName: jobs.fileName,
      fileSize: jobs.fileSize,
      transferProgress: jobs.transferProgress,
      transferError: jobs.transferError,
      destinationPath: jobs.destinationPath,
    })
    .from(jobs)
    .where(inArray(jobs.status, ["queued", "transferring", "completed", "failed"]))
    .all();

  const active = transferJobs.some(
    (j) => j.status === "queued" || j.status === "transferring"
  );

  return NextResponse.json({ active, jobs: transferJobs });
}
