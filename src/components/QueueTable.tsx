"use client";

import { useAppStore, type GroupWithJobs, type JobWithPreview } from "@/lib/store";
import { fetchGroup } from "@/lib/api";
import { StatusBadge, MediaTypeBadge, FileCategoryBadge } from "./StatusBadge";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function QueueTable({ onRefresh }: { onRefresh: () => void }) {
  const {
    groups,
    loading,
    selectedGroupIds,
    toggleGroupSelection,
    selectAllGroups,
    clearSelection,
    activeGroupId,
    setActiveGroup,
    sortBy,
    sortDir,
    setSorting,
  } = useAppStore();

  const handleGroupClick = async (group: GroupWithJobs) => {
    if (activeGroupId === group.id) {
      setActiveGroup(null);
      return;
    }
    const data = await fetchGroup(group.id);
    setActiveGroup(data);
  };

  const handleSort = (column: string) => {
    if (sortBy === column) {
      setSorting(column, sortDir === "asc" ? "desc" : "asc");
    } else {
      setSorting(column, "asc");
    }
    onRefresh();
  };

  const SortIndicator = ({ column }: { column: string }) => {
    if (sortBy !== column) return null;
    return (
      <span className="ml-1 text-accent">
        {sortDir === "asc" ? "\u25B2" : "\u25BC"}
      </span>
    );
  };

  const allSelected =
    groups.length > 0 && groups.every((g) => !!selectedGroupIds[g.id]);

  return (
    <div className="flex-1 overflow-auto">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-bg-secondary z-10">
          <tr className="border-b border-border text-left text-text-muted text-xs uppercase tracking-wider">
            <th className="px-4 py-3 w-8">
              <input
                type="checkbox"
                checked={allSelected}
                onChange={() =>
                  allSelected ? clearSelection() : selectAllGroups()
                }
                className="accent-accent"
              />
            </th>
            <th className="px-4 py-3 w-16">Type</th>
            <th
              className="px-4 py-3 cursor-pointer hover:text-text-primary"
              onClick={() => handleSort("folderName")}
            >
              Title <SortIndicator column="folderName" />
            </th>
            <th
              className="px-4 py-3 w-24 cursor-pointer hover:text-text-primary"
              onClick={() => handleSort("totalFileSize")}
            >
              Size <SortIndicator column="totalFileSize" />
            </th>
            <th
              className="px-4 py-3 w-28 cursor-pointer hover:text-text-primary"
              onClick={() => handleSort("status")}
            >
              Status <SortIndicator column="status" />
            </th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td
                colSpan={5}
                className="px-4 py-12 text-center text-text-muted"
              >
                <span className="animate-spin inline-block w-5 h-5 border-2 border-text-muted/30 border-t-text-muted rounded-full mr-2" />
                Loading...
              </td>
            </tr>
          ) : groups.length === 0 ? (
            <tr>
              <td
                colSpan={5}
                className="px-4 py-12 text-center text-text-muted"
              >
                No groups found. Configure a scan path in Settings and click
                Scan.
              </td>
            </tr>
          ) : (
            groups.map((group) => (
                <GroupRow
                  key={group.id}
                  group={group}
                  isExpanded={activeGroupId === group.id}
                  isSelected={!!selectedGroupIds[group.id]}
                  isActive={activeGroupId === group.id}
                  onToggleSelect={() => toggleGroupSelection(group.id)}
                  onClick={() => handleGroupClick(group)}
                />
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

function GroupRow({
  group,
  isExpanded,
  isSelected,
  isActive,
  onToggleSelect,
  onClick,
}: {
  group: GroupWithJobs;
  isExpanded: boolean;
  isSelected: boolean;
  isActive: boolean;
  onToggleSelect: () => void;
  onClick: () => void;
}) {
  const year = group.tmdbYear || group.parsedYear;

  return (
    <>
      {/* Group header row */}
      <tr
        className={`border-b border-border/50 cursor-pointer transition-colors ${
          isActive
            ? "bg-accent/10"
            : isSelected
            ? "bg-bg-hover/50"
            : "hover:bg-bg-hover/30"
        }`}
        onClick={onClick}
      >
        <td className="px-4 py-2.5" onClick={(e) => e.stopPropagation()}>
          <input
            type="checkbox"
            checked={isSelected}
            onChange={onToggleSelect}
            className="accent-accent"
          />
        </td>
        <td className="px-4 py-2.5">
          <MediaTypeBadge type={group.mediaType} />
        </td>
        <td className="px-4 py-2.5 max-w-0">
          <div className="flex items-baseline gap-2">
            <span className="font-medium text-text-primary truncate">
              {group.tmdbTitle || group.parsedTitle || group.folderName}
            </span>
            {year && (
              <span className="text-xs text-text-muted">({year})</span>
            )}
            <span className="text-xs text-text-muted">
              {group.totalFileCount} {group.totalFileCount === 1 ? "file" : "files"}
            </span>
          </div>
          {group.tmdbTitle && group.folderName !== group.tmdbTitle && (
            <div className="text-[11px] text-text-muted font-mono truncate">
              {group.folderName}
            </div>
          )}
        </td>
        <td className="px-4 py-2.5 text-text-secondary text-xs">
          {formatSize(group.totalFileSize)}
        </td>
        <td className="px-4 py-2.5">
          <StatusBadge status={group.status} />
        </td>
      </tr>

      {/* Expanded file rows */}
      {isExpanded &&
        group.jobs.map((job) => (
          <FileRow key={job.id} job={job} />
        ))}
    </>
  );
}

function FileRow({ job }: { job: JobWithPreview }) {
  const seLabel =
    job.parsedSeason != null && job.parsedEpisode != null
      ? `S${String(job.parsedSeason).padStart(2, "0")}E${String(
          job.parsedEpisode
        ).padStart(2, "0")}`
      : null;

  return (
    <tr className="border-b border-border/20 bg-bg-primary/30">
      <td className="px-4 py-1.5" />
      <td className="px-4 py-1.5">
        <FileCategoryBadge category={job.fileCategory || "episode"} />
      </td>
      <td className="px-4 py-1.5 max-w-0">
        <div className="flex items-center gap-2">
          {seLabel && (
            <span className="font-mono text-xs text-text-muted w-14 flex-shrink-0">
              {seLabel}
            </span>
          )}
          <div className="min-w-0">
            <span className="font-mono text-xs text-text-secondary truncate block">
              {job.fileName}
            </span>
            {job.tmdbEpisodeTitle && (
              <span className="text-[11px] text-text-muted block">
                {job.tmdbEpisodeTitle}
              </span>
            )}
            {job.previewName && (
              <span
                className="font-mono text-[11px] text-accent/70 truncate block"
                title={job.previewName}
              >
                &rarr; {job.previewName}
              </span>
            )}
          </div>
        </div>
      </td>
      <td className="px-4 py-1.5 text-text-muted text-xs">
        {formatSize(job.fileSize)}
      </td>
      <td className="px-4 py-1.5">
        <StatusBadge status={job.status} />
      </td>
    </tr>
  );
}
