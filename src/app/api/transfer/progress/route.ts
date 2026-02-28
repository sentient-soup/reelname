import { db } from "@/lib/db";
import { jobs } from "@/lib/db/schema";
import { inArray } from "drizzle-orm";

export async function GET(request: Request) {
  const url = new URL(request.url);
  const idsParam = url.searchParams.get("ids");

  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    start(controller) {
      const send = () => {
        try {
          let transferJobs;
          if (idsParam) {
            const ids = idsParam.split(",").map(Number);
            transferJobs = db
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
              .where(inArray(jobs.id, ids))
              .all();
          } else {
            transferJobs = db
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
          }

          const data = JSON.stringify(transferJobs);
          controller.enqueue(encoder.encode(`data: ${data}\n\n`));

          // Check if all done (no queued or transferring jobs remain)
          const allDone = transferJobs.every(
            (j) =>
              j.status === "completed" ||
              j.status === "failed"
          );
          if (allDone && transferJobs.length > 0) {
            controller.enqueue(
              encoder.encode(`data: ${JSON.stringify({ done: true })}\n\n`)
            );
            controller.close();
            return;
          }
        } catch {
          controller.close();
          return;
        }

        setTimeout(send, 500);
      };

      send();
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
}
