"use client";

import { useAppStore } from "@/lib/store";
import { motion, AnimatePresence } from "framer-motion";
import { useState, useEffect, useRef, useCallback } from "react";
import {
  fetchDestinations,
  createDestination,
  deleteDestination,
  startTransfer,
  testSshConnection,
  fetchTransferStatus,
} from "@/lib/api";

interface TransferJob {
  id: number;
  status: string;
  fileName: string;
  fileSize: number;
  transferProgress: number | null;
  transferError: string | null;
  destinationPath: string | null;
}

function useIsMobile(breakpoint = 768) {
  const [mobile, setMobile] = useState(false);
  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${breakpoint - 1}px)`);
    setMobile(mql.matches);
    const handler = (e: MediaQueryListEvent) => setMobile(e.matches);
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [breakpoint]);
  return mobile;
}

function IconPencil() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
      <path d="m15 5 4 4" />
    </svg>
  );
}

function IconTransfer() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
      <path d="M12 10v6" />
      <path d="m15 13-3 3-3-3" />
    </svg>
  );
}

function IconTrash() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 6h18" />
      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatRate(bytesPerSec: number): string {
  if (bytesPerSec < 1024 * 1024)
    return `${(bytesPerSec / 1024).toFixed(0)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

export function TransferDrawer({ onRefresh }: { onRefresh: () => void }) {
  const isMobile = useIsMobile();
  const {
    transferDrawerOpen,
    setTransferDrawerOpen,
    selectedGroupIds,
    destinations,
    setDestinations,
    groups,
  } = useAppStore();

  const [selectedDest, setSelectedDest] = useState<number | null>(null);
  const [showAddDest, setShowAddDest] = useState(false);
  const [destForm, setDestForm] = useState({
    name: "",
    type: "local" as "local" | "ssh",
    basePath: "",
    sshHost: "",
    sshPort: "22",
    sshUser: "",
    sshKeyPath: "",
    sshKeyPassphrase: "",
  });
  const [testingConnection, setTestingConnection] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    error?: string;
  } | null>(null);
  const [transferring, setTransferring] = useState(false);
  const [activeTransfers, setActiveTransfers] = useState<TransferJob[]>([]);
  const [transferRates, setTransferRates] = useState<Record<number, number>>(
    {}
  );
  const prevProgress = useRef<Record<number, { progress: number; time: number }>>({});
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (transferDrawerOpen) {
      fetchDestinations().then(setDestinations);
    }
  }, [transferDrawerOpen, setDestinations]);

  // Connect to SSE when transferring
  const startProgressStream = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    const es = new EventSource("/api/transfer/progress");
    eventSourceRef.current = es;

    es.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.done) {
        es.close();
        eventSourceRef.current = null;
        setTransferring(false);
        onRefresh();
        return;
      }

      const transferJobs = data as TransferJob[];
      setActiveTransfers(transferJobs);

      // Calculate transfer rates
      const now = Date.now();
      const newRates: Record<number, number> = {};
      for (const job of transferJobs) {
        const prog = job.transferProgress ?? 0;
        const transferred = prog * job.fileSize;
        const prev = prevProgress.current[job.id];
        if (prev && now - prev.time > 0) {
          const prevTransferred = prev.progress * job.fileSize;
          const elapsed = (now - prev.time) / 1000;
          if (elapsed > 0) {
            newRates[job.id] = (transferred - prevTransferred) / elapsed;
          }
        }
        prevProgress.current[job.id] = { progress: prog, time: now };
      }
      setTransferRates((prev) => ({ ...prev, ...newRates }));

      // Check if all done (no queued or actively transferring)
      const hasActive = transferJobs.some(
        (j) => j.status === "transferring" || j.status === "queued"
      );
      if (!hasActive && transferJobs.length > 0) {
        es.close();
        eventSourceRef.current = null;
        setTransferring(false);
        onRefresh();
      }
    };

    es.onerror = () => {
      es.close();
      eventSourceRef.current = null;
      // Reconnect: check if transfers are still active
      setTimeout(async () => {
        try {
          const status = await fetchTransferStatus();
          if (status.active) {
            setActiveTransfers(status.jobs);
            setTransferring(true);
            startProgressStream();
          } else if (status.jobs.length > 0) {
            setActiveTransfers(status.jobs);
            setTransferring(false);
            onRefresh();
          }
        } catch {
          // Server unreachable, give up
        }
      }, 2000);
    };
  }, [onRefresh]);

  // Mount initialization: recover in-flight transfers
  useEffect(() => {
    fetchTransferStatus().then((status) => {
      if (status.active) {
        setActiveTransfers(status.jobs);
        setTransferring(true);
        setTransferDrawerOpen(true);
        startProgressStream();
      } else if (status.jobs.length > 0) {
        // Completed/failed from a previous run, show them
        setActiveTransfers(status.jobs);
      }
    }).catch(() => {
      // Ignore fetch errors on mount
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, []);

  const handleAddDestination = async () => {
    const data: Record<string, unknown> = {
      name: destForm.name,
      type: destForm.type,
      basePath: destForm.basePath,
    };
    if (destForm.type === "ssh") {
      data.sshHost = destForm.sshHost;
      data.sshPort = parseInt(destForm.sshPort, 10);
      data.sshUser = destForm.sshUser;
      data.sshKeyPath = destForm.sshKeyPath;
      if (destForm.sshKeyPassphrase) {
        data.sshKeyPassphrase = destForm.sshKeyPassphrase;
      }
    }
    await createDestination(data);
    const dests = await fetchDestinations();
    setDestinations(dests);
    setShowAddDest(false);
    setTestResult(null);
    setDestForm({
      name: "",
      type: "local",
      basePath: "",
      sshHost: "",
      sshPort: "22",
      sshUser: "",
      sshKeyPath: "",
      sshKeyPassphrase: "",
    });
  };

  const handleDeleteDest = async (id: number) => {
    await deleteDestination(id);
    const dests = await fetchDestinations();
    setDestinations(dests);
    if (selectedDest === id) setSelectedDest(null);
  };

  const handleTestConnection = async () => {
    setTestingConnection(true);
    setTestResult(null);
    const result = await testSshConnection({
      sshHost: destForm.sshHost,
      sshPort: parseInt(destForm.sshPort, 10),
      sshUser: destForm.sshUser,
      sshKeyPath: destForm.sshKeyPath,
      sshKeyPassphrase: destForm.sshKeyPassphrase || undefined,
      basePath: destForm.basePath,
    });
    setTestResult(result);
    setTestingConnection(false);
  };

  const handleTransferTo = async (destId: number) => {
    const ids = Object.keys(selectedGroupIds).map(Number);
    if (ids.length === 0) return;
    setTransferring(true);
    setActiveTransfers([]);
    prevProgress.current = {};
    setTransferRates({});
    await startTransfer({ groupIds: ids }, destId);
    startProgressStream();
  };

  const clearTransferHistory = () => {
    setActiveTransfers([]);
    prevProgress.current = {};
    setTransferRates({});
  };

  // Count confirmed groups
  const confirmedSelected = Object.keys(selectedGroupIds)
    .map(Number)
    .filter((id) => {
      const group = groups.find((g) => g.id === id);
      return group?.status === "confirmed";
    });

  const totalFiles = confirmedSelected.reduce((sum, id) => {
    const group = groups.find((g) => g.id === id);
    return sum + (group?.totalFileCount || 0);
  }, 0);

  // Aggregate transfer stats
  const totalTransferSize = activeTransfers.reduce(
    (s, j) => s + j.fileSize,
    0
  );
  const totalTransferred = activeTransfers.reduce(
    (s, j) => s + (j.transferProgress ?? 0) * j.fileSize,
    0
  );
  const overallProgress =
    totalTransferSize > 0 ? totalTransferred / totalTransferSize : 0;
  const completedCount = activeTransfers.filter(
    (j) => j.status === "completed"
  ).length;
  const failedCount = activeTransfers.filter(
    (j) => j.status === "failed"
  ).length;
  const activeCount = activeTransfers.filter(
    (j) => j.status === "transferring"
  ).length;
  const queuedCount = activeTransfers.filter(
    (j) => j.status === "queued"
  ).length;

  // Mobile: fullscreen overlay. Desktop: bottom drawer with fixed height.
  const drawerStyle: React.CSSProperties = isMobile
    ? { position: "fixed", inset: 0, zIndex: 50 }
    : { position: "relative", height: 380, borderTop: "1px solid var(--color-border)" };

  return (
    <>
    <AnimatePresence>
      {transferDrawerOpen && (
        <motion.div
          key="transfer-drawer"
          initial={{ y: "100%" }}
          animate={{ y: 0 }}
          exit={{ y: "100%" }}
          transition={{ type: "spring", damping: 25, stiffness: 300 }}
          className="flex flex-col bg-bg-secondary"
          style={drawerStyle}
        >
          <div className="flex items-center justify-between px-3 md:px-6 py-2 border-b border-border">
            <h2 className="text-sm font-semibold text-text-primary">
              Transfer{" "}
              {transferring || activeTransfers.length > 0 ? (
                <span className="text-text-muted font-normal">
                  {completedCount}/{activeTransfers.length} complete
                  {failedCount > 0 && (
                    <span className="text-error ml-1">
                      ({failedCount} failed)
                    </span>
                  )}
                </span>
              ) : (
                <span className="text-text-muted font-normal">
                  ({confirmedSelected.length} confirmed group
                  {confirmedSelected.length !== 1 ? "s" : ""}, {totalFiles} file
                  {totalFiles !== 1 ? "s" : ""})
                </span>
              )}
            </h2>
            <button
              onClick={() => setTransferDrawerOpen(false)}
              className="text-text-muted hover:text-text-primary"
            >
              &times;
            </button>
          </div>

          <div className={`flex ${isMobile ? "flex-col" : "flex-row"} flex-1 overflow-hidden`}>
            {/* Destinations list */}
            <div className={`${isMobile ? "w-full border-b" : "w-80 border-r"} border-border p-3 overflow-y-auto flex-shrink-0`}>
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                  Destinations
                </h3>
                <button
                  onClick={() => setShowAddDest(true)}
                  className="text-xs text-accent hover:text-accent-hover"
                >
                  + Add
                </button>
              </div>

              <div className="space-y-2">
                {destinations.map((d) => (
                  <div
                    key={d.id}
                    className="p-3 rounded-lg bg-bg-tertiary/50 hover:bg-bg-tertiary transition-colors"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <p className="text-sm font-medium text-text-primary truncate">
                            {d.name}
                          </p>
                          <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider ${
                            d.type === "ssh"
                              ? "bg-accent/20 text-accent"
                              : "bg-info/20 text-info"
                          }`}>
                            {d.type === "ssh" ? "SSH" : "Local"}
                          </span>
                        </div>
                        {d.type === "ssh" && d.sshHost && (
                          <p className="text-xs text-text-muted truncate">
                            {d.sshUser ? `${d.sshUser}@` : ""}{d.sshHost}{d.sshPort && d.sshPort !== 22 ? `:${d.sshPort}` : ""}
                          </p>
                        )}
                        <p className="text-xs text-text-muted truncate font-mono">
                          {d.basePath}
                        </p>
                      </div>
                      <div className="flex items-center gap-1 flex-shrink-0">
                        {confirmedSelected.length > 0 && !transferring && (
                          <button
                            onClick={() => {
                              setSelectedDest(d.id);
                              handleTransferTo(d.id);
                            }}
                            title={`Transfer ${totalFiles} file${totalFiles !== 1 ? "s" : ""} to ${d.name}`}
                            className="p-1.5 rounded-md text-accent hover:bg-accent/20 transition-colors"
                          >
                            <IconTransfer />
                          </button>
                        )}
                        <button
                          onClick={() => setShowAddDest(true)}
                          title="Edit destination"
                          className="p-1.5 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors"
                        >
                          <IconPencil />
                        </button>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteDest(d.id);
                          }}
                          title="Delete destination"
                          className="p-1.5 rounded-md text-text-muted hover:text-error hover:bg-error/10 transition-colors"
                        >
                          <IconTrash />
                        </button>
                      </div>
                    </div>
                  </div>
                ))}

                {destinations.length === 0 && (
                  <p className="text-xs text-text-muted text-center py-4">
                    No destinations configured.
                  </p>
                )}
              </div>
            </div>

            {/* Transfer progress area */}
            <div className="flex-1 flex flex-col overflow-hidden">
              {activeTransfers.length > 0 ? (
                <TransferProgress
                  jobs={activeTransfers}
                  rates={transferRates}
                  overallProgress={overallProgress}
                  totalSize={totalTransferSize}
                  totalTransferred={totalTransferred}
                  completedCount={completedCount}
                  activeCount={activeCount}
                  queuedCount={queuedCount}
                  failedCount={failedCount}
                  transferring={transferring}
                  onClear={clearTransferHistory}
                />
              ) : (
                <div className="flex-1 flex items-center justify-center">
                  <p className="text-text-muted text-sm text-center px-4">
                    {confirmedSelected.length > 0
                      ? `${confirmedSelected.length} confirmed group${confirmedSelected.length !== 1 ? "s" : ""} (${totalFiles} file${totalFiles !== 1 ? "s" : ""}) ready. Click "Transfer" on a destination.`
                      : "Select confirmed groups in the queue, then click \"Transfer\" on a destination."}
                  </p>
                </div>
              )}
            </div>
          </div>
        </motion.div>
      )}

    </AnimatePresence>

      {/* Floating indicator when drawer is closed but transfers are active */}
      {!transferDrawerOpen && transferring && (activeCount > 0 || queuedCount > 0) && (
        <button
          onClick={() => setTransferDrawerOpen(true)}
          className="fixed bottom-4 right-4 z-40 flex items-center gap-2 px-3 py-2 rounded-lg bg-bg-secondary border border-border shadow-lg hover:bg-bg-hover transition-colors"
        >
          <span className="animate-spin inline-block w-3.5 h-3.5 border-2 border-accent/30 border-t-accent rounded-full" />
          <span className="text-xs text-text-primary font-medium">
            {activeCount + queuedCount} transferring
          </span>
          <span className="text-xs text-text-muted">
            {(overallProgress * 100).toFixed(0)}%
          </span>
        </button>
      )}

      {/* Add Destination Modal */}
      {showAddDest && (
        <AddDestinationModal
          destForm={destForm}
          setDestForm={setDestForm}
          testingConnection={testingConnection}
          testResult={testResult}
          onTestConnection={handleTestConnection}
          onSetTestResult={setTestResult}
          onSave={handleAddDestination}
          onClose={() => {
            setShowAddDest(false);
            setTestResult(null);
          }}
        />
      )}
    </>
  );
}

const INPUT_CLASS =
  "w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent";

function AddDestinationModal({
  destForm,
  setDestForm,
  testingConnection,
  testResult,
  onTestConnection,
  onSetTestResult,
  onSave,
  onClose,
}: {
  destForm: {
    name: string;
    type: "local" | "ssh";
    basePath: string;
    sshHost: string;
    sshPort: string;
    sshUser: string;
    sshKeyPath: string;
    sshKeyPassphrase: string;
  };
  setDestForm: (form: typeof destForm) => void;
  testingConnection: boolean;
  testResult: { ok: boolean; error?: string } | null;
  onTestConnection: () => void;
  onSetTestResult: (r: null) => void;
  onSave: () => void;
  onClose: () => void;
}) {
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
        className="bg-bg-secondary border border-border rounded-lg shadow-xl w-[calc(100%-32px)] max-w-[440px] max-h-[90vh] overflow-y-auto flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-border">
          <h2 className="text-sm font-semibold text-text-primary">
            Add Destination
          </h2>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            &times;
          </button>
        </div>

        {/* Form */}
        <div className="p-5 space-y-3">
          <div>
            <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
              Name
            </label>
            <input
              placeholder="e.g. Media Server"
              value={destForm.name}
              onChange={(e) =>
                setDestForm({ ...destForm, name: e.target.value })
              }
              className={INPUT_CLASS}
            />
          </div>

          <div>
            <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
              Type
            </label>
            <select
              value={destForm.type}
              onChange={(e) => {
                onSetTestResult(null);
                setDestForm({
                  ...destForm,
                  type: e.target.value as "local" | "ssh",
                });
              }}
              className={INPUT_CLASS}
            >
              <option value="local">Local</option>
              <option value="ssh">SSH/SFTP</option>
            </select>
          </div>

          <div>
            <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
              Base Path
            </label>
            <input
              placeholder={
                destForm.type === "ssh"
                  ? "/mnt/media/movies"
                  : "D:\\Media\\Movies"
              }
              value={destForm.basePath}
              onChange={(e) =>
                setDestForm({ ...destForm, basePath: e.target.value })
              }
              className={INPUT_CLASS}
            />
          </div>

          {destForm.type === "ssh" && (
            <>
              <div className="flex gap-3">
                <div className="flex-1">
                  <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
                    Host
                  </label>
                  <input
                    placeholder="192.168.1.100"
                    value={destForm.sshHost}
                    onChange={(e) =>
                      setDestForm({ ...destForm, sshHost: e.target.value })
                    }
                    className={INPUT_CLASS}
                  />
                </div>
                <div className="w-20">
                  <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
                    Port
                  </label>
                  <input
                    placeholder="22"
                    value={destForm.sshPort}
                    onChange={(e) =>
                      setDestForm({ ...destForm, sshPort: e.target.value })
                    }
                    className={INPUT_CLASS}
                  />
                </div>
              </div>

              <div>
                <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
                  Username
                </label>
                <input
                  placeholder="root"
                  value={destForm.sshUser}
                  onChange={(e) =>
                    setDestForm({ ...destForm, sshUser: e.target.value })
                  }
                  className={INPUT_CLASS}
                />
              </div>

              <div>
                <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
                  SSH Key Path
                </label>
                <input
                  placeholder="~/.ssh/id_rsa"
                  value={destForm.sshKeyPath}
                  onChange={(e) =>
                    setDestForm({ ...destForm, sshKeyPath: e.target.value })
                  }
                  className={INPUT_CLASS}
                />
              </div>

              <div>
                <label className="text-[10px] uppercase tracking-wider text-text-muted mb-1 block">
                  Key Passphrase{" "}
                  <span className="normal-case tracking-normal text-text-muted/60">
                    (optional)
                  </span>
                </label>
                <input
                  type="password"
                  placeholder="Leave blank if none"
                  value={destForm.sshKeyPassphrase}
                  onChange={(e) =>
                    setDestForm({
                      ...destForm,
                      sshKeyPassphrase: e.target.value,
                    })
                  }
                  className={INPUT_CLASS}
                />
              </div>

              <button
                onClick={onTestConnection}
                disabled={
                  testingConnection || !destForm.sshHost || !destForm.sshUser
                }
                className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover disabled:opacity-50 transition-colors"
              >
                {testingConnection ? "Testing..." : "Test Connection"}
              </button>
              {testResult && (
                <p
                  className={`text-xs ${
                    testResult.ok ? "text-success" : "text-error"
                  }`}
                >
                  {testResult.ok
                    ? "Connection successful"
                    : testResult.error || "Connection failed"}
                </p>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex gap-2 px-5 py-3 border-t border-border">
          <button
            onClick={onClose}
            className="flex-1 px-3 py-2 text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={onSave}
            disabled={!destForm.name || !destForm.basePath}
            className="flex-1 px-3 py-2 text-sm rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium"
          >
            Save Destination
          </button>
        </div>
      </div>
    </div>
  );
}

function TransferProgress({
  jobs,
  rates,
  overallProgress,
  totalSize,
  totalTransferred,
  completedCount,
  activeCount,
  queuedCount,
  failedCount,
  transferring,
  onClear,
}: {
  jobs: TransferJob[];
  rates: Record<number, number>;
  overallProgress: number;
  totalSize: number;
  totalTransferred: number;
  completedCount: number;
  activeCount: number;
  queuedCount: number;
  failedCount: number;
  transferring: boolean;
  onClear: () => void;
}) {
  const totalRate = Object.values(rates).reduce(
    (s, r) => s + Math.max(0, r),
    0
  );

  return (
    <div className="flex flex-col h-full">
      {/* Overall progress bar */}
      <div className="px-4 py-3 border-b border-border/50 space-y-2">
        <div className="flex items-center justify-between text-xs">
          <span className="text-text-secondary font-medium">
            Overall: {formatSize(totalTransferred)} / {formatSize(totalSize)}
            <span className="text-text-muted ml-2">
              ({(overallProgress * 100).toFixed(1)}%)
            </span>
          </span>
          <span className="text-text-muted flex items-center gap-2">
            {totalRate > 0 && formatRate(totalRate)}
            {activeCount > 0 && (
              <span>
                {activeCount} active
              </span>
            )}
            {queuedCount > 0 && (
              <span>
                {queuedCount} queued
              </span>
            )}
            {!transferring && (
              <button
                onClick={onClear}
                className="text-text-muted hover:text-text-primary transition-colors"
                title="Clear transfer history"
              >
                Clear
              </button>
            )}
          </span>
        </div>
        <div className="w-full h-2 bg-bg-tertiary rounded-full overflow-hidden">
          <div
            className="h-full bg-accent rounded-full transition-all duration-300"
            style={{ width: `${Math.min(overallProgress * 100, 100)}%` }}
          />
        </div>
      </div>

      {/* Individual file progress */}
      <div className="flex-1 overflow-y-auto px-4 py-2 space-y-1.5">
        {jobs.map((job) => {
          const progress = job.transferProgress ?? 0;
          const transferred = progress * job.fileSize;
          const rate = rates[job.id] ?? 0;

          return (
            <div
              key={job.id}
              className="flex items-center gap-3 py-1"
            >
              {/* Status icon */}
              <span className="flex-shrink-0 w-4 text-center">
                {job.status === "completed" ? (
                  <span className="text-success text-xs">&#10003;</span>
                ) : job.status === "failed" ? (
                  <span className="text-error text-xs">&#10007;</span>
                ) : job.status === "queued" ? (
                  <span className="text-text-muted text-xs">&#8943;</span>
                ) : (
                  <span className="animate-spin inline-block w-3 h-3 border border-accent/30 border-t-accent rounded-full" />
                )}
              </span>

              {/* File info + progress bar */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-0.5">
                  <span className="text-xs text-text-secondary truncate mr-2">
                    {job.fileName}
                  </span>
                  <span className="text-[10px] text-text-muted flex-shrink-0">
                    {job.status === "completed" ? (
                      formatSize(job.fileSize)
                    ) : job.status === "failed" ? (
                      <span className="text-error">
                        {job.transferError || "Failed"}
                      </span>
                    ) : job.status === "queued" ? (
                      <span className="text-text-muted">
                        {formatSize(job.fileSize)} &middot; queued
                      </span>
                    ) : (
                      <>
                        {formatSize(transferred)} / {formatSize(job.fileSize)}
                        {rate > 0 && (
                          <span className="ml-1.5">{formatRate(rate)}</span>
                        )}
                      </>
                    )}
                  </span>
                </div>
                {job.status === "transferring" && (
                  <div className="w-full h-1 bg-bg-tertiary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-accent/70 rounded-full transition-all duration-300"
                      style={{
                        width: `${Math.min(progress * 100, 100)}%`,
                      }}
                    />
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
