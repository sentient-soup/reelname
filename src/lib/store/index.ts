import { create } from "zustand";
import type { Job, Group, MatchCandidate, Destination } from "@/lib/types";

export type JobWithPreview = Job & {
  previewName?: string | null;
};

export type GroupWithJobs = Group & {
  jobs: JobWithPreview[];
  candidates?: MatchCandidate[];
};

interface AppState {
  // Groups
  groups: GroupWithJobs[];
  totalGroups: number;
  loading: boolean;
  scanning: boolean;

  // Expanded groups (using Record for reliable Zustand shallow comparison)
  expandedGroupIds: Record<number, boolean>;

  // Filters
  statusFilter: string | null;
  mediaTypeFilter: string | null;
  searchQuery: string;
  page: number;
  sortBy: string;
  sortDir: "asc" | "desc";

  // Selection (group-level, using Record for reliable Zustand shallow comparison)
  selectedGroupIds: Record<number, boolean>;
  activeGroupId: number | null;
  activeGroup: GroupWithJobs | null;

  // Panels
  matchPanelOpen: boolean;
  settingsOpen: boolean;
  transferDrawerOpen: boolean;

  // Settings
  settings: Record<string, string>;

  // Destinations
  destinations: Destination[];

  // Actions
  setGroups: (groups: GroupWithJobs[], total: number) => void;
  setLoading: (loading: boolean) => void;
  setScanning: (scanning: boolean) => void;
  setStatusFilter: (status: string | null) => void;
  setMediaTypeFilter: (mediaType: string | null) => void;
  setSearchQuery: (query: string) => void;
  setPage: (page: number) => void;
  setSorting: (sortBy: string, sortDir: "asc" | "desc") => void;

  // Group expansion
  toggleGroupExpanded: (id: number) => void;
  expandAll: () => void;
  collapseAll: () => void;

  // Selection
  toggleGroupSelection: (id: number) => void;
  selectAllGroups: () => void;
  clearSelection: () => void;
  getSelectedGroupIds: () => number[];

  // Active group
  setActiveGroup: (group: GroupWithJobs | null) => void;
  setMatchPanelOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setTransferDrawerOpen: (open: boolean) => void;
  setSettings: (settings: Record<string, string>) => void;
  setDestinations: (destinations: Destination[]) => void;

  // Mutations
  updateGroup: (id: number, updates: Partial<Group>) => void;
  removeGroups: (ids: number[]) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  groups: [],
  totalGroups: 0,
  loading: false,
  scanning: false,

  expandedGroupIds: {},

  statusFilter: null,
  mediaTypeFilter: null,
  searchQuery: "",
  page: 1,
  sortBy: "createdAt",
  sortDir: "desc",

  selectedGroupIds: {},
  activeGroupId: null,
  activeGroup: null,

  matchPanelOpen: true,
  settingsOpen: false,
  transferDrawerOpen: false,

  settings: {},
  destinations: [],

  setGroups: (groups, total) => set({ groups, totalGroups: total }),
  setLoading: (loading) => set({ loading }),
  setScanning: (scanning) => set({ scanning }),
  setStatusFilter: (statusFilter) => set({ statusFilter, page: 1 }),
  setMediaTypeFilter: (mediaTypeFilter) => set({ mediaTypeFilter, page: 1 }),
  setSearchQuery: (searchQuery) => set({ searchQuery, page: 1 }),
  setPage: (page) => set({ page }),
  setSorting: (sortBy, sortDir) => set({ sortBy, sortDir }),

  toggleGroupExpanded: (id) =>
    set((state) => {
      const next = { ...state.expandedGroupIds };
      if (next[id]) {
        delete next[id];
      } else {
        next[id] = true;
      }
      return { expandedGroupIds: next };
    }),

  expandAll: () =>
    set((state) => {
      const next: Record<number, boolean> = {};
      for (const g of state.groups) next[g.id] = true;
      return { expandedGroupIds: next };
    }),

  collapseAll: () => set({ expandedGroupIds: {} }),

  toggleGroupSelection: (id) =>
    set((state) => {
      const next = { ...state.selectedGroupIds };
      if (next[id]) {
        delete next[id];
      } else {
        next[id] = true;
      }
      return { selectedGroupIds: next };
    }),

  selectAllGroups: () =>
    set((state) => {
      const next: Record<number, boolean> = {};
      for (const g of state.groups) next[g.id] = true;
      return { selectedGroupIds: next };
    }),

  clearSelection: () => set({ selectedGroupIds: {} }),

  getSelectedGroupIds: () => {
    const sel = get().selectedGroupIds;
    return Object.keys(sel).filter((k) => sel[Number(k)]).map(Number);
  },

  setActiveGroup: (group) =>
    set({
      activeGroup: group,
      activeGroupId: group?.id ?? null,
      matchPanelOpen: !!group,
    }),

  setMatchPanelOpen: (open) =>
    set({
      matchPanelOpen: open,
      ...(open ? {} : { activeGroup: null, activeGroupId: null }),
    }),

  setSettingsOpen: (open) => set({ settingsOpen: open }),
  setTransferDrawerOpen: (open) => set({ transferDrawerOpen: open }),
  setSettings: (settings) => set({ settings }),
  setDestinations: (destinations) => set({ destinations }),

  updateGroup: (id, updates) =>
    set((state) => ({
      groups: state.groups.map((g) =>
        g.id === id ? { ...g, ...updates } : g
      ),
      activeGroup:
        state.activeGroup?.id === id
          ? { ...state.activeGroup, ...updates }
          : state.activeGroup,
    })),

  removeGroups: (ids) => {
    const idSet = new Set(ids);
    set((state) => {
      const nextSelected = { ...state.selectedGroupIds };
      for (const id of ids) delete nextSelected[id];
      return {
        groups: state.groups.filter((g) => !idSet.has(g.id)),
        totalGroups: state.totalGroups - ids.length,
        selectedGroupIds: nextSelected,
        activeGroup:
          state.activeGroup && idSet.has(state.activeGroup.id)
            ? null
            : state.activeGroup,
        matchPanelOpen:
          state.activeGroup && idSet.has(state.activeGroup.id)
            ? false
            : state.matchPanelOpen,
      };
    });
  },
}));
