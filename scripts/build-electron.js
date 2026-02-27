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
// Bundled Node version MUST match the version used to compile native modules
// (i.e., the Node version that ran `pnpm install`). If they differ, native
// addons like better-sqlite3 will fail with NODE_MODULE_VERSION mismatch.
// Auto-detect from the current Node version at build time.
const NODE_VERSION = process.versions.node;

// ── Helpers ────────────────────────────────────────────

function run(cmd, opts = {}) {
  console.log(`\n> ${cmd}`);
  execSync(cmd, { cwd: ROOT, stdio: "inherit", ...opts });
}

function copyDir(src, dest) {
  fs.cpSync(src, dest, { recursive: true });
}

function flattenNodeModules(nmDir) {
  // pnpm standalone output uses a .pnpm virtual store with symlinks.
  // Standard Node.js resolution can't navigate this in a packaged app.
  // Solution: hoist all packages from .pnpm into a flat node_modules.
  const pnpmDir = path.join(nmDir, ".pnpm");
  if (!fs.existsSync(pnpmDir)) return;

  // Walk .pnpm/*/node_modules/* and copy each package to the top level
  for (const storeEntry of fs.readdirSync(pnpmDir)) {
    const innerNm = path.join(pnpmDir, storeEntry, "node_modules");
    if (!fs.existsSync(innerNm)) continue;

    for (const pkg of fs.readdirSync(innerNm, { withFileTypes: true })) {
      const srcPkg = path.join(innerNm, pkg.name);
      const destPkg = path.join(nmDir, pkg.name);

      // Resolve symlinks to real paths
      let realSrc;
      try {
        realSrc = fs.realpathSync(srcPkg);
      } catch {
        continue;
      }

      // Skip if already exists at top level (first copy wins — top-level
      // packages from the original output take priority)
      if (fs.existsSync(destPkg)) continue;

      // Handle scoped packages (@org/name)
      if (pkg.name.startsWith("@")) {
        fs.mkdirSync(destPkg, { recursive: true });
        for (const scopedPkg of fs.readdirSync(realSrc, { withFileTypes: true })) {
          const scopedSrc = path.join(realSrc, scopedPkg.name);
          const scopedDest = path.join(destPkg, scopedPkg.name);
          if (!fs.existsSync(scopedDest)) {
            const realScoped = fs.realpathSync(scopedSrc);
            fs.cpSync(realScoped, scopedDest, { recursive: true, dereference: true });
          }
        }
      } else {
        fs.cpSync(realSrc, destPkg, { recursive: true, dereference: true });
      }
    }
  }

  // Remove .pnpm directory — no longer needed
  fs.rmSync(pnpmDir, { recursive: true, force: true });

  // Dereference any remaining symlinks at top level
  for (const entry of fs.readdirSync(nmDir, { withFileTypes: true })) {
    const fullPath = path.join(nmDir, entry.name);
    if (entry.isSymbolicLink()) {
      const target = fs.realpathSync(fullPath);
      fs.rmSync(fullPath, { recursive: true, force: true });
      fs.cpSync(target, fullPath, { recursive: true, dereference: true });
    }
  }
}

function pruneStandalone(nmDir) {
  // Remove top-level packages that Next.js standalone incorrectly includes.
  // IMPORTANT: Do NOT prune anything under next/dist/compiled/ — Next.js
  // cross-references these modules at server startup in unpredictable ways
  // (e.g. @edge-runtime/cookies is used by the Node server, not just edge).
  const prunePackages = [
    "typescript",       // devDep, 23MB — not needed at runtime
  ];

  for (const pkg of prunePackages) {
    const pkgPath = path.join(nmDir, pkg);
    if (fs.existsSync(pkgPath)) {
      fs.rmSync(pkgPath, { recursive: true, force: true });
      console.log(`  Pruned ${pkg}/`);
    }
  }

  // Only prune next/dist/esm — a duplicate ESM build of the entire dist that
  // is never loaded by the standalone Node.js server (it uses CJS).
  const esmDir = path.join(nmDir, "next", "dist", "esm");
  if (fs.existsSync(esmDir)) {
    const size = execSync(`du -sh "${esmDir}"`, { encoding: "utf-8" }).trim().split("\t")[0];
    fs.rmSync(esmDir, { recursive: true, force: true });
    console.log(`  Pruned next/dist/esm/ (${size})`);
  }
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
  const downloadNodeOnly = process.argv.includes("--download-node-only");

  if (downloadNodeOnly) {
    console.log(`Downloading Node.js binary for platform: ${platform}`);
    await downloadNodeBinary(platform);
    return;
  }

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

  // Step 2b: Flatten pnpm node_modules into standard layout
  // pnpm's .pnpm virtual store uses symlinks that break when packaged.
  // Hoist all packages to a flat node_modules so standard Node resolution works.
  console.log("\n=== Step 2b: Flattening node_modules ===");
  const nmDir = path.join(STANDALONE, "node_modules");
  if (fs.existsSync(nmDir)) {
    flattenNodeModules(nmDir);
    console.log("Flattened standalone/node_modules/ for packaging");

    console.log("\n=== Step 2c: Pruning unnecessary packages ===");
    pruneStandalone(nmDir);
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
  run(`pnpm exec electron-builder ${platformFlag} --config electron-builder.js`, {
    env: { ...process.env, ...signingEnv },
  });

  console.log("\n=== Build complete! Check dist-electron/ ===");
}

main().catch((err) => {
  console.error("Build failed:", err);
  process.exit(1);
});
