import { NextResponse } from "next/server";
import { db, DATA_DIR } from "@/lib/db";
import { settings } from "@/lib/db/schema";
import fs from "fs";
import path from "path";
import os from "os";

function getConfigFilePath(): string {
  const platform = process.platform;
  let dir: string;
  if (platform === "win32") {
    dir = process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming");
  } else if (platform === "darwin") {
    dir = path.join(os.homedir(), "Library", "Application Support");
  } else {
    dir = process.env.XDG_CONFIG_HOME || path.join(os.homedir(), ".config");
  }
  return path.join(dir, "ReelName", "reelname-config.json");
}

function readConfig(): Record<string, string> {
  try {
    const raw = fs.readFileSync(getConfigFilePath(), "utf-8");
    return JSON.parse(raw);
  } catch {
    return {};
  }
}

function writeConfig(config: Record<string, string>) {
  const configPath = getConfigFilePath();
  fs.mkdirSync(path.dirname(configPath), { recursive: true });
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
}

export async function GET() {
  const allSettings = db.select().from(settings).all();
  const result: Record<string, string> = {};
  for (const s of allSettings) {
    result[s.key] = s.value;
  }
  // Include current data directory info
  result.data_dir = DATA_DIR;
  // Include configured (pending) data dir if different
  const config = readConfig();
  if (config.data_dir) {
    result.configured_data_dir = config.data_dir;
  }
  return NextResponse.json(result);
}

export async function PUT(request: Request) {
  const body = await request.json();

  // Handle data_dir separately — written to external config file
  if ("data_dir" in body) {
    const config = readConfig();
    const newDir = String(body.data_dir).trim();
    if (newDir) {
      config.data_dir = newDir;
    } else {
      delete config.data_dir;
    }
    writeConfig(config);
    delete body.data_dir;
  }

  for (const [key, value] of Object.entries(body)) {
    db.insert(settings)
      .values({ key, value: String(value) })
      .onConflictDoUpdate({
        target: settings.key,
        set: { value: String(value) },
      })
      .run();
  }

  const allSettings = db.select().from(settings).all();
  const result: Record<string, string> = {};
  for (const s of allSettings) {
    result[s.key] = s.value;
  }
  result.data_dir = DATA_DIR;
  const config = readConfig();
  if (config.data_dir) {
    result.configured_data_dir = config.data_dir;
  }
  return NextResponse.json(result);
}
