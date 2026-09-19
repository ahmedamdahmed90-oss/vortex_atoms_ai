import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Language } from '../i18n/config';
import { DEFAULT_LANGUAGE, STORAGE_KEY } from '../i18n/config';

interface SettingsState {
  language: Language;
  theme: 'light' | 'dark' | 'system';
  apiUrl: string;
  wsUrl: string;
  animationsEnabled: boolean;
  compactMode: boolean;
  
  setLanguage: (lang: Language) => void;
  setTheme: (theme: SettingsState['theme']) => void;
  setApiUrl: (url: string) => void;
  setWsUrl: (url: string) => void;
  setAnimationsEnabled: (enabled: boolean) => void;
  setCompactMode: (enabled: boolean) => void;
  applyTheme: () => void;
  resetToDefaults: () => void;
}

const DEFAULT_SETTINGS = {
  language: DEFAULT_LANGUAGE,
  theme: 'system' as const,
  apiUrl: 'http://localhost:8080/v1',
  wsUrl: 'ws://localhost:8080/ws',
  animationsEnabled: true,
  compactMode: false,
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      ...DEFAULT_SETTINGS,

      setLanguage: (lang) => {
        localStorage.setItem(STORAGE_KEY, lang);
        document.documentElement.lang = lang;
        document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
        set({ language: lang });
      },
      setTheme: (theme) => {
        set({ theme });
        get().applyTheme();
      },
      setApiUrl: (url) => set({ apiUrl: url }),
      setWsUrl: (url) => set({ wsUrl: url }),
      setAnimationsEnabled: (enabled) => {
        set({ animationsEnabled: enabled });
        document.documentElement.classList.toggle('animations-disabled', !enabled);
      },
      setCompactMode: (enabled) => {
        set({ compactMode: enabled });
        document.documentElement.classList.toggle('compact-mode', enabled);
      },
      applyTheme: () => {
        const { theme } = get();
        const root = document.documentElement;
        
        if (theme === 'system') {
          const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
          root.classList.toggle('dark', prefersDark);
        } else {
          root.classList.toggle('dark', theme === 'dark');
        }
      },
      resetToDefaults: () => {
        localStorage.removeItem(STORAGE_KEY);
        localStorage.removeItem('vortex-settings-store');
        set(DEFAULT_SETTINGS);
        get().applyTheme();
        document.documentElement.classList.remove('animations-disabled', 'compact-mode');
      },
    }),
    {
      name: 'vortex-settings-store',
      version: 2,
      migrate: (persistedState: unknown) => {
        const state = persistedState as Partial<SettingsState>;
        return {
          ...DEFAULT_SETTINGS,
          ...state,
          animationsEnabled: state.animationsEnabled ?? true,
          compactMode: state.compactMode ?? false,
        };
      },
    }
  )
);

const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
mediaQuery.addEventListener('change', () => {
  const { theme, applyTheme } = useSettingsStore.getState();
  if (theme === 'system') {
    applyTheme();
  }
});