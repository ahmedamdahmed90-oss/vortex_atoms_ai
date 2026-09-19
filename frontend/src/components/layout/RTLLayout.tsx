import { useSettingsStore } from '../../stores/settingsStore';

interface RTLLayoutProps {
  children: React.ReactNode;
  sidebar?: React.ReactNode;
  header?: React.ReactNode;
  sidebarOpen?: boolean;
  onSidebarToggle?: () => void;
}

export function RTLLayout({ children, sidebar, header, sidebarOpen: _sidebarOpen, onSidebarToggle: _onSidebarToggle }: RTLLayoutProps) {
  const { language } = useSettingsStore();

  return (
    <div className="min-h-screen flex" dir={language === 'ar' ? 'rtl' : 'ltr'} style={{ direction: language === 'ar' ? 'rtl' : 'ltr' }}>
      {sidebar && <div className="hidden lg:flex lg:flex-col">{sidebar}</div>}
      <div className="flex-1 flex flex-col min-w-0">
        {header && <div className="lg:ml-64 lg:w-[calc(100%-16rem)]">{header}</div>}
        <main className="flex-1 p-4 lg:p-6 lg:ml-64 overflow-auto">{children}</main>
      </div>
    </div>
  );
}