"use client";

import { useState, useEffect, useCallback } from "react";
import { fetchSeasons, fetchSeasonEpisodes, updateJob } from "@/lib/api";
import { useToastStore } from "./Toast";
import type { JobWithPreview } from "@/lib/store";

interface TmdbSeason {
  season_number: number;
  name: string;
  episode_count: number;
}

interface TmdbEpisode {
  episode_number: number;
  name: string;
  air_date: string | null;
  overview: string;
}

interface Props {
  job: JobWithPreview;
  groupId: number;
  onClose: () => void;
  onSaved: () => void;
}

export function EpisodeResolveModal({ job, groupId, onClose, onSaved }: Props) {
  const [seasons, setSeasons] = useState<TmdbSeason[]>([]);
  const [episodes, setEpisodes] = useState<TmdbEpisode[]>([]);
  const [selectedSeason, setSelectedSeason] = useState<number | null>(null);
  const [loadingSeasons, setLoadingSeasons] = useState(true);
  const [loadingEpisodes, setLoadingEpisodes] = useState(false);
  const [saving, setSaving] = useState<number | null>(null);

  // Load seasons on mount
  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoadingSeasons(true);
      const data = await fetchSeasons(groupId);
      if (cancelled) return;
      setSeasons(data.seasons || []);
      setLoadingSeasons(false);

      // Default to the job's current season, or Season 0 for specials
      const defaultSeason =
        job.fileCategory === "special" ? 0 : (job.parsedSeason ?? 1);
      const available = (data.seasons || []) as TmdbSeason[];
      const match = available.find((s) => s.season_number === defaultSeason);
      setSelectedSeason(match ? defaultSeason : available[0]?.season_number ?? null);
    })();
    return () => { cancelled = true; };
  }, [groupId, job.parsedSeason, job.fileCategory]);

  // Load episodes when season changes
  useEffect(() => {
    if (selectedSeason == null) return;
    let cancelled = false;
    (async () => {
      setLoadingEpisodes(true);
      const data = await fetchSeasonEpisodes(groupId, selectedSeason);
      if (cancelled) return;
      setEpisodes(data.episodes || []);
      setLoadingEpisodes(false);
    })();
    return () => { cancelled = true; };
  }, [groupId, selectedSeason]);

  const handleUse = useCallback(
    async (ep: TmdbEpisode) => {
      if (selectedSeason == null) return;
      setSaving(ep.episode_number);

      const updates: Record<string, unknown> = {
        parsedSeason: selectedSeason,
        parsedEpisode: ep.episode_number,
        tmdbEpisodeTitle: ep.name,
      };

      // Season 0 → special; moving out of season 0 → episode
      if (selectedSeason === 0) {
        updates.fileCategory = "special";
      } else if (job.fileCategory === "special") {
        updates.fileCategory = "episode";
      }

      await updateJob(job.id, updates);
      useToastStore
        .getState()
        .addToast(`Resolved → S${String(selectedSeason).padStart(2, "0")}E${String(ep.episode_number).padStart(2, "0")} ${ep.name}`, "success");
      setSaving(null);
      onSaved();
      onClose();
    },
    [selectedSeason, job.id, job.fileCategory, onSaved, onClose]
  );

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={onClose}
    >
      <div
        className="bg-bg-secondary border border-border rounded-lg shadow-xl w-[520px] max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold text-text-primary">
            Resolve Episode
          </h2>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            &times;
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* Current identification */}
          <div className="p-3 rounded-md bg-bg-tertiary/50 space-y-1">
            <p className="text-[10px] uppercase tracking-wider text-text-muted">
              Current Identification
            </p>
            <p className="text-xs text-text-muted font-mono truncate">
              {job.fileName}
            </p>
            <div className="flex items-center gap-2 text-sm text-text-primary">
              {job.parsedSeason != null && job.parsedEpisode != null ? (
                <span className="font-mono font-medium">
                  S{String(job.parsedSeason).padStart(2, "0")}E
                  {String(job.parsedEpisode).padStart(2, "0")}
                </span>
              ) : (
                <span className="text-text-muted italic">No episode info</span>
              )}
              {job.tmdbEpisodeTitle && (
                <span className="text-text-secondary truncate">
                  {job.tmdbEpisodeTitle}
                </span>
              )}
            </div>
          </div>

          {/* Season selector */}
          <div className="space-y-1">
            <label className="text-[10px] uppercase tracking-wider text-text-muted">
              Season
            </label>
            {loadingSeasons ? (
              <p className="text-xs text-text-muted">Loading seasons...</p>
            ) : (
              <select
                value={selectedSeason ?? ""}
                onChange={(e) => setSelectedSeason(parseInt(e.target.value, 10))}
                className="w-full px-3 py-1.5 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
              >
                {seasons.map((s) => (
                  <option key={s.season_number} value={s.season_number}>
                    {s.name} ({s.episode_count} episodes)
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* Episodes list */}
          <div className="space-y-1">
            <p className="text-[10px] uppercase tracking-wider text-text-muted">
              Episodes
            </p>
            {loadingEpisodes ? (
              <p className="text-xs text-text-muted">Loading episodes...</p>
            ) : episodes.length === 0 ? (
              <p className="text-xs text-text-muted italic">
                No episodes in this season
              </p>
            ) : (
              <div className="space-y-1 max-h-[40vh] overflow-y-auto">
                {episodes.map((ep) => {
                  const isCurrent =
                    selectedSeason === job.parsedSeason &&
                    ep.episode_number === job.parsedEpisode;
                  return (
                    <div
                      key={ep.episode_number}
                      className={`flex items-start gap-3 p-2 rounded-md text-xs transition-colors ${
                        isCurrent
                          ? "bg-accent/10 border border-accent/30"
                          : "bg-bg-tertiary/30 hover:bg-bg-hover/50"
                      }`}
                    >
                      <span className="font-mono text-text-secondary w-6 flex-shrink-0 pt-0.5">
                        {String(ep.episode_number).padStart(2, "0")}
                      </span>
                      <div className="flex-1 min-w-0">
                        <p className="text-text-primary font-medium truncate">
                          {ep.name}
                        </p>
                        {ep.air_date && (
                          <p className="text-text-muted text-[10px]">
                            {ep.air_date}
                          </p>
                        )}
                        {ep.overview && (
                          <p className="text-text-muted line-clamp-2 mt-0.5">
                            {ep.overview}
                          </p>
                        )}
                      </div>
                      <button
                        onClick={() => handleUse(ep)}
                        disabled={saving != null}
                        className={`self-center px-2 py-1 rounded text-xs flex-shrink-0 transition-colors ${
                          isCurrent
                            ? "bg-accent/20 text-accent cursor-default"
                            : "bg-accent text-white hover:bg-accent-hover"
                        } disabled:opacity-50`}
                      >
                        {saving === ep.episode_number
                          ? "..."
                          : isCurrent
                          ? "Current"
                          : "Use"}
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
