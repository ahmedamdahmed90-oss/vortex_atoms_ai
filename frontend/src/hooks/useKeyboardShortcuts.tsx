import { useEffect, useRef } from 'react';
import { useSettingsStore } from '../stores/settingsStore';

interface Shortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  action: () => void;
}

// eslint-disable-next-line react/only-export-components
export function useKeyboardShortcuts(shortcuts: Shortcut[]) {
  const shortcutsRef = useRef(shortcuts);
  shortcutsRef.current = shortcuts;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      for (const shortcut of shortcutsRef.current) {
        const keyMatch = e.key.toLowerCase() === shortcut.key.toLowerCase();
        const ctrlMatch = e.ctrlKey === (shortcut.ctrl || false);
        const shiftMatch = e.shiftKey === (shortcut.shift || false);
        const altMatch = e.altKey === (shortcut.alt || false);

        if (keyMatch && ctrlMatch && shiftMatch && altMatch) {
          e.preventDefault();
          e.stopPropagation();
          shortcut.action();
          return;
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}

export function KeyboardShortcutsHelp({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { language } = useSettingsStore();
  const isArabic = language === 'ar';

  const shortcuts = [
    { keys: ['Ctrl', 'K'], description: isArabic ? 'فتح لوحة الأوامر' : 'Open command palette' },
    { keys: ['Ctrl', 'N'], description: isArabic ? 'محادثة جديدة' : 'New chat' },
    { keys: ['Ctrl', 'Shift', 'D'], description: isArabic ? 'تبديل الوضع الداكن' : 'Toggle dark mode' },
    { keys: ['Ctrl', '/'], description: isArabic ? 'عرض الاختصارات' : 'Show shortcuts' },
    { keys: ['Escape'], description: isArabic ? 'إغلاق' : 'Close' },
  ];

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full p-6 animate-in" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-xl font-bold text-gray-900 dark:text-gray-100 mb-4">
          {isArabic ? 'اختصارات لوحة المفاتيح' : 'Keyboard Shortcuts'}
        </h2>
        <div className="space-y-3">
          {shortcuts.map((shortcut, index) => (
            <div key={index} className="flex items-center justify-between">
              <span className="text-sm text-gray-600 dark:text-gray-400">{shortcut.description}</span>
              <div className="flex gap-1">
                {shortcut.keys.map((key, keyIndex) => (
                  <kbd key={keyIndex} className="px-2 py-1 text-xs font-mono bg-gray-100 dark:bg-gray-700 rounded border border-gray-300 dark:border-gray-600">
                    {key}
                  </kbd>
                ))}
              </div>
            </div>
          ))}
        </div>
        <button
          onClick={onClose}
          className="mt-6 w-full py-2 px-4 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
        >
          {isArabic ? 'إغلاق' : 'Close'}
        </button>
      </div>
    </div>
  );
}

// eslint-disable-next-line react/only-export-components
export function useChatKeyboardShortcuts(actions: {
  onNewChat: () => void;
  onSendMessage: () => void;
  onClearChat: () => void;
  onToggleTheme: () => void;
  onShowShortcuts: () => void;
}) {
  const { language } = useSettingsStore();
  const isArabic = language === 'ar';

  const shortcuts: Shortcut[] = [
    {
      key: 'n',
      ctrl: true,
      description: isArabic ? 'محادثة جديدة' : 'New chat',
      action: actions.onNewChat,
    },
    {
      key: 'Enter',
      ctrl: true,
      description: isArabic ? 'إرسال رسالة' : 'Send message',
      action: actions.onSendMessage,
    },
    {
      key: 'l',
      ctrl: true,
      shift: true,
      description: isArabic ? 'مسح المحادثة' : 'Clear chat',
      action: actions.onClearChat,
    },
    {
      key: 'd',
      ctrl: true,
      shift: true,
      description: isArabic ? 'تبديل المظهر' : 'Toggle theme',
      action: actions.onToggleTheme,
    },
    {
      key: '/',
      ctrl: true,
      description: isArabic ? 'عرض الاختصارات' : 'Show shortcuts',
      action: actions.onShowShortcuts,
    },
  ];

  useKeyboardShortcuts(shortcuts);
}