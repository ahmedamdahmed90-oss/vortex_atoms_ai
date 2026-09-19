import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { HealthResponse } from '../types';

interface DashboardState {
  activeTab: 'overview' | 'models' | 'knowledge' | 'tools' | 'logs' | 'settings' | 'security' | 'performance';
  sidebarOpen: boolean;

  // Overview
  health: HealthResponse | null;
  
  // Models
  models: Array<{ name: string; architecture: string; max_seq_len: number; max_generation_tokens: number }> | null;
  currentModel: string | null;
  
  // Knowledge
  knowledgeChunks: number;
  searchResults: Array<{ id: string; score: number; text: string }> | null;
  
  // Tools
  tools: Array<{ name: string; description: string }> | null;
  
  // Logs
  logs: string[];
  logFilter: 'all' | 'error' | 'warn' | 'info' | 'debug';
  
  setActiveTab: (tab: DashboardState['activeTab']) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setHealth: (health: DashboardState['health']) => void;
  setModels: (models: DashboardState['models']) => void;
  setCurrentModel: (model: string) => void;
  setKnowledgeChunks: (count: number) => void;
  setSearchResults: (results: DashboardState['searchResults']) => void;
  setTools: (tools: DashboardState['tools']) => void;
  addLog: (log: string) => void;
  clearLogs: () => void;
  setLogFilter: (filter: DashboardState['logFilter']) => void;
}

export const useDashboardStore = create<DashboardState>()(
  persist(
    (set) => ({
      activeTab: 'overview',
      sidebarOpen: true,
      health: null,
      models: null,
      currentModel: null,
      knowledgeChunks: 0,
      searchResults: null,
      tools: null,
      logs: [],
      logFilter: 'all',

      setActiveTab: (tab) => set({ activeTab: tab }),
      toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
      setSidebarOpen: (open) => set({ sidebarOpen: open }),
      setHealth: (health) => set({ health }),
      setModels: (models) => set({ models }),
      setCurrentModel: (model) => set({ currentModel: model }),
      setKnowledgeChunks: (count) => set({ knowledgeChunks: count }),
      setSearchResults: (results) => set({ searchResults: results }),
      setTools: (tools) => set({ tools }),
      addLog: (log) => set((state) => ({ logs: [log, ...state.logs].slice(0, 1000) })),
      clearLogs: () => set({ logs: [] }),
      setLogFilter: (filter) => set({ logFilter: filter }),
    }),
    {
      name: 'vortex-dashboard-store',
      partialize: (state) => ({
        activeTab: state.activeTab,
        sidebarOpen: state.sidebarOpen,
        logFilter: state.logFilter,
      }),
    }
  )
);