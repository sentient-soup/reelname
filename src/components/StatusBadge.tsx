const STATUS_CONFIG: Record<string, { label: string; color: string }> = {
  scanned: { label: "Scanned", color: "bg-status-scanned" },
  matched: { label: "Matched", color: "bg-status-matched" },
  ambiguous: { label: "Ambiguous", color: "bg-status-ambiguous" },
  confirmed: { label: "Confirmed", color: "bg-status-confirmed" },
  transferring: { label: "Transferring", color: "bg-status-transferring" },
  completed: { label: "Completed", color: "bg-status-completed" },
  failed: { label: "Failed", color: "bg-status-failed" },
  skipped: { label: "Skipped", color: "bg-status-skipped" },
};

const MEDIA_TYPE_CONFIG: Record<string, { label: string; color: string }> = {
  movie: { label: "Movie", color: "bg-info" },
  tv: { label: "TV", color: "bg-accent" },
  unknown: { label: "?", color: "bg-bg-tertiary" },
};

const FILE_CATEGORY_CONFIG: Record<string, { label: string; color: string }> = {
  episode: { label: "Episode", color: "bg-accent/70" },
  movie: { label: "Movie", color: "bg-info/70" },
  special: { label: "Special", color: "bg-warning/70" },
  extra: { label: "Extra", color: "bg-bg-tertiary" },
};

export function StatusBadge({ status }: { status: string }) {
  const config = STATUS_CONFIG[status] || { label: status, color: "bg-bg-tertiary" };
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium text-white ${config.color}`}
    >
      {config.label}
    </span>
  );
}

export function MediaTypeBadge({ type }: { type: string }) {
  const config = MEDIA_TYPE_CONFIG[type] || { label: type, color: "bg-bg-tertiary" };
  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider text-white ${config.color}`}
    >
      {config.label}
    </span>
  );
}

export function FileCategoryBadge({ category }: { category: string }) {
  const config = FILE_CATEGORY_CONFIG[category] || { label: category, color: "bg-bg-tertiary" };
  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium text-white ${config.color}`}
    >
      {config.label}
    </span>
  );
}
