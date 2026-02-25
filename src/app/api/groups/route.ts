import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { groups, jobs, settings } from "@/lib/db/schema";
import { eq, like, sql, desc, asc } from "drizzle-orm";
import { formatGroupedPath } from "@/lib/naming";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const status = searchParams.get("status");
  const mediaType = searchParams.get("mediaType");
  const search = searchParams.get("search");
  const sortBy = searchParams.get("sortBy") || "createdAt";
  const sortDir = searchParams.get("sortDir") || "desc";
  const page = parseInt(searchParams.get("page") || "1", 10);
  const limit = parseInt(searchParams.get("limit") || "50", 10);

  const conditions = [];

  if (status) {
    conditions.push(
      eq(groups.status, status as typeof groups.status.enumValues[number])
    );
  }
  if (mediaType) {
    conditions.push(
      eq(groups.mediaType, mediaType as typeof groups.mediaType.enumValues[number])
    );
  }
  if (search) {
    conditions.push(like(groups.folderName, `%${search}%`));
  }

  const where =
    conditions.length > 0
      ? sql`${sql.join(conditions, sql` AND `)}`
      : undefined;

  const sortColumn =
    sortBy === "folderName"
      ? groups.folderName
      : sortBy === "totalFileSize"
      ? groups.totalFileSize
      : sortBy === "totalFileCount"
      ? groups.totalFileCount
      : sortBy === "status"
      ? groups.status
      : sortBy === "mediaType"
      ? groups.mediaType
      : sortBy === "matchConfidence"
      ? groups.matchConfidence
      : groups.createdAt;

  const orderFn = sortDir === "asc" ? asc : desc;

  const results = db
    .select()
    .from(groups)
    .where(where)
    .orderBy(orderFn(sortColumn))
    .limit(limit)
    .offset((page - 1) * limit)
    .all();

  const countResult = db
    .select({ count: sql<number>`count(*)` })
    .from(groups)
    .where(where)
    .get();

  // Load naming settings for preview paths
  const allSettings = db.select().from(settings).all();
  const settingsMap: Record<string, string> = {};
  for (const s of allSettings) settingsMap[s.key] = s.value;
  const namingSettings = {
    naming_preset: settingsMap["naming_preset"] || "jellyfin",
    specials_folder_name: settingsMap["specials_folder_name"] || "Specials",
    extras_folder_name: settingsMap["extras_folder_name"] || "Extras",
  };

  // Fetch jobs for each group
  const groupsWithJobs = results.map((group) => {
    const groupJobs = db
      .select()
      .from(jobs)
      .where(eq(jobs.groupId, group.id))
      .all();

    // Compute preview names for groups with a TMDB match
    const jobsWithPreview = groupJobs.map((job) => ({
      ...job,
      previewName: group.tmdbId
        ? formatGroupedPath(job, group, namingSettings)
        : null,
    }));

    return {
      ...group,
      jobs: jobsWithPreview,
    };
  });

  return NextResponse.json({
    groups: groupsWithJobs,
    total: countResult?.count || 0,
    page,
    limit,
  });
}
