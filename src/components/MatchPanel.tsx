import { useAppStore } from "@/lib/store";
import { updateGroup, searchTmdb } from "@/lib/api";
import { useToastStore } from "./Toast";
import { useState } from "react";
import { StatusBadge, FileCategoryBadge } from "./StatusBadge";
import { EpisodeResolveModal } from "./EpisodeResolveModal";
import type { MatchCandidate } from "@/lib/types";
import type { JobWithPreview } from "@/lib/store";

const TMDB_IMG_BASE = "https://image.tmdb.org/t/p/w185";

export function MatchPanel({ onRefresh }: { onRefresh: () => void }) {
  const {
    activeGroup,
    matchPanelOpen,
    setMatchPanelOpen,
    updateGroup: updateStoreGroup,
  } = useAppStore();
  const [manualQuery, setManualQuery] = useState("");
  const [searchResults, setSearchResults] = useState<MatchCandidate[]>([]);
  const [searching, setSearching] = useState(false);
  const [resolveJob, setResolveJob] = useState<JobWithPreview | null>(null);
  const [editing, setEditing] = useState(false);
  const [editFields, setEditFields] = useState({
    parsedTitle: "",
    parsedYear: "",
  });

  if (!matchPanelOpen) return null;

  if (!activeGroup) {
    return (
      <div className="w-[420px] border-l border-border bg-bg-secondary flex flex-col h-full overflow-hidden">
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold text-text-primary">
            Match Details
          </h2>
          <button
            onClick={() => setMatchPanelOpen(false)}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            &times;
          </button>
        </div>
        <div className="flex-1 flex items-center justify-center p-4">
          <p className="text-sm text-text-muted text-center">
            Select a group to view match details
          </p>
        </div>
      </div>
    );
  }

  const handleConfirmMatch = async (candidate: {
    tmdbId: number;
    title: string;
    year?: number | null;
    posterPath?: string | null;
    confidence: number;
    mediaType: string;
  }) => {
    const updates = {
      status: "confirmed" as const,
      tmdbId: candidate.tmdbId,
      tmdbTitle: candidate.title,
      tmdbYear: candidate.year,
      tmdbPosterPath: candidate.posterPath,
      matchConfidence: candidate.confidence,
      mediaType: candidate.mediaType as "movie" | "tv" | "unknown",
    };
    await updateGroup(activeGroup.id, updates);
    updateStoreGroup(activeGroup.id, updates);
    onRefresh();
  };

  const handleManualSearch = async () => {
    if (!manualQuery.trim()) return;
    setSearching(true);
    const data = await searchTmdb(
      manualQuery,
      activeGroup.mediaType !== "unknown" ? activeGroup.mediaType : undefined,
      activeGroup.parsedYear ?? undefined
    ) as { results?: MatchCandidate[] };
    setSearchResults(data.results || []);
    setSearching(false);
  };

  const startEditing = () => {
    setEditFields({
      parsedTitle: activeGroup.parsedTitle || "",
      parsedYear: activeGroup.parsedYear?.toString() || "",
    });
    setEditing(true);
  };

  const handleSaveEdit = async () => {
    const updates: Record<string, unknown> = {
      parsedTitle: editFields.parsedTitle || null,
      parsedYear: editFields.parsedYear
        ? parseInt(editFields.parsedYear, 10)
        : null,
    };
    await updateGroup(activeGroup.id, updates);
    updateStoreGroup(
      activeGroup.id,
      updates as Record<string, string | number | null>
    );
    setEditing(false);
    useToastStore.getState().addToast("Group updated", "success");
    onRefresh();
  };

  const handleSkip = async () => {
    await updateGroup(activeGroup.id, { status: "skipped" });
    updateStoreGroup(activeGroup.id, { status: "skipped" });
    setMatchPanelOpen(false);
    onRefresh();
  };

  const candidates = activeGroup.candidates || [];

  return (
      <div className="w-[420px] border-l border-border bg-bg-secondary flex flex-col h-full overflow-hidden">

          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border">
            <h2 className="text-sm font-semibold text-text-primary">
              Match Details
            </h2>
            <button
              onClick={() => setMatchPanelOpen(false)}
              className="text-text-muted hover:text-text-primary text-lg leading-none"
            >
              &times;
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-4">
            {/* Group info */}
            <div className="space-y-2">
              <p className="font-mono text-xs text-text-muted break-all">
                {activeGroup.folderName}
              </p>
              <div className="flex items-center gap-2">
                <StatusBadge status={activeGroup.status} />
                <span className="text-xs text-text-muted">
                  {activeGroup.totalFileCount} file
                  {activeGroup.totalFileCount !== 1 ? "s" : ""}
                </span>
                {!editing && (
                  <button
                    onClick={startEditing}
                    className="text-xs text-accent hover:text-accent-hover"
                  >
                    Edit
                  </button>
                )}
              </div>

              {editing ? (
                <div className="space-y-2 p-2 rounded bg-bg-tertiary/50">
                  <div>
                    <label className="text-[10px] uppercase tracking-wider text-text-muted">
                      Title
                    </label>
                    <input
                      value={editFields.parsedTitle}
                      onChange={(e) =>
                        setEditFields({
                          ...editFields,
                          parsedTitle: e.target.value,
                        })
                      }
                      className="w-full px-2 py-1 text-xs rounded bg-bg-tertiary border border-border text-text-primary"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] uppercase tracking-wider text-text-muted">
                      Year
                    </label>
                    <input
                      value={editFields.parsedYear}
                      onChange={(e) =>
                        setEditFields({
                          ...editFields,
                          parsedYear: e.target.value,
                        })
                      }
                      className="w-full px-2 py-1 text-xs rounded bg-bg-tertiary border border-border text-text-primary"
                    />
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={handleSaveEdit}
                      className="px-3 py-1 text-xs rounded bg-accent text-white hover:bg-accent-hover"
                    >
                      Save
                    </button>
                    <button
                      onClick={() => setEditing(false)}
                      className="px-3 py-1 text-xs rounded bg-bg-tertiary text-text-secondary hover:bg-bg-hover"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <div>
                  <p className="text-sm text-text-primary font-medium">
                    {activeGroup.tmdbTitle ||
                      activeGroup.parsedTitle ||
                      activeGroup.folderName}
                  </p>
                  <div className="flex gap-4 text-xs text-text-muted mt-1">
                    {(activeGroup.tmdbYear || activeGroup.parsedYear) && (
                      <span>
                        Year:{" "}
                        {activeGroup.tmdbYear || activeGroup.parsedYear}
                      </span>
                    )}
                    <span className="uppercase">{activeGroup.mediaType}</span>
                  </div>
                </div>
              )}
            </div>

            {/* Episode list */}
            {activeGroup.jobs.length > 0 && (
              <div className="space-y-1">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                  Files ({activeGroup.jobs.length})
                </h3>
                <div className="max-h-80 overflow-y-auto space-y-1">
                  {activeGroup.jobs.map((job) => {
                    const canResolve = !!activeGroup.tmdbId && activeGroup.mediaType === "tv";
                    return (
                    <div
                      key={job.id}
                      onClick={canResolve ? () => setResolveJob(job) : undefined}
                      className={`flex items-center gap-2 py-1 px-2 rounded bg-bg-tertiary/30 text-xs${
                        canResolve ? " cursor-pointer hover:bg-bg-hover/50 transition-colors" : ""
                      }`}
                    >
                      <FileCategoryBadge
                        category={job.fileCategory || "episode"}
                      />
                      {job.parsedSeason != null &&
                      job.parsedEpisode != null ? (
                        <span className="font-mono text-text-secondary w-12 flex-shrink-0">
                          S{String(job.parsedSeason).padStart(2, "0")}E
                          {String(job.parsedEpisode).padStart(2, "0")}
                        </span>
                      ) : null}
                      <span className="text-text-muted truncate flex-1">
                        {job.tmdbEpisodeTitle || job.fileName}
                      </span>
                    </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* TMDB Candidates */}
            {candidates.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                  Candidates
                </h3>
                {candidates.map((c) => (
                  <CandidateCard
                    key={c.id}
                    candidate={c}
                    onConfirm={() => handleConfirmMatch(c)}
                  />
                ))}
              </div>
            )}

            {/* Manual Search */}
            <div className="space-y-2">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                Manual Search
              </h3>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={manualQuery}
                  onChange={(e) => setManualQuery(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleManualSearch()}
                  placeholder="Search TMDB..."
                  className="flex-1 px-3 py-1.5 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                />
                <button
                  onClick={handleManualSearch}
                  disabled={searching}
                  className="px-3 py-1.5 text-sm rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors"
                >
                  {searching ? "..." : "Search"}
                </button>
              </div>
              {searchResults.map((r, i) => (
                <CandidateCard
                  key={`search-${i}`}
                  candidate={r}
                  onConfirm={() =>
                    handleConfirmMatch({
                      ...r,
                      confidence: 1.0,
                    })
                  }
                />
              ))}
            </div>
          </div>

          {/* Footer actions */}
          <div className="flex gap-2 p-4 border-t border-border">
            <button
              onClick={handleSkip}
              className="flex-1 px-3 py-2 text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover transition-colors"
            >
              Skip
            </button>
            {activeGroup.status === "matched" ||
            activeGroup.status === "ambiguous" ? (
              <button
                onClick={() =>
                  candidates[0] && handleConfirmMatch(candidates[0])
                }
                disabled={candidates.length === 0}
                className="flex-1 px-3 py-2 text-sm rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors"
              >
                Confirm Top Match
              </button>
            ) : null}
          </div>

          {/* Episode Resolve Modal */}
          {resolveJob && activeGroup.tmdbId && (
            <EpisodeResolveModal
              job={resolveJob}
              groupId={activeGroup.id}
              onClose={() => setResolveJob(null)}
              onSaved={onRefresh}
            />
          )}
      </div>
  );
}

function CandidateCard({
  candidate,
  onConfirm,
}: {
  candidate: MatchCandidate;
  onConfirm: () => void;
}) {
  return (
    <div className="flex gap-3 p-2 rounded-lg bg-bg-tertiary/50 hover:bg-bg-hover/50 transition-colors">
      {candidate.posterPath ? (
        <img
          src={`${TMDB_IMG_BASE}${candidate.posterPath}`}
          alt={candidate.title}
          className="w-12 h-18 rounded object-cover flex-shrink-0"
        />
      ) : (
        <div className="w-12 h-18 rounded bg-bg-tertiary flex items-center justify-center text-text-muted text-xs flex-shrink-0">
          ?
        </div>
      )}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-text-primary truncate">
          {candidate.title}
        </p>
        <div className="flex items-center gap-2 text-xs text-text-muted">
          {candidate.year && <span>{candidate.year}</span>}
          <span className="uppercase">{candidate.mediaType}</span>
          <span
            className={`font-mono ${
              candidate.confidence >= 0.85
                ? "text-success"
                : candidate.confidence >= 0.5
                ? "text-warning"
                : "text-error"
            }`}
          >
            {(candidate.confidence * 100).toFixed(0)}%
          </span>
        </div>
        {candidate.overview && (
          <p className="text-xs text-text-muted mt-1 line-clamp-2">
            {candidate.overview}
          </p>
        )}
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onConfirm();
        }}
        className="self-center px-2 py-1 text-xs rounded bg-accent text-white hover:bg-accent-hover transition-colors flex-shrink-0"
      >
        Use
      </button>
    </div>
  );
}
