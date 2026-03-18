import fs from "fs";
import path from "path";
import { Client as SSHClient } from "ssh2";
import { db } from "./db";
import { jobs, groups, destinations, subtitleFiles, settings } from "./db/schema";
import { eq } from "drizzle-orm";
import { formatGroupedPath } from "./naming";
import type { Job, Destination } from "./db/schema";

const MAX_CONCURRENT = 2;
let activeTransfers = 0;
const transferQueue: Array<{ jobId: number; destinationId: number }> = [];

function updateJobProgress(
  jobId: number,
  progress: number,
  error?: string | null
) {
  const status =
    error ? "failed" : progress >= 1 ? "completed" : "transferring";
  db.update(jobs)
    .set({
      transferProgress: progress,
      transferError: error ?? null,
      status,
      updatedAt: new Date().toISOString(),
    })
    .where(eq(jobs.id, jobId))
    .run();
}

/**
 * Build the relative destination path for a job using group context and naming presets.
 */
function buildRelativePath(job: Job): string {
  // Look up the group for this job
  const group = job.groupId
    ? db.select().from(groups).where(eq(groups.id, job.groupId)).get()
    : null;

  // Look up naming settings
  const namingPreset =
    db.select().from(settings).where(eq(settings.key, "naming_preset")).get()?.value || "jellyfin";
  const specialsFolderName =
    db.select().from(settings).where(eq(settings.key, "specials_folder_name")).get()?.value || "Specials";
  const extrasFolderName =
    db.select().from(settings).where(eq(settings.key, "extras_folder_name")).get()?.value || "Extras";

  if (group) {
    return formatGroupedPath(job, group, {
      naming_preset: namingPreset,
      specials_folder_name: specialsFolderName,
      extras_folder_name: extrasFolderName,
    });
  }

  // Fallback for ungrouped jobs: construct a minimal group-like object
  return formatGroupedPath(job, {
    id: 0,
    status: "matched",
    mediaType: job.mediaType,
    folderPath: "",
    folderName: "",
    totalFileCount: 1,
    totalFileSize: job.fileSize,
    parsedTitle: job.parsedTitle,
    parsedYear: job.parsedYear,
    tmdbId: job.tmdbId,
    tmdbTitle: job.tmdbTitle,
    tmdbYear: job.tmdbYear,
    tmdbPosterPath: job.tmdbPosterPath,
    matchConfidence: job.matchConfidence,
    destinationId: job.destinationId,
    createdAt: job.createdAt,
    updatedAt: job.updatedAt,
  }, {
    naming_preset: namingPreset,
    specials_folder_name: specialsFolderName,
    extras_folder_name: extrasFolderName,
  });
}

/**
 * Build subtitle destination path by replacing the media extension with
 * the subtitle extension (and optional language code).
 * E.g. "Show S01E01 - Title.mkv" → "Show S01E01 - Title.en.srt"
 */
function buildSubtitlePath(
  mediaRelativePath: string,
  subtitleExt: string,
  languageCode: string | null
): string {
  const lastDot = mediaRelativePath.lastIndexOf(".");
  const basePath = lastDot !== -1 ? mediaRelativePath.slice(0, lastDot) : mediaRelativePath;
  const ext = subtitleExt.startsWith(".") ? subtitleExt : `.${subtitleExt}`;
  return languageCode ? `${basePath}.${languageCode}${ext}` : `${basePath}${ext}`;
}

/**
 * Transfer subtitle files for a completed job (local copy, fire-and-forget)
 */
function transferSubtitlesLocal(
  jobId: number,
  mediaRelativePath: string,
  destBasePath: string
) {
  const subs = db
    .select()
    .from(subtitleFiles)
    .where(eq(subtitleFiles.jobId, jobId))
    .all();

  for (const sub of subs) {
    try {
      const subRelPath = buildSubtitlePath(
        mediaRelativePath,
        sub.fileExtension,
        sub.languageCode
      );
      const fullDest = path.join(destBasePath, subRelPath);
      fs.mkdirSync(path.dirname(fullDest), { recursive: true });
      fs.copyFileSync(sub.sourcePath, fullDest);
    } catch (err) {
      console.error(
        `Failed to copy subtitle ${sub.sourcePath}:`,
        err instanceof Error ? err.message : err
      );
    }
  }
}

/**
 * Transfer subtitle files for a completed job via SFTP
 */
