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

      const jobs = data as TransferJob[];
      setActiveTransfers(jobs);

      // Calculate transfer rates
      const now = Date.now();
      const newRates: Record<number, number> = {};
      for (const job of jobs) {
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

      // Update group statuses in the main table
      const hasActive = jobs.some((j) => j.status === "transferring");
      if (!hasActive && jobs.length > 0) {
        es.close();
        eventSourceRef.current = null;
        setTransferring(false);
        onRefresh();
      }
    };

    es.onerror = () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, [onRefresh]);

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

  const handleTransfer = async () => {
    const ids = Object.keys(selectedGroupIds).map(Number);
    if (!selectedDest || ids.length === 0) return;
    setTransferring(true);
    setActiveTransfers([]);
    prevProgress.current = {};
    setTransferRates({});
    await startTransfer({ groupIds: ids }, selectedDest);
    // Start listening for progress
    startProgressStream();
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

  return (
    <AnimatePresence>
      {transferDrawerOpen && (
        <motion.div
          initial={{ y: "100%" }}
          animate={{ y: 0 }}
          exit={{ y: "100%" }}
          transition={{ type: "spring", damping: 25, stiffness: 300 }}
          className="border-t border-border bg-bg-secondary"
          style={{ height: transferring || activeTransfers.length > 0 ? "360px" : "340px" }}
        >
          <div className="flex items-center justify-between px-6 py-2 border-b border-border">
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

          <div className="flex h-[calc(100%-40px)]">
            {/* Destinations list */}
            <div className="w-72 border-r border-border p-3 overflow-y-auto flex-shrink-0">
              <div className="flex items-center justify-between mb-2">
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

              {destinations.map((d) => (
                <div
                  key={d.id}
                  onClick={() => setSelectedDest(d.id)}
                  className={`flex items-center justify-between p-2 rounded cursor-pointer mb-1 transition-colors ${
                    selectedDest === d.id
                      ? "bg-accent/20 border border-accent/40"
                      : "hover:bg-bg-hover"
                  }`}
                >
                  <div>
                    <p className="text-sm text-text-primary">{d.name}</p>
                    <p className="text-xs text-text-muted truncate">
                      {d.basePath}
                    </p>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteDest(d.id);
                    }}
                    className="text-text-muted hover:text-error text-xs"
                  >
                    &times;
                  </button>
                </div>
              ))}
            </div>

            {/* Transfer action / progress area */}
            <div className="flex-1 flex flex-col overflow-hidden">
              {transferring || activeTransfers.length > 0 ? (
                <TransferProgress
                  jobs={activeTransfers}
                  rates={transferRates}
                  overallProgress={overallProgress}
                  totalSize={totalTransferSize}
                  totalTransferred={totalTransferred}
                  completedCount={completedCount}
                  activeCount={activeCount}
                  failedCount={failedCount}
                />
              ) : confirmedSelected.length === 0 ? (
                <div className="flex-1 flex items-center justify-center">
                  <p className="text-text-muted text-sm text-center">
                    Select confirmed groups in the queue and a destination to
                    transfer.
                  </p>
                </div>
              ) : !selectedDest ? (
                <div className="flex-1 flex items-center justify-center">
                  <p className="text-text-muted text-sm text-center">
                    Select a destination to transfer {totalFiles} file
                    {totalFiles !== 1 ? "s" : ""} from{" "}
                    {confirmedSelected.length} group
                    {confirmedSelected.length !== 1 ? "s" : ""}.
                  </p>
                </div>
              ) : (
                <div className="flex-1 flex items-center justify-center">
                  <div className="text-center space-y-3">
                    <p className="text-sm text-text-primary">
                      Transfer {totalFiles} file
                      {totalFiles !== 1 ? "s" : ""} from{" "}
                      {confirmedSelected.length} group
                      {confirmedSelected.length !== 1 ? "s" : ""} to{" "}
                      <span className="font-medium text-accent">
                        {destinations.find((d) => d.id === selectedDest)?.name}
                      </span>
                    </p>
                    <button
                      onClick={handleTransfer}
                      disabled={transferring}
                      className="px-6 py-2 rounded-md bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium"
                    >
                      Start Transfer
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </motion.div>
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
    </AnimatePresence>
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
        className="bg-bg-secondary border border-border rounded-lg shadow-xl w-[440px] flex flex-col"
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
  failedCount,
}: {
  jobs: TransferJob[];
  rates: Record<number, number>;
  overallProgress: number;
  totalSize: number;
  totalTransferred: number;
  completedCount: number;
  activeCount: number;
  failedCount: number;
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
          <span className="text-text-muted">
            {totalRate > 0 && formatRate(totalRate)}
            {activeCount > 0 && (
              <span className="ml-2">
                {activeCount} active
              </span>
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
