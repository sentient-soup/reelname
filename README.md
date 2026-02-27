# ReelName

Media file renaming and transfer tool. Scans folders of video files, identifies them against The Movie Database, generates properly formatted filenames for Jellyfin or Plex, and transfers them to local or remote destinations via SFTP.

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
- **Electron** for desktop distribution (tray-only, no window)

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

## Desktop App (Electron)

ReelName can be packaged as a desktop app with a system tray icon. The Electron wrapper spawns the Next.js server as a separate child process and opens your default browser -- there is no Electron window.

```
Electron main process (tray icon)
  └─ spawns: bundled node → server.js (Next.js standalone)
  └─ opens: default browser → http://reelname.localhost:5267
```

### Why a Separate Node Process?

The Next.js server uses native modules (better-sqlite3, ssh2) compiled against the system Node ABI. Rather than rebuilding them against Electron's Node ABI, the server runs in a bundled Node.js binary. This adds ~30-50MB but eliminates native module build complexity.

### Data Directory

In desktop mode, Electron sets `REELNAME_DATA_DIR` to the platform-appropriate location:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%/ReelName` |
| macOS | `~/Library/Application Support/ReelName` |
| Linux | `~/.config/ReelName` |

The data directory is configurable in Settings (requires restart).

### Dev Testing

```bash
pnpm electron:dev
```

Builds the standalone output if missing, copies static assets, and launches Electron.

### Building the Installer

```bash
pnpm electron:build          # auto-detects current platform
pnpm electron:build:win      # Windows NSIS installer
pnpm electron:build:mac      # macOS DMG
pnpm electron:build:linux    # Linux AppImage + deb
```

The build pipeline (`scripts/build-electron.js`):
1. Runs `pnpm build` (produces `.next/standalone/`)
2. Copies `.next/static/` and `public/` into the standalone output
3. Downloads a Node.js LTS binary for the target platform
4. Runs `electron-builder` to produce the installer

Output goes to `dist-electron/`.

## Scripts

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start dev server (Turbopack) |
| `pnpm build` | Production build (standalone output) |
| `pnpm start` | Start production server |
| `pnpm lint` | Run ESLint |
| `pnpm db:push` | Push schema changes to SQLite |
| `pnpm db:studio` | Open Drizzle Studio |
| `pnpm electron:dev` | Launch Electron tray wrapper (dev) |
| `pnpm electron:build` | Build installer for current platform |
| `pnpm electron:build:win` | Build Windows NSIS installer |
| `pnpm electron:build:mac` | Build macOS DMG |
| `pnpm electron:build:linux` | Build Linux AppImage + deb |

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
electron/
  main.js                 # Tray icon, server management, lifecycle
  preload.js              # Empty (electron-builder requirement)
scripts/
  build-electron.js       # Full build pipeline
  electron-dev.js         # Dev launcher
build-resources/          # Electron build assets
  icon.png                # 512px app icon (tray, Linux)
  icon.ico                # Multi-size Windows icon (16-256px)
  icon-1024.png           # macOS hi-res icon
icons/                    # Source icon pack (all sizes + dark variants)
public/                   # Web static assets
  favicon.ico             # Browser tab favicon
  icon.svg                # Modern SVG favicon
  apple-touch-icon.png    # iOS/Safari bookmarks
```

## Icons

Source icons live in `icons/` (with `icons/dark/` variants). They are distributed to:

| File | Used for |
|---|---|
| `public/favicon.ico` | Browser tab favicon |
| `public/icon.svg` | Modern browser SVG favicon |
| `public/apple-touch-icon.png` | iOS/Safari bookmarks |
| `build-resources/icon.png` | Electron tray icon, Linux builds |
| `build-resources/icon.ico` | Windows exe, installer, taskbar, search |
| `build-resources/icon-1024.png` | macOS DMG/app icon |
