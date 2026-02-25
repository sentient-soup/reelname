import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { jobs } from "@/lib/db/schema";
import { eq, like, sql, desc, asc } from "drizzle-orm";

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
    conditions.push(eq(jobs.status, status as typeof jobs.status.enumValues[number]));
  }
  if (mediaType) {
    conditions.push(eq(jobs.mediaType, mediaType as typeof jobs.mediaType.enumValues[number]));
  }
  if (search) {
    conditions.push(like(jobs.fileName, `%${search}%`));
  }

  const where = conditions.length > 0
    ? sql`${sql.join(conditions, sql` AND `)}`
    : undefined;

  const sortColumn = sortBy === "fileName" ? jobs.fileName
    : sortBy === "fileSize" ? jobs.fileSize
    : sortBy === "status" ? jobs.status
    : sortBy === "mediaType" ? jobs.mediaType
    : sortBy === "matchConfidence" ? jobs.matchConfidence
    : jobs.createdAt;

  const orderFn = sortDir === "asc" ? asc : desc;

  const results = db
    .select()
    .from(jobs)
    .where(where)
    .orderBy(orderFn(sortColumn))
    .limit(limit)
    .offset((page - 1) * limit)
    .all();

  const countResult = db
    .select({ count: sql<number>`count(*)` })
    .from(jobs)
    .where(where)
    .get();

  return NextResponse.json({
    jobs: results,
    total: countResult?.count || 0,
    page,
    limit,
  });
}
