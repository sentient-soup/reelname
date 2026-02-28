import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { groups } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { getShowSeasons, getSeason } from "@/lib/tmdb";

export async function GET(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const groupId = parseInt(id, 10);

  const group = db.select().from(groups).where(eq(groups.id, groupId)).get();
  if (!group) {
    return NextResponse.json({ error: "Group not found" }, { status: 404 });
  }

  if (!group.tmdbId) {
    return NextResponse.json(
      { error: "Group has no TMDB match" },
      { status: 400 }
    );
  }

  const { searchParams } = new URL(request.url);
  const seasonParam = searchParams.get("season");

  if (seasonParam != null) {
    const seasonNumber = parseInt(seasonParam, 10);
    const season = await getSeason(group.tmdbId, seasonNumber);
    if (!season) {
      return NextResponse.json(
        { error: "Season not found" },
        { status: 404 }
      );
    }
    return NextResponse.json(season);
  }

  const seasons = await getShowSeasons(group.tmdbId);
  return NextResponse.json({ seasons });
}
