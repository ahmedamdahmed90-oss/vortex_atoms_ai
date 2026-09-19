import { useState, useEffect, Suspense, lazy } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ToastProvider } from './components/ui/Toast';
import { RTLLayout } from './components/layout/RTLLayout';
import { Header } from './components/layout/Header';
import { Sidebar } from './components/layout/Sidebar';
import { useSettingsStore } from './stores/settingsStore';
import { ErrorBoundary } from './components/ui/ErrorBoundary';
import { Spinner } from './components/ui/Spinner';

const ChatInterface = lazy(() => import('./components/chat/ChatInterface').then(m => ({ default: m.ChatInterface })));
const Dashboard = lazy(() => import('./components/dashboard/Dashboard').then(m => ({ default: m.Dashboard })));

const ChatIcon = () => (
  <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
  </svg>
);

const DashboardIcon = () => (
  <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
  </svg>
);

function AppRoutes() {
  const [page, setPage] = useState<'chat' | 'dashboard'>('chat');
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const tabs = [
    { id: 'chat', label: 'الشات', icon: <ChatIcon /> },
    { id: 'dashboard', label: 'لوحة التحكم', icon: <DashboardIcon /> },
  ];

  return (
    <RTLLayout
      sidebar={<Sidebar isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} activeTab={page} onTabChange={(tab) => { setPage(tab as 'chat' | 'dashboard'); setSidebarOpen(false); }} tabs={tabs} />}
      header={<Header onMenuClick={() => setSidebarOpen(true)} title={page === 'chat' ? 'الشات الذكي' : 'لوحة التحكم'} />}
      sidebarOpen={sidebarOpen}
    >
      <div className="animate-in h-full">
        <Suspense fallback={<div className="flex items-center justify-center h-full"><Spinner size="lg" /></div>}>
          {page === 'chat' ? <ChatInterface /> : <Dashboard />}
        </Suspense>
      </div>
    </RTLLayout>
  );
}

function App() {
  const { language } = useSettingsStore();

  useEffect(() => {
    document.documentElement.lang = language;
    document.documentElement.dir = language === 'ar' ? 'rtl' : 'ltr';
  }, [language]);

  return (
    <BrowserRouter>
      <ToastProvider>
        <ErrorBoundary>
          <Routes>
            <Route path="/*" element={<AppRoutes />} />
          </Routes>
        </ErrorBoundary>
      </ToastProvider>
    </BrowserRouter>
  );
}

export default App;