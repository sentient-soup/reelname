import { useAppStore } from "@/lib/store";
import { updateSettings } from "@/lib/api";
import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { open } from "@tauri-apps/plugin-dialog";

export function SettingsModal() {
  const { settingsOpen, setSettingsOpen, settings, setSettings } = useAppStore();
  const [form, setForm] = useState<Record<string, string>>({});

  useEffect(() => {
    setForm({ ...settings });
  }, [settings, settingsOpen]);

  const handleSave = async () => {
    const updated = await updateSettings(form);
    setSettings(updated);
    setSettingsOpen(false);
  };

  const handleBrowseScanPath = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select scan directory",
    });
    if (selected) {
      setForm({ ...form, scan_path: selected as string });
    }
  };

  return (
    <AnimatePresence>
      {settingsOpen && (
        <>
          <motion.div
            key="settings-backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/60 z-40"
            onClick={() => setSettingsOpen(false)}
          />
          <motion.div
            key="settings-panel"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4"
          >
            <div className="bg-bg-secondary border border-border rounded-xl w-full max-w-md p-6 space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-text-primary">Settings</h2>
                <button
                  onClick={() => setSettingsOpen(false)}
                  className="text-text-muted hover:text-text-primary text-xl leading-none"
                >
                  &times;
                </button>
              </div>

              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    Scan Path
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={form.scan_path || ""}
                      onChange={(e) => setForm({ ...form, scan_path: e.target.value })}
                      placeholder="/path/to/media/folder"
                      className="flex-1 px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                    />
                    <button
                      onClick={handleBrowseScanPath}
                      className="px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover transition-colors"
                    >
                      Browse
                    </button>
                  </div>
                </div>

                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    TMDB API Key
                  </label>
                  <input
                    type="password"
                    value={form.tmdb_api_key || ""}
                    onChange={(e) => setForm({ ...form, tmdb_api_key: e.target.value })}
                    placeholder="Enter your TMDB API key"
                    className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                  />
                  <p className="mt-1 text-xs text-text-muted">
                    Get one at themoviedb.org/settings/api
                  </p>
                </div>

                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    Auto-Match Threshold
                  </label>
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.05"
                    value={form.auto_match_threshold || "0.85"}
                    onChange={(e) =>
                      setForm({ ...form, auto_match_threshold: e.target.value })
                    }
                    className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
                  />
                </div>

                <hr className="border-border" />

                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    Naming Preset
                  </label>
                  <select
                    value={form.naming_preset || "jellyfin"}
                    onChange={(e) =>
                      setForm({ ...form, naming_preset: e.target.value })
                    }
                    className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary focus:outline-none focus:border-accent"
                  >
                    <option value="jellyfin">Jellyfin</option>
                    <option value="plex">Plex</option>
                  </select>
                  <p className="mt-1 text-xs text-text-muted">
                    {form.naming_preset === "plex"
                      ? "Plex: Title (Year)/Season 01/Title (Year) - s01e01 - Episode.ext"
                      : "Jellyfin: Title (Year)/Season 01/Title S01E01 - Episode.ext"}
                  </p>
                </div>

                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    Specials Folder Name
                  </label>
                  <input
                    type="text"
                    value={form.specials_folder_name || "Specials"}
                    onChange={(e) =>
                      setForm({ ...form, specials_folder_name: e.target.value })
                    }
                    placeholder="Specials"
                    className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-text-muted mb-1">
                    Extras Folder Name
                  </label>
                  <input
                    type="text"
                    value={form.extras_folder_name || "Extras"}
                    onChange={(e) =>
                      setForm({ ...form, extras_folder_name: e.target.value })
                    }
                    placeholder="Extras"
                    className="w-full px-3 py-2 text-sm rounded-md bg-bg-tertiary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                  />
                </div>
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  onClick={() => setSettingsOpen(false)}
                  className="px-4 py-2 text-sm rounded-md bg-bg-tertiary text-text-secondary hover:bg-bg-hover transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSave}
                  className="px-4 py-2 text-sm rounded-md bg-accent text-white hover:bg-accent-hover transition-colors"
                >
                  Save
                </button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
