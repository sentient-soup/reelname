#!/usr/bin/env node

/**
 * Dev launcher for testing the Electron tray wrapper.
 * Builds the Next.js standalone output if missing, then launches Electron.
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..");
const STANDALONE = path.join(ROOT, ".next", "standalone", "server.js");

function run(cmd) {
  console.log(`> ${cmd}`);
  execSync(cmd, { cwd: ROOT, stdio: "inherit" });
}

// Build standalone if not present
if (!fs.existsSync(STANDALONE)) {
  console.log("Standalone build not found. Building...\n");
  run("pnpm build");
  console.log("\nStandalone build ready.\n");
}

// Always copy static assets — standalone output doesn't include them
const staticSrc = path.join(ROOT, ".next", "static");
const staticDest = path.join(ROOT, ".next", "standalone", ".next", "static");
if (fs.existsSync(staticSrc)) {
  fs.cpSync(staticSrc, staticDest, { recursive: true });
  console.log("Copied .next/static/ into standalone");
}

const publicSrc = path.join(ROOT, "public");
const publicDest = path.join(ROOT, ".next", "standalone", "public");
if (fs.existsSync(publicSrc)) {
  fs.cpSync(publicSrc, publicDest, { recursive: true });
  console.log("Copied public/ into standalone");
}

// Launch Electron
console.log("\nStarting Electron...\n");
run("npx electron electron/main.js");
