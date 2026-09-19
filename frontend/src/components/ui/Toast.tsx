import { useState, useEffect, useContext, createContext } from 'react';
import { createPortal } from 'react-dom';
import { clsx } from 'clsx';
import { X, CheckCircle, AlertCircle, Info, AlertTriangle } from 'lucide-react';

interface Toast {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  title: string;
  message?: string;
  duration?: number;
}

interface ToastContextValue {
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = (toast: Omit<Toast, 'id'>) => {
    const id = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    setToasts((prev) => [...prev, { ...toast, id }]);
  };

  const removeToast = (id: string) => { setToasts((prev) => prev.filter((t) => t.id !== id)); };

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </ToastContext.Provider>
  );
}

// eslint-disable-next-line react/only-export-components
export function useToast() {
  const context = useContext(ToastContext);
  if (!context) throw new Error('useToast must be used within a ToastProvider');
  return context;
}

function ToastContainer({ toasts, onRemove }: { toasts: Toast[]; onRemove: (id: string) => void }) {
  return createPortal(
    <div className="fixed bottom-4 left-4 z-50 flex flex-col-reverse gap-2 pointer-events-none">{toasts.map((toast) => <ToastItem key={toast.id} toast={toast} onRemove={onRemove} />)}</div>,
    document.body
  );
}

function ToastItem({ toast, onRemove }: { toast: Toast; onRemove: (id: string) => void }) {
  useEffect(() => {
    const ms = toast.duration ?? 5000;
    if (ms <= 0) return;
    const timer = setTimeout(() => onRemove(toast.id), ms);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, onRemove]);
  const icons = { success: CheckCircle, error: AlertCircle, info: Info, warning: AlertTriangle };
  const styles: Record<string, string> = { success: 'bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-800 text-green-800 dark:text-green-300', error: 'bg-red-50 dark:bg-red-900/30 border-red-200 dark:border-red-800 text-red-800 dark:text-red-300', info: 'bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-300', warning: 'bg-yellow-50 dark:bg-yellow-900/30 border-yellow-200 dark:border-yellow-800 text-yellow-800 dark:text-yellow-300' };
  const Icon = icons[toast.type];
  return (
    <div className={clsx('pointer-events-auto flex items-start gap-3 p-4 rounded-xl border shadow-lg min-w-[300px] max-w-md animate-in', styles[toast.type])} role="alert">
      <Icon className="h-5 w-5 flex-shrink-0 mt-0.5" />
      <div className="flex-1 min-w-0"><p className="font-medium">{toast.title}</p>{toast.message && <p className="mt-1 text-sm opacity-80">{toast.message}</p>}</div>
      <button onClick={() => onRemove(toast.id)} className="flex-shrink-0 p-1 rounded hover:bg-black/10 dark:hover:bg-white/10 transition-colors" aria-label="إغلاق"><X className="h-4 w-4" /></button>
    </div>
  );
}