async function transferSubtitlesSFTP(
  jobId: number,
  mediaRelativePath: string,
  dest: Destination
): Promise<void> {
  const subs = db
    .select()
    .from(subtitleFiles)
    .where(eq(subtitleFiles.jobId, jobId))
    .all();

  if (subs.length === 0) return;

  return new Promise((resolve, reject) => {
    const conn = new SSHClient();

    conn.on("ready", () => {
      conn.sftp((err, sftp) => {
        if (err) {
          conn.end();
          reject(err);
          return;
        }

        let completed = 0;
        const total = subs.length;

        for (const sub of subs) {
          const subRelPath = buildSubtitlePath(
            mediaRelativePath,
            sub.fileExtension,
            sub.languageCode
          );
          const fullDest =
            dest.basePath.replace(/\\/g, "/") +
            "/" +
            subRelPath.replace(/\\/g, "/");

          // Create remote directories then upload
          const dirs = path.dirname(fullDest).split("/").filter(Boolean);
          let currentDir = "/";
          const mkdirAndUpload = (index: number) => {
            if (index >= dirs.length) {
              // Upload the file
              const readStream = fs.createReadStream(sub.sourcePath);
              const writeStream = sftp.createWriteStream(fullDest);
              writeStream.on("close", () => {
                completed++;
                if (completed === total) {
                  conn.end();
                  resolve();
                }
              });
              writeStream.on("error", (writeErr: Error) => {
                console.error(
                  `Failed to SFTP subtitle ${sub.sourcePath}:`,
                  writeErr.message
                );
                completed++;
                if (completed === total) {
                  conn.end();
                  resolve();
                }
              });
              readStream.pipe(writeStream);
              return;
            }
            currentDir += (currentDir === "/" ? "" : "/") + dirs[index];
            sftp.mkdir(currentDir, () => mkdirAndUpload(index + 1));
          };
          mkdirAndUpload(0);
        }
      });
    });

    conn.on("error", (connErr) => {
      console.error("SFTP subtitle connection error:", connErr.message);
      resolve(); // Don't fail the main transfer for subtitle errors
    });

    const connectConfig: Record<string, unknown> = {
      host: dest.sshHost!,
      port: dest.sshPort || 22,
      username: dest.sshUser!,
    };

    if (dest.sshKeyPath) {
      connectConfig.privateKey = fs.readFileSync(dest.sshKeyPath);
      if (dest.sshKeyPassphrase) {
        connectConfig.passphrase = dest.sshKeyPassphrase;
      }
    }

    conn.connect(connectConfig);
  });
}

/**
 * Local file copy with progress tracking
 */
async function transferLocal(
  job: Job,
  dest: Destination
): Promise<void> {
  const relativePath = buildRelativePath(job);
  const fullDest = path.join(dest.basePath, relativePath);

  // Create directory structure
  fs.mkdirSync(path.dirname(fullDest), { recursive: true });

  const totalSize = job.fileSize;
  let transferred = 0;

  // Check for partial file (resume support)
  if (fs.existsSync(fullDest)) {
    const existingStat = fs.statSync(fullDest);
    if (existingStat.size === totalSize) {
      // Already complete
      updateJobProgress(job.id, 1);
      return;
    }
    if (existingStat.size < totalSize) {
      transferred = existingStat.size;
    }
  }

  return new Promise((resolve, reject) => {
    const readStream = fs.createReadStream(job.sourcePath, {
      start: transferred,
    });
    const writeStream = fs.createWriteStream(fullDest, {
      flags: transferred > 0 ? "a" : "w",
    });

    readStream.on("data", (chunk) => {
      transferred += chunk.length;
      const progress = Math.min(transferred / totalSize, 1);
      updateJobProgress(job.id, progress);
    });

    readStream.on("error", (err) => {
      updateJobProgress(job.id, transferred / totalSize, err.message);
      reject(err);
    });

    writeStream.on("error", (err) => {
      updateJobProgress(job.id, transferred / totalSize, err.message);
      reject(err);
    });

    writeStream.on("finish", () => {
      updateJobProgress(job.id, 1);

      // Save destination path on the job
      db.update(jobs)
        .set({
          destinationId: dest.id,
          destinationPath: fullDest,
          updatedAt: new Date().toISOString(),
        })
        .where(eq(jobs.id, job.id))
        .run();

      resolve();
    });

    readStream.pipe(writeStream);
  });
}

/**
 * SFTP transfer with progress tracking
 */
