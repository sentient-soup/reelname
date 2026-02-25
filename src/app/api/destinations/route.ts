import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { destinations } from "@/lib/db/schema";

export async function GET() {
  const all = db.select().from(destinations).all();
  return NextResponse.json(all);
}

export async function POST(request: Request) {
  const body = await request.json();
  const inserted = db.insert(destinations).values(body).returning().get();
  return NextResponse.json(inserted);
}
