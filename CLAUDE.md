# ReelName -- Agent Guide

## What This Is

A Next.js 16 app (App Router) that scans media files, matches them against TMDB, and transfers them with proper naming to local or SSH destinations. SQLite database, server-side API routes, React client with Zustand state.

## Commands

```bash
pnpm install          # Install dependencies
pnpm dev              # Dev server (Turbopack) -- DO NOT run from agent sessions
pnpm build            # Production build
pnpm lint             # ESLint
npx tsc --noEmit      # Typecheck (safe to run)
pnpm db:push          # Push Drizzle schema to SQLite
```

## Architecture

### Database

SQLite at `data/reelname.db`, managed by Drizzle ORM. Schema is in `src/lib/db/schema.ts`. Migrations are inline in `src/lib/db/index.ts` using `tryExec()` for additive ALTER TABLE statements. The `initializeDatabase()` function creates all tables on first run.

When adding a column: add it to both the schema definition in `schema.ts` AND add a `tryExec("ALTER TABLE ... ADD COLUMN ...")` migration in `index.ts` so existing databases get updated.

Drizzle's `.set()` expects **camelCase JS property names**, not snake_case SQL column names. Drizzle handles the mapping internally via the schema definition. Never manually convert camelCase to snake_case when building update objects.

### API Routes

All under `src/app/api/`. Next.js App Router convention: each `route.ts` exports named HTTP method handlers (`GET`, `POST`, `PATCH`, `DELETE`). Route params are accessed via `{ params }: { params: Promise<{ id: string }> }` and must be awaited.

### State

Zustand store in `src/lib/store/index.ts`. The `GroupWithJobs` type extends `Group` with `jobs: JobWithPreview[]` and `candidates?: MatchCandidate[]`. The store holds groups, filters, selections, active group, settings, and destinations.

### Key Data Flow

- **Groups** contain **Jobs** (one group = one folder, jobs = individual files)
- **MatchCandidates** belong to groups (TMDB search results for review)
- Status progression: `scanned` -> `matched`/`ambiguous` -> `confirmed` -> `transferring` -> `completed`/`failed`
- Confirming a group cascades TMDB info to all child jobs
- For TV groups, episode titles are fetched from TMDB and stored per-job

### File Categories

Jobs have a `fileCategory`: `episode`, `movie`, `special`, `extra`. Specials are Season 0 episodes. Extras have an `extraType` (behind_the_scenes, deleted_scenes, featurettes, interviews, scenes, shorts, trailers, other).

### Transfer System

`src/lib/transfer.ts` manages a queue with max 2 concurrent transfers. Supports local file copy (with resume) and SFTP via ssh2. Progress is tracked per-job in the database and streamed to the client via SSE at `/api/transfer/progress`.

### TMDB Integration

`src/lib/tmdb.ts` wraps the TMDB v3 API with rate limiting (35 req/10s). `src/lib/matcher.ts` scores results using title similarity (Levenshtein), year match, media type consistency, and popularity.

### Naming

`src/lib/naming.ts` formats destination paths. Two presets: `jellyfin` and `plex`. Handles movies, TV episodes, specials, and extras with configurable folder names.

## Conventions

- **Path alias**: `@/*` maps to `./src/*`
- **TypeScript strict mode** is on
- **Client components** use `"use client"` directive
- **Styling**: Tailwind CSS 4 with CSS custom properties for theme colors (defined in `globals.css`). Use existing color tokens: `text-primary`, `text-secondary`, `text-muted`, `bg-primary`, `bg-secondary`, `bg-tertiary`, `bg-hover`, `accent`, `accent-hover`, `border`, `success`, `warning`, `error`, `info`.
- **Toasts**: Import `useToastStore` from `@/components/Toast`, call `useToastStore.getState().addToast(message, type)`.
- **Client API calls**: Add helpers to `src/lib/api.ts`. All return `res.json()` directly.
- **Modals**: Fixed overlay with `z-50`, `bg-black/60` backdrop, close on Escape and backdrop click. See `EpisodeResolveModal.tsx` or `TransferDrawer.tsx`'s `AddDestinationModal` for patterns.
- **Form inputs**: Use consistent class `w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent`.

## Files to Know

| File | What It Does |
|------|-------------|
| `src/lib/db/schema.ts` | All table definitions and exported types |
| `src/lib/db/index.ts` | DB connection, table creation, migrations |
| `src/lib/api.ts` | Every client-side API call |
| `src/lib/store/index.ts` | Zustand store shape and actions |
| `src/lib/scanner.ts` | File discovery and grouping logic |
| `src/lib/parser.ts` | Filename parsing (season, episode, year, quality, codec) |
| `src/lib/matcher.ts` | TMDB matching and scoring algorithm |
| `src/lib/naming.ts` | Destination path formatting |
| `src/lib/transfer.ts` | Transfer queue and SFTP logic |
| `src/app/page.tsx` | Main page, orchestrates data fetching and all panels |
| `src/components/QueueTable.tsx` | Main data table |
| `src/components/MatchPanel.tsx` | TMDB match review side panel |
| `src/components/TransferDrawer.tsx` | Transfer UI and destination management |
