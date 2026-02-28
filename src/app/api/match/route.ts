import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { settings } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { matchAllGroups } from "@/lib/matcher";

export async function POST() {
  try {
    const tmdbKey = db
      .select()
      .from(settings)
      .where(eq(settings.key, "tmdb_api_key"))
      .get();

    if (!tmdbKey?.value || tmdbKey.value.trim().length === 0) {
      return NextResponse.json(
        { error: "No TMDB API key configured. Set it in Settings." },
        { status: 400 }
      );
    }

    const result = await matchAllGroups();
    return NextResponse.json(result);
  } catch (error) {
    const message = error instanceof Error ? error.message : "Matching failed";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
