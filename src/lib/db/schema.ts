import { sqliteTable, text, integer, real } from "drizzle-orm/sqlite-core";

export const groups = sqliteTable("groups", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  status: text("status", {
    enum: [
      "scanned",
      "matched",
      "ambiguous",
      "confirmed",
      "queued",
      "transferring",
      "completed",
      "failed",
      "skipped",
    ],
  })
    .notNull()
    .default("scanned"),
  mediaType: text("media_type", { enum: ["movie", "tv", "unknown"] })
    .notNull()
    .default("unknown"),

  // Source info
  folderPath: text("folder_path").notNull(),
  folderName: text("folder_name").notNull(),
  totalFileCount: integer("total_file_count").notNull().default(0),
  totalFileSize: integer("total_file_size").notNull().default(0),

  // Parsed info (from folder name)
  parsedTitle: text("parsed_title"),
  parsedYear: integer("parsed_year"),

  // TMDB info (one match per group)
  tmdbId: integer("tmdb_id"),
  tmdbTitle: text("tmdb_title"),
  tmdbYear: integer("tmdb_year"),
  tmdbPosterPath: text("tmdb_poster_path"),
  matchConfidence: real("match_confidence"),

  // Transfer info
  destinationId: integer("destination_id").references(() => destinations.id),

  createdAt: text("created_at")
    .notNull()
    .$defaultFn(() => new Date().toISOString()),
  updatedAt: text("updated_at")
    .notNull()
    .$defaultFn(() => new Date().toISOString()),
});

export const jobs = sqliteTable("jobs", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  groupId: integer("group_id").references(() => groups.id, {
    onDelete: "cascade",
  }),
  status: text("status", {
    enum: [
      "scanned",
      "matched",
      "ambiguous",
      "confirmed",
      "queued",
      "transferring",
      "completed",
      "failed",
      "skipped",
    ],
  })
    .notNull()
    .default("scanned"),
  mediaType: text("media_type", { enum: ["movie", "tv", "unknown"] })
    .notNull()
    .default("unknown"),
  fileCategory: text("file_category", {
    enum: ["episode", "movie", "special", "extra"],
  })
    .notNull()
    .default("episode"),
  extraType: text("extra_type", {
    enum: [
      "behind_the_scenes",
      "deleted_scenes",
      "featurettes",
      "interviews",
      "scenes",
      "shorts",
      "trailers",
      "other",
    ],
  }),

  // Source info
  sourcePath: text("source_path").notNull(),
  fileName: text("file_name").notNull(),
  fileSize: integer("file_size").notNull(),
  fileExtension: text("file_extension").notNull(),

  // Parsed info
  parsedTitle: text("parsed_title"),
  parsedYear: integer("parsed_year"),
  parsedSeason: integer("parsed_season"),
  parsedEpisode: integer("parsed_episode"),
  parsedQuality: text("parsed_quality"),
  parsedCodec: text("parsed_codec"),

  // TMDB info (episode-level)
  tmdbId: integer("tmdb_id"),
  tmdbTitle: text("tmdb_title"),
  tmdbYear: integer("tmdb_year"),
  tmdbPosterPath: text("tmdb_poster_path"),
  tmdbEpisodeTitle: text("tmdb_episode_title"),
  matchConfidence: real("match_confidence"),

  // Transfer info
  destinationId: integer("destination_id").references(() => destinations.id),
  destinationPath: text("destination_path"),
  transferProgress: real("transfer_progress"),
  transferError: text("transfer_error"),

  createdAt: text("created_at")
    .notNull()
    .$defaultFn(() => new Date().toISOString()),
  updatedAt: text("updated_at")
    .notNull()
    .$defaultFn(() => new Date().toISOString()),
});

export const matchCandidates = sqliteTable("match_candidates", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  jobId: integer("job_id").references(() => jobs.id, { onDelete: "cascade" }),
  groupId: integer("group_id").references(() => groups.id, {
    onDelete: "cascade",
  }),
  tmdbId: integer("tmdb_id").notNull(),
  mediaType: text("media_type", { enum: ["movie", "tv"] }).notNull(),
  title: text("title").notNull(),
  year: integer("year"),
  posterPath: text("poster_path"),
  overview: text("overview"),
  confidence: real("confidence").notNull(),
});

export const destinations = sqliteTable("destinations", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  name: text("name").notNull(),
  type: text("type", { enum: ["local", "ssh"] })
    .notNull()
    .default("local"),
  basePath: text("base_path").notNull(),

  // SSH config
  sshHost: text("ssh_host"),
  sshPort: integer("ssh_port").default(22),
  sshUser: text("ssh_user"),
  sshKeyPath: text("ssh_key_path"),
  sshKeyPassphrase: text("ssh_key_passphrase"),

  // Naming templates (per-destination override)
  movieTemplate: text("movie_template"),
  tvTemplate: text("tv_template"),
});

export const settings = sqliteTable("settings", {
  key: text("key").primaryKey(),
  value: text("value").notNull(),
});

// Types
export type Group = typeof groups.$inferSelect;
export type NewGroup = typeof groups.$inferInsert;
export type Job = typeof jobs.$inferSelect;
export type NewJob = typeof jobs.$inferInsert;
export type MatchCandidate = typeof matchCandidates.$inferSelect;
export type NewMatchCandidate = typeof matchCandidates.$inferInsert;
export type Destination = typeof destinations.$inferSelect;
export type Setting = typeof settings.$inferSelect;
