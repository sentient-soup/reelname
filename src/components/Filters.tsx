"use client";

import { useAppStore } from "@/lib/store";
import { bulkAction } from "@/lib/api";

const STATUSES = [
  "scanned", "matched", "ambiguous", "confirmed",
  "transferring", "completed", "failed", "skipped",
];

const MEDIA_TYPES = ["movie", "tv", "unknown"];

export function Filters({ onRefresh }: { onRefresh: () => void }) {
  const {
    statusFilter,
    setStatusFilter,
    mediaTypeFilter,
    setMediaTypeFilter,
    searchQuery,
    setSearchQuery,
    selectedGroupIds,
    clearSelection,
  } = useAppStore();

  const handleBulk = async (action: string) => {
    const ids = Object.keys(selectedGroupIds).map(Number);
    if (ids.length === 0) return;
    await bulkAction(action, { groupIds: ids });
    clearSelection();
    onRefresh();
  };

  return (
    <div className="flex flex-col gap-3 px-6 py-3 border-b border-border bg-bg-secondary/50">
      <div className="flex items-center gap-3 flex-wrap">
        {/* Search */}
        <input
          type="text"
          placeholder="Search groups..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="px-3 py-1.5 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent w-64"
        />

        {/* Status filter */}
        <select
          value={statusFilter || ""}
          onChange={(e) => setStatusFilter(e.target.value || null)}
          className="px-3 py-1.5 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="">All statuses</option>
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {s.charAt(0).toUpperCase() + s.slice(1)}
            </option>
          ))}
        </select>

        {/* Media type filter */}
        <select
          value={mediaTypeFilter || ""}
          onChange={(e) => setMediaTypeFilter(e.target.value || null)}
          className="px-3 py-1.5 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="">All types</option>
          {MEDIA_TYPES.map((t) => (
            <option key={t} value={t}>
              {t === "tv" ? "TV" : t.charAt(0).toUpperCase() + t.slice(1)}
            </option>
          ))}
        </select>

        <div className="flex-1" />

        {/* Bulk actions */}
        {Object.keys(selectedGroupIds).length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-xs text-text-muted">
              {Object.keys(selectedGroupIds).length} selected:
            </span>
            <button
              onClick={() => handleBulk("confirm")}
              className="px-2.5 py-1 text-xs rounded bg-status-confirmed text-white hover:opacity-90 transition-opacity"
            >
              Confirm
            </button>
            <button
              onClick={() => handleBulk("skip")}
              className="px-2.5 py-1 text-xs rounded bg-status-skipped text-white hover:opacity-90 transition-opacity"
            >
              Skip
            </button>
            <button
              onClick={() => handleBulk("rematch")}
              className="px-2.5 py-1 text-xs rounded bg-status-ambiguous text-white hover:opacity-90 transition-opacity"
            >
              Rematch
            </button>
            <button
              onClick={() => handleBulk("delete")}
              className="px-2.5 py-1 text-xs rounded bg-status-failed text-white hover:opacity-90 transition-opacity"
            >
              Delete
            </button>
            <button
              onClick={clearSelection}
              className="px-2.5 py-1 text-xs rounded bg-bg-tertiary text-text-secondary hover:bg-bg-hover transition-colors"
            >
              Clear
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
