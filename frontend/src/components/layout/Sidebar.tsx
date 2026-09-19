import { clsx } from 'clsx';
import { X } from 'lucide-react';
import { useSettingsStore } from '../../stores/settingsStore';

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