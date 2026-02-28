import { useAppStore } from "@/lib/store";

export function Pagination({ onRefresh }: { onRefresh: () => void }) {
  const { page, setPage, totalGroups } = useAppStore();
  const limit = 50;
  const totalPages = Math.max(1, Math.ceil(totalGroups / limit));

  if (totalPages <= 1) return null;

  const handlePrev = () => {
    if (page > 1) {
      setPage(page - 1);
      onRefresh();
    }
  };

  const handleNext = () => {
    if (page < totalPages) {
      setPage(page + 1);
      onRefresh();
    }
  };

  return (
    <div className="flex items-center justify-between px-6 py-2 border-t border-border bg-bg-secondary/50 text-xs text-text-muted">
      <span>
        Page {page} of {totalPages} ({totalGroups} group{totalGroups !== 1 ? "s" : ""})
      </span>
      <div className="flex gap-2">
        <button
          onClick={handlePrev}
          disabled={page <= 1}
          className="px-2.5 py-1 rounded bg-bg-tertiary text-text-secondary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        >
          Prev
        </button>
        <button
          onClick={handleNext}
          disabled={page >= totalPages}
          className="px-2.5 py-1 rounded bg-bg-tertiary text-text-secondary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        >
          Next
        </button>
      </div>
    </div>
  );
}
