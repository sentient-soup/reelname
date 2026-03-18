"use client";

import { useAppStore } from "@/lib/store";

export function Header({
  onScan,
}: {
  onScan?: () => void;
}) {
  const {
    scanning,
    scanProgress,
    setSettingsOpen,
    groups,
    totalGroups,
    transferDrawerOpen,
    setTransferDrawerOpen,
  } = useAppStore();

  const totalFiles = groups.reduce((sum, g) => sum + g.totalFileCount, 0);

  let progressLabel = "";
  let progressPct = 0;
  if (scanProgress) {
    if (scanProgress.phase === "discovering") {
      progressLabel = "Discovering...";
      progressPct = 0;
    } else if (scanProgress.phase === "scanning") {
      progressLabel = `Scanning ${scanProgress.processed}/${scanProgress.total}`;
      progressPct = scanProgress.total > 0
        ? Math.round((scanProgress.processed / scanProgress.total) * 100)
        : 0;
    } else if (scanProgress.phase === "matching") {
      if (scanProgress.total > 0) {
        progressLabel = `Matching ${scanProgress.processed}/${scanProgress.total}`;
        progressPct = Math.round((scanProgress.processed / scanProgress.total) * 100);
      } else {
        progressLabel = "Matching...";
        progressPct = 100;
      }
    }
  }

  return (
    <header className="flex items-center justify-between px-3 py-2.5 sm:px-6 sm:py-4 border-b border-border bg-bg-secondary">
      <div className="flex items-center gap-2 sm:gap-3 min-w-0 overflow-hidden">
        <div className="flex items-center gap-2 flex-shrink-0">
          <img src="/icon.svg" alt="" width={28} height={28} className="rounded-md" />
          <h1 className="text-lg sm:text-xl font-bold tracking-tight">
            <span className="text-accent">Reel</span>
            <span className="text-text-primary">Name</span>
          </h1>
        </div>
        <span className="text-text-muted text-xs sm:text-sm truncate">
          {totalGroups} group{totalGroups !== 1 ? "s" : ""} ({totalFiles} file
          {totalFiles !== 1 ? "s" : ""})
        </span>
      </div>

      <div className="flex items-center gap-2 sm:gap-3 flex-shrink-0">
        {/* Scan progress — inline in header */}
        {scanProgress && (
          <div className="hidden sm:flex items-center gap-2 mr-1">
            <div className="w-28 h-1.5 rounded-full bg-bg-tertiary overflow-hidden">
              <div
                className="h-full rounded-full bg-accent transition-all duration-300 ease-out"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <span className="text-[11px] text-text-muted whitespace-nowrap">
              {progressLabel}
            </span>
          </div>
        )}

        <button
          onClick={() => setTransferDrawerOpen(!transferDrawerOpen)}
          className="px-2.5 py-1.5 text-xs sm:text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
        >
          Transfers
        </button>
        <button
          onClick={() => setSettingsOpen(true)}
          className="px-2.5 py-1.5 text-xs sm:text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
        >
          Settings
        </button>
        <button
          onClick={onScan}
          disabled={scanning}
          className="px-3 py-1.5 text-xs sm:text-sm font-medium rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {scanning ? (
            <span className="flex items-center gap-2">
              <span className="animate-spin inline-block w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full" />
              Working...
            </span>
          ) : (
            "Scan"
          )}
        </button>
      </div>
    </header>
  );
}
