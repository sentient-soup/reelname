#!/usr/bin/env node

/**
 * Squash-commits current main state onto the release branch.
 *
 * Usage:
 *   pnpm release            # auto-reads version from package.json
 *   pnpm release -- v0.3.0  # override the release label
 */

const { execSync } = require("child_process");

function run(cmd, opts) {
  return execSync(cmd, { encoding: "utf-8", stdio: "pipe", ...opts }).trim();
}

function fail(msg) {
  console.error(`\n  ERROR: ${msg}\n`);
  process.exit(1);
}

// ── Preflight checks ──────────────────────────────────────────────

const currentBranch = run("git rev-parse --abbrev-ref HEAD");
if (currentBranch !== "main") {
  fail("You must be on the main branch to run this script.");
}

const status = run("git status --porcelain");
if (status) {
  fail("Working tree is dirty. Commit or stash your changes first.");
}

// Determine release label
const label =
  process.argv[2] || `v${require("../package.json").version}`;

// Make sure release branch exists
try {
  run("git rev-parse --verify release");
} catch {
  fail(
    'No "release" branch found. Create it first:\n  git checkout --orphan release && git commit --allow-empty -m "init release" && git checkout main'
  );
}

const mainSha = run("git rev-parse --short HEAD");

// ── Build the release commit ──────────────────────────────────────

console.log(`Releasing ${label} (main @ ${mainSha}) onto release branch...`);

run("git checkout release");

try {
  // Clear the working tree and replace with main's content
  run("git rm -rf .", { stdio: "ignore" });
} catch {
  // git rm fails if tree is already empty — that's fine
}

run("git checkout main -- .");
run("git add -A");

// Check if there are actual changes to commit
const diff = run("git diff --cached --stat");
if (!diff) {
  console.log("No changes since last release. Nothing to do.");
  run("git checkout main");
  process.exit(0);
}

run(`git commit -m "${label}"`);
run(`git tag -f ${label}`);
run("git checkout main");

console.log(`\n  Done! Release branch updated with ${label}.`);
console.log(`  Tag "${label}" points to the release commit.`);
console.log(`\n  To push:  git push origin release --tags\n`);
