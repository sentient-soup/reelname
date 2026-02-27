#!/usr/bin/env node

/**
 * Build pipeline for Electron distribution:
 * 1. Run `pnpm build` (produces .next/standalone/)
 * 2. Copy static assets and public folder into standalone
 * 3. Download Node.js LTS binary for the target platform
 * 4. Run electron-builder
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { pipeline } = require("stream/promises");
const { createWriteStream } = require("fs");

const ROOT = path.resolve(__dirname, "..");
const STANDALONE = path.join(ROOT, ".next", "standalone");
const NODE_VERSION = "20.20.0"; // LTS

// ── Helpers ────────────────────────────────────────────

function run(cmd, opts = {}) {
  console.log(`\n> ${cmd}`);
  execSync(cmd, { cwd: ROOT, stdio: "inherit", ...opts });
}

function copyDir(src, dest) {
  fs.cpSync(src, dest, { recursive: true });
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (url) => {
      https
        .get(url, (res) => {
          if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            follow(res.headers.location);
            return;
          }
          if (res.statusCode !== 200) {
            reject(new Error(`Download failed: ${res.statusCode} for ${url}`));
            return;
          }
          const stream = createWriteStream(dest);
          pipeline(res, stream).then(resolve).catch(reject);
        })
        .on("error", reject);
    };
    follow(url);
  });
}

// ── Platform detection ─────────────────────────────────

function getTargetPlatform() {
  const arg = process.argv.find((a) => a.startsWith("--platform="));
  if (arg) return arg.split("=")[1];
  const map = { win32: "win", darwin: "mac", linux: "linux" };
  return map[process.platform] || "linux";
}

function getNodeDownloadInfo(platform) {
  const base = `https://nodejs.org/dist/v${NODE_VERSION}`;
  switch (platform) {
    case "win":
      return {
        url: `${base}/node-v${NODE_VERSION}-win-x64.zip`,
        archive: "zip",
        innerDir: `node-v${NODE_VERSION}-win-x64`,
        binaryName: "node.exe",
      };
    case "mac":
      return {
        url: `${base}/node-v${NODE_VERSION}-darwin-arm64.tar.gz`,
        archive: "tar.gz",
        innerDir: `node-v${NODE_VERSION}-darwin-arm64`,
        binaryName: "node",
      };
    case "linux":
      return {
        url: `${base}/node-v${NODE_VERSION}-linux-x64.tar.xz`,
        archive: "tar.xz",
        innerDir: `node-v${NODE_VERSION}-linux-x64`,
        binaryName: "node",
      };
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
}

// ── Download & extract Node binary ─────────────────────

async function downloadNodeBinary(platform) {
  const info = getNodeDownloadInfo(platform);
  const nodeDir = path.join(ROOT, "build-resources", `${platform}-node`);
  const nodeBin = path.join(nodeDir, info.binaryName);

  if (fs.existsSync(nodeBin)) {
    console.log(`Node binary already exists: ${nodeBin}`);
    return nodeDir;
  }

  fs.mkdirSync(nodeDir, { recursive: true });

  const ext = info.archive === "zip" ? ".zip" : info.archive === "tar.xz" ? ".tar.xz" : ".tar.gz";
  const archivePath = path.join(nodeDir, `node${ext}`);

  console.log(`Downloading Node.js v${NODE_VERSION} for ${platform}...`);
  await download(info.url, archivePath);
  console.log("Download complete. Extracting Node binary...");

  if (info.archive === "zip") {
    // Windows: extract zip, pull out node.exe, clean up
    const tempDir = path.join(nodeDir, "_extract");
    fs.mkdirSync(tempDir, { recursive: true });
    run(`powershell -NoProfile -Command "Expand-Archive -LiteralPath '${archivePath}' -DestinationPath '${tempDir}' -Force"`);
    const extracted = path.join(tempDir, info.innerDir, info.binaryName);
    fs.copyFileSync(extracted, nodeBin);
    fs.rmSync(tempDir, { recursive: true, force: true });
  } else {
    // macOS/Linux: extract just the binary from the tarball
    const tarFlags = info.archive === "tar.xz" ? "xJf" : "xzf";
    const binaryInArchive = info.archive === "tar.gz"
      ? `${info.innerDir}/bin/${info.binaryName}`
      : `${info.innerDir}/bin/${info.binaryName}`;
    run(`tar ${tarFlags} "${archivePath}" -C "${nodeDir}" --strip-components=2 "${binaryInArchive}"`);
    fs.chmodSync(nodeBin, 0o755);
  }

  // Clean up archive
  fs.unlinkSync(archivePath);

  console.log(`Node binary ready: ${nodeBin}`);
  return nodeDir;
}

// ── Main build ─────────────────────────────────────────

async function main() {
  const platform = getTargetPlatform();
  console.log(`Building ReelName for platform: ${platform}`);

  // Step 1: Build Next.js
  console.log("\n=== Step 1: Building Next.js ===");
  run("pnpm build");

  if (!fs.existsSync(path.join(STANDALONE, "server.js"))) {
    throw new Error("Standalone build not found. Ensure output: 'standalone' is set in next.config.ts");
  }

  // Step 2: Copy static assets into standalone output
  console.log("\n=== Step 2: Copying static assets ===");
  const staticSrc = path.join(ROOT, ".next", "static");
  const staticDest = path.join(STANDALONE, ".next", "static");
  if (fs.existsSync(staticSrc)) {
    copyDir(staticSrc, staticDest);
    console.log("Copied .next/static/");
  }

  const publicSrc = path.join(ROOT, "public");
  const publicDest = path.join(STANDALONE, "public");
  if (fs.existsSync(publicSrc)) {
    copyDir(publicSrc, publicDest);
    console.log("Copied public/");
  }

  // Step 3: Download Node binary
  console.log("\n=== Step 3: Downloading Node.js binary ===");
  await downloadNodeBinary(platform);

  // Step 4: Run electron-builder
  // Use pnpm exec so electron-builder correctly detects pnpm (npx falls back to npm)
  console.log("\n=== Step 4: Running electron-builder ===");
  const platformFlag = { win: "--win", mac: "--mac", linux: "--linux" }[platform];
  // Skip code signing unless CSC_LINK is set (signing certificate provided)
  const signingEnv = process.env.CSC_LINK ? {} : { CSC_IDENTITY_AUTO_DISCOVERY: "false" };
  run(`pnpm exec electron-builder ${platformFlag}`, {
    env: { ...process.env, ...signingEnv },
  });

  console.log("\n=== Build complete! Check dist-electron/ ===");
}

main().catch((err) => {
  console.error("Build failed:", err);
  process.exit(1);
});
