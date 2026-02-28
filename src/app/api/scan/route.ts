import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { groups, jobs, settings } from "@/lib/db/schema";
import { eq, isNull } from "drizzle-orm";
import { scanDirectoryGrouped } from "@/lib/scanner";
import { parseFolderName, parseFileName } from "@/lib/parser";
import { matchAllGroups } from "@/lib/matcher";

export async function POST(request: Request) {
  try {
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
      return NextResponse.json(
        { error: "No scan path configured. Set it in settings." },
        { status: 400 }
      );
    }

    // Clean up orphaned jobs (from before grouping refactor) that have no group
    db.delete(jobs).where(isNull(jobs.groupId)).run();

    const scannedGroups = scanDirectoryGrouped(scanPath);

    // Get existing group folder paths to avoid duplicates
    const existingGroups = db
      .select({ folderPath: groups.folderPath })
      .from(groups)
      .all();
    const existingPaths = new Set(existingGroups.map((g) => g.folderPath));

    let addedGroups = 0;
    let addedFiles = 0;
    let skippedGroups = 0;
    const now = new Date().toISOString();

    for (const scannedGroup of scannedGroups) {
      if (existingPaths.has(scannedGroup.folderPath)) {
        skippedGroups++;
        continue;
      }

      const parsedFolder = parseFolderName(scannedGroup.folderName);

      // Determine media type: if any file is episode/special → TV, if all movie → movie
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

      // Insert or re-link child jobs
      for (const file of scannedGroup.files) {
        const parsed = parseFileName(file.fileName);
        const season = file.detectedSeason ?? parsed.season;
        const episode = parsed.episode;

        // Check if a job already exists for this source path (from a previous scan)
        const existingJob = db
          .select()
          .from(jobs)
          .where(eq(jobs.sourcePath, file.sourcePath))
          .get();

        if (existingJob) {
          // Re-link the orphaned job to this group and update its parsed data
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
              // Clear old TMDB data so it gets re-fetched at group level
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
      try {
        matchResult = await matchAllGroups();
      } catch (err) {
        matchError = err instanceof Error ? err.message : "Matching failed";
        console.error("Matching failed:", err);
      }
    } else {
      matchError = "No TMDB API key configured. Set it in Settings to enable auto-matching.";
    }

    return NextResponse.json({
      scannedGroups: scannedGroups.length,
      addedGroups,
      addedFiles,
      skippedGroups,
      matched: matchResult.matched,
      ambiguous: matchResult.ambiguous,
      matchError,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Scan failed";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
