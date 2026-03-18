"use client";

import { useAppStore } from "@/lib/store";
import { bulkAction } from "@/lib/api";

const STATUSES = [
  "scanned", "matched", "ambiguous", "confirmed",
  "transferring", "completed", "failed",
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

  const selectedCount = Object.keys(selectedGroupIds).length;
  const hasSelection = selectedCount > 0;

  const handleBulk = async (action: string) => {
    if (!hasSelection) return;
    const ids = Object.keys(selectedGroupIds).map(Number);
    await bulkAction(action, { groupIds: ids });
    clearSelection();
    onRefresh();
  };

  const disabledBtn = "px-3 py-1 text-xs rounded bg-bg-tertiary text-text-muted cursor-not-allowed border border-border";
  const grayBtn = "px-3 py-1 text-xs rounded bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors";
  const primaryBtn = "px-3 py-1 text-xs rounded bg-accent text-white hover:bg-accent-hover transition-colors";

  return (
    <div className="flex items-center gap-2 sm:gap-3 px-3 py-2 sm:px-6 sm:py-2.5 border-b border-border bg-bg-secondary/50 overflow-x-auto">
      {/* Bulk actions — always visible, greyed out when nothing selected */}
      <div className="flex items-center gap-2 flex-shrink-0">
        <button
          onClick={() => handleBulk("confirm")}
          disabled={!hasSelection}
          className={hasSelection ? primaryBtn : disabledBtn}
        >
          Confirm
        </button>
        <button
          onClick={() => handleBulk("rematch")}
          disabled={!hasSelection}
          className={hasSelection ? grayBtn : disabledBtn}
        >
          Rematch
        </button>
        <button
          onClick={() => handleBulk("delete")}
          disabled={!hasSelection}
          className={hasSelection ? grayBtn : disabledBtn}
        >
          Delete
        </button>
        {hasSelection && (
          <span className="text-xs text-text-muted">
            {selectedCount} selected
          </span>
        )}
      </div>

      <div className="flex-1" />

      {/* Filters — right side */}
      <div className="flex items-center gap-2 flex-shrink-0">
        <select
          value={statusFilter || ""}
          onChange={(e) => setStatusFilter(e.target.value || null)}
          className="px-2 py-1.5 text-xs rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="">All statuses</option>
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {s.charAt(0).toUpperCase() + s.slice(1)}
            </option>
          ))}
        </select>

        <select
          value={mediaTypeFilter || ""}
          onChange={(e) => setMediaTypeFilter(e.target.value || null)}
          className="px-2 py-1.5 text-xs rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="">All types</option>
          {MEDIA_TYPES.map((t) => (
            <option key={t} value={t}>
              {t === "tv" ? "TV" : t.charAt(0).toUpperCase() + t.slice(1)}
            </option>
          ))}
        </select>

        <input
          type="text"
          placeholder="Search..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="px-2 py-1.5 text-xs rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent w-40"
        />
      </div>
    </div>
  );
}
