import { NextRequest, NextResponse } from "next/server";
import { searchMulti, searchMovies, searchTV } from "@/lib/tmdb";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const query = searchParams.get("query");
  const mediaType = searchParams.get("mediaType");
  const year = searchParams.get("year");

  if (!query) {
    return NextResponse.json({ error: "query is required" }, { status: 400 });
  }

  try {
    const yearNum = year ? parseInt(year, 10) : undefined;
    let results;

    if (mediaType === "movie") {
      results = await searchMovies(query, yearNum);
    } else if (mediaType === "tv") {
      results = await searchTV(query, yearNum);
    } else {
      results = await searchMulti(query, yearNum);
    }

    // Normalize results for frontend
    const normalized = results.slice(0, 10).map((r) => ({
      tmdbId: r.id,
      mediaType: r.media_type || "movie",
      title: r.title || r.name || "",
      year: parseInt(
        (r.release_date || r.first_air_date || "").slice(0, 4),
        10
      ) || null,
      posterPath: r.poster_path,
      overview: r.overview?.slice(0, 500) || null,
      confidence: 1.0,
    }));

    return NextResponse.json({ results: normalized });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Search failed";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
