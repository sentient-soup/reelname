import { useCallback, useEffect } from "react";
import { useAppStore } from "@/lib/store";
import { fetchGroups, fetchSettings, triggerScan, triggerMatch } from "@/lib/api";
import { Header } from "@/components/Header";
import { Filters } from "@/components/Filters";
import { QueueTable } from "@/components/QueueTable";
import { MatchPanel } from "@/components/MatchPanel";
import { SettingsModal } from "@/components/SettingsModal";
import { TransferDrawer } from "@/components/TransferDrawer";
import { Pagination } from "@/components/Pagination";
import { ToastContainer, useToastStore } from "@/components/Toast";
import { KeyboardShortcuts } from "@/components/KeyboardShortcuts";

export default function App() {
  const {
    setGroups,
    setLoading,
    statusFilter,
    mediaTypeFilter,
    searchQuery,
    sortBy,
    sortDir,
    page,
  } = useAppStore();

  const loadGroups = useCallback(async () => {
    setLoading(true);
    const params: Record<string, string> = {
      page: String(page),
      sortBy,
      sortDir,
    };
    if (statusFilter) params.status = statusFilter;
    if (mediaTypeFilter) params.mediaType = mediaTypeFilter;
    if (searchQuery) params.search = searchQuery;

    const data = (await fetchGroups(params)) as { groups: Parameters<typeof setGroups>[0]; total: number };
    setGroups(data.groups, data.total);
    setLoading(false);
  }, [page, sortBy, sortDir, statusFilter, mediaTypeFilter, searchQuery, setGroups, setLoading]);

  useEffect(() => {
    loadGroups();
    fetchSettings().then((s) => useAppStore.getState().setSettings(s as Record<string, string>));
  }, [loadGroups]);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => {
      loadGroups();
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, loadGroups]);

  const handleScan = useCallback(async () => {
    const { setScanning, settings } = useAppStore.getState();
    setScanning(true);
    try {
      const result = (await triggerScan(settings.scan_path || undefined)) as {
        error?: string;
        addedGroups?: number;
        addedFiles?: number;
        matched?: number;
        ambiguous?: number;
        matchError?: string;
      };
      if (result.error) {
        useToastStore.getState().addToast(result.error, "error");
      } else {
        let msg = `Added ${result.addedGroups ?? 0} groups (${result.addedFiles ?? 0} files).`;
        if ((result.matched ?? 0) > 0 || (result.ambiguous ?? 0) > 0) {
          msg += ` Matched ${result.matched}, ambiguous ${result.ambiguous}.`;
        }
        if (result.matchError) {
          msg += ` ${result.matchError}`;
          useToastStore.getState().addToast(msg, "warning");
        } else {
          useToastStore.getState().addToast(msg, "success");
        }
      }
      await loadGroups();
    } catch {
      useToastStore.getState().addToast("Scan failed", "error");
    }
    setScanning(false);
  }, [loadGroups]);

  const handleMatch = useCallback(async () => {
    const { setScanning } = useAppStore.getState();
    setScanning(true);
    try {
      const result = (await triggerMatch()) as {
        error?: string;
        matched?: number;
        ambiguous?: number;
      };
      if (result.error) {
        useToastStore.getState().addToast(result.error, "error");
      } else {
        useToastStore
          .getState()
          .addToast(
            `Matched ${result.matched ?? 0} groups, ${result.ambiguous ?? 0} ambiguous.`,
            "success"
          );
      }
      await loadGroups();
    } catch {
      useToastStore.getState().addToast("Matching failed", "error");
    }
    setScanning(false);
  }, [loadGroups]);

  return (
    <div className="h-screen flex flex-col">
      <KeyboardShortcuts onRefresh={loadGroups} onScan={handleScan} />
      <Header onScan={handleScan} onMatch={handleMatch} />
      <Filters onRefresh={loadGroups} />
      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <QueueTable onRefresh={loadGroups} />
          <Pagination onRefresh={loadGroups} />
        </div>
        <MatchPanel onRefresh={loadGroups} />
      </div>
      <TransferDrawer onRefresh={loadGroups} />
      <SettingsModal />
      <ToastContainer />
    </div>
  );
}
