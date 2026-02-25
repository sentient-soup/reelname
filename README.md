# ReelName

Media file ingestion, TMDB matching, and renaming tool. Scans folders of video files, identifies them against The Movie Database, generates properly formatted filenames for Jellyfin or Plex, and transfers them to local or remote destinations via SFTP.

## Workflow

1. **Scan** -- Point at a folder of media. ReelName walks the directory tree, groups files by folder, and parses titles, years, seasons, and episodes from filenames.
2. **Match** -- Each group is searched against TMDB. High-confidence matches are auto-confirmed; ambiguous ones are flagged for review.
3. **Confirm** -- Review matches in the side panel. Override TMDB picks, manually search, or resolve individual episodes for TV shows.
4. **Transfer** -- Select confirmed groups, pick a destination (local path or SSH/SFTP), and transfer. Files are renamed according to the chosen naming preset on the way out.

## Tech Stack

- **Next.js 16** (App Router, Turbopack dev)
- **SQLite** via better-sqlite3 + Drizzle ORM
- **Tailwind CSS 4**, dark theme
- **Zustand** for client state
- **Framer Motion** for animations
- **ssh2** for SFTP transfers
- **TMDB API** for metadata

## Getting Started

### Prerequisites

- Node.js 18+
- pnpm

### Install and Run

```bash
pnpm install
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000). The SQLite database is auto-created at `data/reelname.db` on first run.

### Configuration

Open Settings (gear icon or press `,`):

| Setting | Description |
|---------|-------------|
| Scan Path | Root directory containing media folders |
| TMDB API Key | Required for matching. Get one at [themoviedb.org](https://www.themoviedb.org/settings/api) |
| Auto-Match Threshold | Confidence score (0-1) above which matches auto-confirm. Default: 0.85 |
| Naming Preset | `jellyfin` or `plex` formatting rules |
| Specials Folder | Folder name for Season 0 / specials (default: `Specials`) |
| Extras Folder | Folder name for extras (default: `Extras`) |

### Destinations

Destinations are configured in the Transfer drawer. Two types:

- **Local** -- A filesystem path on the same machine.
- **SSH/SFTP** -- Remote server with host, port, username, SSH key path, and optional key passphrase. Use "Test Connection" to validate before saving.

## Naming Presets

**Jellyfin:**
```
Movies:  {Title} ({Year})/{Title} ({Year}).{ext}
TV:      {Title} ({Year})/Season {SS}/{Title} S{SS}E{EE} - {Episode Title}.{ext}
```

**Plex:**
```
Movies:  {Title} ({Year})/{Title} ({Year}).{ext}
TV:      {Title} ({Year})/Season {SS}/{Title} ({Year}) - s{SS}e{EE} - {Episode Title}.{ext}
```

Specials go into a configurable Specials folder under Season 00. Extras (behind the scenes, deleted scenes, featurettes, etc.) go into their own subfolder under the configured Extras folder.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `R` | Refresh |
| `S` | Scan |
| `,` | Open settings |
| `Ctrl+A` | Select all groups |
| `Ctrl+D` | Deselect all |
| `Arrow Up/Down` | Navigate groups |
| `Escape` | Close active panel |

## Scripts

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start dev server (Turbopack) |
| `pnpm build` | Production build |
| `pnpm start` | Start production server |
| `pnpm lint` | Run ESLint |
| `pnpm db:push` | Push schema changes to SQLite |
| `pnpm db:studio` | Open Drizzle Studio |

## Project Structure

```
src/
  app/
    api/
      destinations/       # CRUD + SSH test connection
      groups/             # Group CRUD + TMDB seasons
      jobs/               # Job CRUD + bulk actions
      match/              # TMDB matching trigger
      scan/               # Folder scanning trigger
      search/             # TMDB search proxy
      settings/           # App settings
      transfer/           # Transfer queue + SSE progress
    layout.tsx
    page.tsx              # Main dashboard
    globals.css           # Theme + Tailwind

  components/
    EpisodeResolveModal   # Season/episode picker for TV episode overrides
    Filters               # Search bar, status/type filters, bulk actions
    Header                # Title bar, stats, action buttons
    KeyboardShortcuts     # Global keyboard handlers
    MatchPanel            # Side panel for TMDB match review
    Pagination            # Page navigation
    QueueTable            # Main group/file table
    SettingsModal         # App configuration
    StatusBadge           # Colored status/category labels
    Toast                 # Notification system
    TransferDrawer        # Destination management + transfer progress

  lib/
    db/
      index.ts            # SQLite connection + migrations
      schema.ts           # Drizzle table definitions
    store/
      index.ts            # Zustand state store
    api.ts                # Client-side fetch helpers
    matcher.ts            # TMDB matching algorithm
    naming.ts             # File path formatting
    parser.ts             # Filename parsing (season, episode, quality, codec)
    scanner.ts            # Directory traversal + file grouping
    tmdb.ts               # TMDB API client (rate-limited)
    transfer.ts           # Local + SFTP transfer queue
```
