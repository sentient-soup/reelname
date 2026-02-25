"use client";

import { useEffect } from "react";
import { useAppStore } from "@/lib/store";
import { fetchGroup } from "@/lib/api";

export function KeyboardShortcuts({
  onRefresh,
  onScan,
}: {
  onRefresh: () => void;
  onScan: () => void;
}) {
  const {
    selectedGroupIds,
    groups,
    activeGroupId,
    setActiveGroup,
    selectAllGroups,
    clearSelection,
    setSettingsOpen,
    matchPanelOpen,
    setMatchPanelOpen,
  } = useAppStore();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement ||
        e.target instanceof HTMLSelectElement
      ) {
        return;
      }

      // Escape - close panels
      if (e.key === "Escape") {
        if (matchPanelOpen) {
          setMatchPanelOpen(false);
          return;
        }
      }

      // Ctrl+A - select all
      if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        selectAllGroups();
        return;
      }

      // Ctrl+D - deselect all
      if (e.key === "d" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        clearSelection();
        return;
      }

      // R - refresh
      if (e.key === "r" && !e.ctrlKey && !e.metaKey) {
        onRefresh();
        return;
      }

      // S - scan
      if (e.key === "s" && !e.ctrlKey && !e.metaKey) {
        onScan();
        return;
      }

      // , - settings
      if (e.key === ",") {
        setSettingsOpen(true);
        return;
      }

      // Arrow up/down - navigate groups
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const currentIndex = groups.findIndex((g) => g.id === activeGroupId);
        let newIndex: number;
        if (e.key === "ArrowDown") {
          newIndex = currentIndex < groups.length - 1 ? currentIndex + 1 : 0;
        } else {
          newIndex = currentIndex > 0 ? currentIndex - 1 : groups.length - 1;
        }
        if (groups[newIndex]) {
          fetchGroup(groups[newIndex].id).then((data) => setActiveGroup(data));
        }
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    groups,
    activeGroupId,
    matchPanelOpen,
    selectedGroupIds,
    selectAllGroups,
    clearSelection,
    setActiveGroup,
    setMatchPanelOpen,
    setSettingsOpen,
    onRefresh,
    onScan,
  ]);

  return null;
}
