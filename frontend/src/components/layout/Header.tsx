import { clsx } from 'clsx';
import { X } from 'lucide-react';
import { useSettingsStore } from '../../stores/settingsStore';
import { useState, useEffect } from 'react';

interface SidebarProps {
  isOpen: boolean;
  onClose: () => void;
  activeTab: string;
  onTabChange: (tab: string) => void;
  tabs: Array<{ id: string; label: string; icon: React.ReactNode }>;
}

export function Sidebar({ isOpen, onClose, activeTab, onTabChange, tabs }: SidebarProps) {
  const { language } = useSettingsStore();

  return (
    <>
      {isOpen && <div className="fixed inset-0 bg-black/50 z-40 lg:hidden" onClick={onClose} aria-hidden="true" />}
      <aside
        className={clsx(
          'fixed lg:static inset-y-0 z-50 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700',
          'flex flex-col transition-transform duration-300 ease-in-out',
          'w-64',
          isOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0',
          language === 'ar' ? 'right-0' : 'left-0'
        )}
      >
        <div className="flex items-center justify-between h-16 px-4 border-b border-gray-200 dark:border-gray-700">
          <h1 className="text-xl font-bold text-primary-600 dark:text-primary-400">Vortex AI</h1>
          <button onClick={onClose} className="lg:hidden p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors" aria-label="إغلاق الشريط الجانبي">
            <X className="h-5 w-5" />
          </button>
        </div>

        <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto" role="navigation" aria-label="القائمة الرئيسية">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => onTabChange(tab.id)}
              className={clsx(
                'w-full flex items-center gap-3 px-4 py-3 rounded-lg text-sm font-medium transition-all duration-200',
                activeTab === tab.id
                  ? 'bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 hover:text-gray-900 dark:hover:text-white'
              )}
              role="tab"
              aria-selected={activeTab === tab.id}
            >
              <span className="flex-shrink-0">{tab.icon}</span>
              <span className="truncate">{tab.label}</span>
            </button>
          ))}
        </nav>

        <div className="p-4 border-t border-gray-200 dark:border-gray-700">
          <div className="text-xs text-gray-500 dark:text-gray-400 text-center">
            Vortex Atoms AI v0.1.0
          </div>
        </div>
      </aside>
    </>
  );
}

interface HeaderProps {
  onMenuClick: () => void;
  title: string;
  actions?: React.ReactNode;
}

export function Header({ onMenuClick, title, actions }: HeaderProps) {
  const { language, theme, setTheme } = useSettingsStore();
  const [darkMode, setDarkMode] = useState(false);
  const [showLangMenu, setShowLangMenu] = useState(false);
  const [showThemeMenu, setShowThemeMenu] = useState(false);

  useEffect(() => { if (theme === 'system') { setDarkMode(window.matchMedia('(prefers-color-scheme: dark)').matches); } else { setDarkMode(theme === 'dark'); } }, [theme]);

  const langOptions = [{ code: 'ar', label: 'العربية', icon: '🇸🇦' }, { code: 'en', label: 'English', icon: '🇺🇸' }];
  const themeOptions = [{ value: 'light' as const, label: 'فاتح', icon: <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" /></svg> }, { value: 'dark' as const, label: 'داكن', icon: <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" /></svg> }, { value: 'system' as const, label: 'النظام', icon: <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10a2 2 0 002 2h2a2 2 0 002-2zm0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" /></svg> }];

  return (
    <header className="sticky top-0 z-30 h-16 bg-white/80 dark:bg-gray-900/80 backdrop-blur-sm border-b border-gray-200 dark:border-gray-700"><div className="flex items-center justify-between h-full px-4 lg:px-6"><div className="flex items-center gap-4"><button onClick={onMenuClick} className="lg:hidden p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" aria-label="فتح القائمة"><svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" /></svg></button><h1 className="text-lg font-semibold text-gray-900 dark:text-gray-100 truncate">{title}</h1></div><div className="flex items-center gap-2">{actions}<div className="relative"><button onClick={() => setShowLangMenu(!showLangMenu)} className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" aria-label="اللغة"><svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9" /></svg><span className="hidden sm:inline">{language === 'ar' ? 'العربية' : 'English'}</span></button>{showLangMenu && (<><div className="fixed inset-0 z-40" onClick={() => setShowLangMenu(false)} /><div className="absolute right-0 mt-1 w-40 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-50 animate-in">{langOptions.map((opt) => (<button key={opt.code} onClick={() => { useSettingsStore.getState().setLanguage(opt.code as 'ar' | 'en'); setShowLangMenu(false); }} className="w-full flex items-center gap-2 px-4 py-2 text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700"><span>{opt.icon}</span><span>{opt.label}</span></button>))}</div></>)}</div><div className="relative"><button onClick={() => setShowThemeMenu(!showThemeMenu)} className="p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" aria-label={darkMode ? 'الوضع الفاتح' : 'الوضع الداكن'}>{darkMode ? <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" /></svg> : <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" /></svg>}</button>{showThemeMenu && (<><div className="fixed inset-0 z-40" onClick={() => setShowThemeMenu(false)} /><div className="absolute right-0 mt-1 w-40 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-50 animate-in">{themeOptions.map((opt) => (<button key={opt.value} onClick={() => { setTheme(opt.value); setShowThemeMenu(false); }} className="w-full flex items-center gap-2 px-4 py-2 text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700"><span>{opt.icon}</span><span>{opt.label}</span></button>))}</div></>)}</div></div></div></header>
  );
}