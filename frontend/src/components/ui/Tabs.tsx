import { clsx } from 'clsx';

interface TabsProps {
  tabs: Array<{ id: string; label: string; icon?: React.ReactNode }>;
  activeTab: string;
  onChange: (tabId: string) => void;
  className?: string;
  variant?: 'default' | 'pills' | 'underline';
}

export function Tabs({ tabs, activeTab, onChange, className, variant = 'default' }: TabsProps) {
  const variants = {
    default: 'bg-gray-100 dark:bg-gray-800 rounded-lg p-1',
    pills: 'space-x-1',
    underline: 'border-b border-gray-200 dark:border-gray-700',
  };

  const tabVariants = {
    default: 'px-4 py-2 rounded-md text-sm font-medium transition-all duration-200',
    pills: 'px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200',
    underline: 'px-4 py-3 border-b-2 -mb-px text-sm font-medium transition-all duration-200',
  };

  const activeStyles = {
    default: 'bg-white dark:bg-gray-700 text-primary-700 dark:text-primary-300 shadow-sm',
    pills: 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300',
    underline: 'border-primary-500 text-primary-600 dark:text-primary-400',
  };

  const inactiveStyles = {
    default: 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100',
    pills: 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 hover:text-gray-900 dark:hover:text-white',
    underline: 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200',
  };

  return (
    <div className={clsx(variants[variant], className)} role="tablist">
      {tabs.map((tab) => {
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            aria-controls={`panel-${tab.id}`}
            id={`tab-${tab.id}`}
            onClick={() => onChange(tab.id)}
            className={clsx(
              tabVariants[variant],
              isActive ? activeStyles[variant] : inactiveStyles[variant],
              'flex items-center gap-2'
            )}
          >
            {tab.icon && <span className="flex-shrink-0">{tab.icon}</span>}
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}

interface TabPanelProps {
  id: string;
  activeTab: string;
  children: React.ReactNode;
  className?: string;
}

export function TabPanel({ id, activeTab, children, className }: TabPanelProps) {
  if (activeTab !== id) return null;
  
  return (
    <div
      role="tabpanel"
      id={`panel-${id}`}
      aria-labelledby={`tab-${id}`}
      className={clsx('animate-in', className)}
    >
      {children}
    </div>
  );
}