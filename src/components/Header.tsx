"use client";

import { useAppStore } from "@/lib/store";

export function Header({
  onScan,
  onMatch,
}: {
  onScan?: () => void;
  onMatch?: () => void;
}) {
  const {
    scanning,
    setSettingsOpen,
    selectedGroupIds,
    groups,
    totalGroups,
    transferDrawerOpen,
    setTransferDrawerOpen,
  } = useAppStore();

  const totalFiles = groups.reduce((sum, g) => sum + g.totalFileCount, 0);
  const selectedCount = Object.keys(selectedGroupIds).length;

  return (
    <header className="flex items-center justify-between px-6 py-4 border-b border-border bg-bg-secondary">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2.5">
          <img src="/icon.svg" alt="" width={28} height={28} className="rounded-md" />
          <h1 className="text-xl font-bold tracking-tight">
            <span className="text-accent">Reel</span>
            <span className="text-text-primary">Name</span>
          </h1>
        </div>
        <span className="text-text-muted text-sm">
          {totalGroups} group{totalGroups !== 1 ? "s" : ""} ({totalFiles} file
          {totalFiles !== 1 ? "s" : ""})
          {selectedCount > 0 && (
            <span className="text-accent ml-2">
              ({selectedCount} selected)
            </span>
          )}
        </span>
      </div>

      <div className="flex items-center gap-3">
        <button
          onClick={() => setTransferDrawerOpen(!transferDrawerOpen)}
          className="px-3 py-1.5 text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
        >
          Transfers
        </button>
        <button
          onClick={() => setSettingsOpen(true)}
          className="px-3 py-1.5 text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
        >
          Settings
        </button>
        <button
          onClick={onMatch}
          disabled={scanning}
          className="px-4 py-1.5 text-sm font-medium rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Match
        </button>
        <button
          onClick={onScan}
          disabled={scanning}
          className="px-4 py-1.5 text-sm font-medium rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
