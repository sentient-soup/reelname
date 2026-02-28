import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { destinations } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const destId = parseInt(id, 10);
  const body = await request.json();

  const updated = db
    .update(destinations)
    .set(body)
    .where(eq(destinations.id, destId))
    .returning()
    .get();

  if (!updated) {
    return NextResponse.json({ error: "Destination not found" }, { status: 404 });
  }

  return NextResponse.json(updated);
}

export async function DELETE(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const destId = parseInt(id, 10);

  db.delete(destinations).where(eq(destinations.id, destId)).run();
  return NextResponse.json({ success: true });
}
