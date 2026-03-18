import { db } from "@/lib/db";
import { groups, jobs, subtitleFiles, settings } from "@/lib/db/schema";
import { eq, isNull } from "drizzle-orm";
import { scanDirectoryGrouped } from "@/lib/scanner";
import { parseFolderName, parseFileName } from "@/lib/parser";
import { matchAllGroups } from "@/lib/matcher";

export async function POST(request: Request) {
  const body = await request.json().catch(() => ({}));
  let scanPath = body.path as string | undefined;

  if (!scanPath) {
    const setting = db
      .select()
      .from(settings)
      .where(eq(settings.key, "scan_path"))
      .get();
    scanPath = setting?.value;
  }

  if (!scanPath) {
    return Response.json(
      { error: "No scan path configured. Set it in settings." },
      { status: 400 }
    );
  }

  const encoder = new TextEncoder();
  const path = scanPath;

  const stream = new ReadableStream({
    async start(controller) {
      function send(event: string, data: Record<string, unknown>) {
        controller.enqueue(
          encoder.encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
        );
      }

      try {
        // Clean up orphaned jobs
        db.delete(jobs).where(isNull(jobs.groupId)).run();

        // Read subtitle settings
        const subExtSetting = db
          .select()
          .from(settings)
          .where(eq(settings.key, "subtitle_extensions"))
          .get();
        const subtitleExts = new Set(
          (subExtSetting?.value || "srt")
            .split(",")
            .map((e) => {
              const trimmed = e.trim().toLowerCase();
              return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
            })
            .filter(Boolean)
        );

        const subLangSetting = db
          .select()
          .from(settings)
          .where(eq(settings.key, "subtitle_languages"))
          .get();
        const subtitleLangs = subLangSetting?.value
          ? new Set(
              subLangSetting.value
                .split(",")
                .map((l) => l.trim().toLowerCase())
                .filter(Boolean)
            )
          : null;

        send("phase", { phase: "discovering" });

        const scannedGroups = scanDirectoryGrouped(path, subtitleExts, subtitleLangs);

        // Get existing group folder paths to avoid duplicates
        const existingGroups = db
          .select({ folderPath: groups.folderPath })
          .from(groups)
          .all();
        const existingPaths = new Set(existingGroups.map((g) => g.folderPath));

        const totalGroups = scannedGroups.length;
        send("discovered", { total: totalGroups });

        let addedGroups = 0;
        let addedFiles = 0;
        let skippedGroups = 0;
        const now = new Date().toISOString();

        for (let i = 0; i < scannedGroups.length; i++) {
          const scannedGroup = scannedGroups[i];

          if (existingPaths.has(scannedGroup.folderPath)) {
            skippedGroups++;
            send("progress", {
              processed: i + 1,
              total: totalGroups,
              added: addedGroups,
              skipped: skippedGroups,
              name: scannedGroup.folderName,
            });
            continue;
          }

          const parsedFolder = parseFolderName(scannedGroup.folderName);

          const hasEpisodes = scannedGroup.files.some(
            (f) => f.fileCategory === "episode" || f.fileCategory === "special"
          );
          const allMovies = scannedGroup.files.every(
            (f) => f.fileCategory === "movie"
          );
          const mediaType = allMovies ? "movie" : hasEpisodes ? "tv" : "unknown";

          const totalSize = scannedGroup.files.reduce((sum, f) => sum + f.fileSize, 0);

          const insertedGroup = db
            .insert(groups)
            .values({
              status: "scanned",
              mediaType,
              folderPath: scannedGroup.folderPath,
              folderName: scannedGroup.folderName,
              totalFileCount: scannedGroup.files.length,
              totalFileSize: totalSize,
              parsedTitle: parsedFolder.title,
              parsedYear: parsedFolder.year,
              createdAt: now,
              updatedAt: now,
            })
            .returning()
            .get();

          addedGroups++;

          for (const file of scannedGroup.files) {
            const parsed = parseFileName(file.fileName);
            const season = file.detectedSeason ?? parsed.season;
            const episode = parsed.episode;

            const existingJob = db
              .select()
              .from(jobs)
              .where(eq(jobs.sourcePath, file.sourcePath))
              .get();

            if (existingJob) {
              db.update(jobs)
                .set({
                  groupId: insertedGroup.id,
                  status: "scanned",
                  mediaType,
                  fileCategory: file.fileCategory as "episode" | "movie" | "special" | "extra",
                  extraType: file.extraType as typeof jobs.extraType.enumValues[number] | null | undefined,
                  parsedTitle: parsed.title,
                  parsedYear: parsed.year,
                  parsedSeason: season,
                  parsedEpisode: episode,
                  parsedQuality: parsed.quality,
                  parsedCodec: parsed.codec,
                  tmdbId: null,
                  tmdbTitle: null,
                  tmdbYear: null,
                  tmdbPosterPath: null,
                  tmdbEpisodeTitle: null,
                  matchConfidence: null,
                  updatedAt: now,
                })
                .where(eq(jobs.id, existingJob.id))
                .run();
            } else {
              db.insert(jobs)
                .values({
                  groupId: insertedGroup.id,
                  status: "scanned",
                  mediaType,
                  fileCategory: file.fileCategory as "episode" | "movie" | "special" | "extra",
                  extraType: file.extraType as typeof jobs.extraType.enumValues[number] | null | undefined,
                  sourcePath: file.sourcePath,
                  fileName: file.fileName,
                  fileSize: file.fileSize,
                  fileExtension: file.fileExtension,
                  parsedTitle: parsed.title,
                  parsedYear: parsed.year,
                  parsedSeason: season,
                  parsedEpisode: episode,
                  parsedQuality: parsed.quality,
                  parsedCodec: parsed.codec,
                  createdAt: now,
                  updatedAt: now,
                })
                .run();
            }

            addedFiles++;
          }

          // Match subtitle files to their parent media jobs
          if (scannedGroup.subtitles.length > 0) {
            const groupJobs = db
              .select()
              .from(jobs)
              .where(eq(jobs.groupId, insertedGroup.id))
              .all();

            const videoBaseToJob = new Map<string, number>();
            for (const j of groupJobs) {
              const baseName = j.fileName.replace(/\.[^.]+$/, "").toLowerCase();
              videoBaseToJob.set(baseName, j.id);
            }

            for (const sub of scannedGroup.subtitles) {
              const jobId = videoBaseToJob.get(sub.matchBase);
              if (jobId != null) {
                db.insert(subtitleFiles)
                  .values({
                    jobId,
                    sourcePath: sub.sourcePath,
                    fileName: sub.fileName,
                    fileExtension: sub.fileExtension,
                    languageCode: sub.languageCode,
                  })
                  .run();
              }
            }
          }

          send("progress", {
            processed: i + 1,
            total: totalGroups,
            added: addedGroups,
            skipped: skippedGroups,
            name: scannedGroup.folderName,
          });
        }

        // Auto-match if TMDB key is configured
        let matchResult = { matched: 0, ambiguous: 0 };
        let matchError: string | null = null;
        const tmdbKey = db
          .select()
          .from(settings)
          .where(eq(settings.key, "tmdb_api_key"))
          .get();
        if (tmdbKey?.value && tmdbKey.value.trim().length > 0) {
          send("phase", { phase: "matching" });
          try {
            matchResult = await matchAllGroups((processed, total) => {
              send("matchProgress", { processed, total });
            });
          } catch (err) {
            matchError = err instanceof Error ? err.message : "Matching failed";
            console.error("Matching failed:", err);
          }
        } else {
          matchError = "No TMDB API key configured. Set it in Settings to enable auto-matching.";
        }

        send("complete", {
          scannedGroups: totalGroups,
          addedGroups,
          addedFiles,
          skippedGroups,
          matched: matchResult.matched,
          ambiguous: matchResult.ambiguous,
          matchError,
        });

        controller.close();
      } catch (error) {
        const message = error instanceof Error ? error.message : "Scan failed";
        send("error", { error: message });
        controller.close();
      }
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