async function transferSFTP(
  job: Job,
  dest: Destination
): Promise<void> {
  const relativePath = buildRelativePath(job);
  // Use forward slashes for remote path
  const fullDest = dest.basePath.replace(/\\/g, "/") + "/" + relativePath.replace(/\\/g, "/");

  return new Promise((resolve, reject) => {
    const conn = new SSHClient();

    conn.on("ready", () => {
      conn.sftp((err, sftp) => {
        if (err) {
          updateJobProgress(job.id, 0, err.message);
          conn.end();
          reject(err);
          return;
        }

        // Create remote directories
        const dirs = path.dirname(fullDest).split("/").filter(Boolean);
        let currentDir = "/";
        const mkdirRecursive = (index: number) => {
          if (index >= dirs.length) {
            doTransfer(sftp);
            return;
          }
          currentDir += (currentDir === "/" ? "" : "/") + dirs[index];
          sftp.mkdir(currentDir, (mkErr) => {
            // Ignore EEXIST errors
            mkdirRecursive(index + 1);
          });
        };

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const doTransfer = (sftpStream: any) => {
          const totalSize = job.fileSize;
          let transferred = 0;

          const readStream = fs.createReadStream(job.sourcePath);
          const writeStream = sftpStream.createWriteStream(fullDest);

          readStream.on("data", (chunk) => {
            transferred += chunk.length;
            const progress = Math.min(transferred / totalSize, 1);
            updateJobProgress(job.id, progress);
          });

          readStream.on("error", (readErr) => {
            updateJobProgress(job.id, transferred / totalSize, readErr.message);
            conn.end();
            reject(readErr);
          });

          writeStream.on("error", (writeErr: Error) => {
            updateJobProgress(
              job.id,
              transferred / totalSize,
              writeErr.message
            );
            conn.end();
            reject(writeErr);
          });

          writeStream.on("close", () => {
            updateJobProgress(job.id, 1);
            db.update(jobs)
              .set({
                destinationId: dest.id,
                destinationPath: fullDest,
                updatedAt: new Date().toISOString(),
              })
              .where(eq(jobs.id, job.id))
              .run();
            conn.end();
            resolve();
          });

          readStream.pipe(writeStream);
        };

        mkdirRecursive(0);
      });
    });

    conn.on("error", (connErr) => {
      updateJobProgress(job.id, 0, connErr.message);
      reject(connErr);
    });

    const connectConfig: Record<string, unknown> = {
      host: dest.sshHost!,
      port: dest.sshPort || 22,
      username: dest.sshUser!,
    };

    if (dest.sshKeyPath) {
      connectConfig.privateKey = fs.readFileSync(dest.sshKeyPath);
      if (dest.sshKeyPassphrase) {
        connectConfig.passphrase = dest.sshKeyPassphrase;
      }
    }

    conn.connect(connectConfig);
  });
}

/**
 * Process a single transfer
 */
async function processTransfer(jobId: number, destinationId: number) {
  activeTransfers++;

  try {
    const job = db.select().from(jobs).where(eq(jobs.id, jobId)).get();
    const dest = db
      .select()
      .from(destinations)
      .where(eq(destinations.id, destinationId))
      .get();

    if (!job || !dest) {
      throw new Error("Job or destination not found");
    }

    // Mark as transferring
    db.update(jobs)
      .set({
        status: "transferring",
        transferProgress: 0,
        transferError: null,
        updatedAt: new Date().toISOString(),
      })
      .where(eq(jobs.id, jobId))
      .run();

    const relativePath = buildRelativePath(job);

    if (dest.type === "ssh") {
      await transferSFTP(job, dest);
      await transferSubtitlesSFTP(jobId, relativePath, dest).catch((err) =>
        console.error("Subtitle SFTP error:", err)
      );
    } else {
      await transferLocal(job, dest);
      transferSubtitlesLocal(jobId, relativePath, dest.basePath);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : "Transfer failed";
    updateJobProgress(jobId, 0, message);
  }

  activeTransfers--;
  processQueue();
}

/**
 * Process next items from the queue
 */
function processQueue() {
  while (activeTransfers < MAX_CONCURRENT && transferQueue.length > 0) {
    const next = transferQueue.shift()!;
    processTransfer(next.jobId, next.destinationId);
  }
}

/**
 * Queue transfers for execution
 */
export function queueTransfers(
  jobIds: number[],
  destinationId: number
): { queued: number } {
  for (const jobId of jobIds) {
    transferQueue.push({ jobId, destinationId });
  }
  processQueue();
  return { queued: jobIds.length };
}
