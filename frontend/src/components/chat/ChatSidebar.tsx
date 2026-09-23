import { X } from 'lucide-react';
import { useChatStore } from '../../stores/chatStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { Button } from '../ui/Button';
import { formatTime } from '../../utils/helpers';

interface ChatSidebarProps {
  isOpen: boolean;
  onClose: () => void;
  onNewChat: () => void;
}

export function ChatSidebar({ isOpen, onClose, onNewChat }: ChatSidebarProps) {
  const { language } = useSettingsStore();
  const { temperature, maxTokens, systemPrompt, setTemperature, setMaxTokens, setSystemPrompt } = useChatStore();

  const conversations = [{ id: 'current', title: language === 'ar' ? 'المحادثة الحالية' : 'Current Chat', time: Date.now() }];

  return (
    <>
      {isOpen && <div className="fixed inset-0 bg-black/50 z-40 lg:hidden" onClick={onClose} aria-hidden="true" />}
      <aside
        className={`fixed lg:static inset-y-0 z-50 bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 flex flex-col transition-transform duration-300 ease-in-out w-80 ${
          isOpen ? 'translate-x-0' : 'translate-x-full lg:translate-x-0'
        } ${isOpen ? '' : 'invisible lg:visible'}`}
        style={{ right: '0' }}
      >
        <div className="flex items-center justify-between h-16 px-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">{language === 'ar' ? 'المحادثات' : 'Conversations'}</h2>
          <button onClick={onClose} className="lg:hidden p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors" aria-label="إغلاق الشريط الجانبي"><X className="h-5 w-5" /></button>
        </div>

        <div className="p-3 border-b border-gray-200 dark:border-gray-700"><Button onClick={onNewChat} variant="primary" size="sm" className="w-full gap-2" leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" /></svg>}>{language === 'ar' ? 'محادثة جديدة' : 'New Chat'}</Button></div>

        <nav className="flex-1 overflow-y-auto px-3 py-3 space-y-1" aria-label="قائمة المحادثات">{conversations.map((conv) => (<button key={conv.id} className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all duration-200 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 hover:text-gray-900 dark:hover:text-white"><svg className="h-4 w-4 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" /></svg><span className="truncate flex-1">{conv.title}</span><span className="text-xs text-gray-400 flex-shrink-0">{formatTime(conv.time)}</span></button>))}</nav>

        <div className="p-3 border-t border-gray-200 dark:border-gray-700 space-y-2">
          <p className="flex items-center gap-2 px-2 py-1 text-sm font-medium text-gray-600 dark:text-gray-400"><svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /></svg>{language === 'ar' ? 'إعدادات الشات' : 'Chat Settings'}</p>

          <div className="space-y-3 pt-2"><div><label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">درجة الحرارة: {temperature}</label><input type="range" min="0" max="2" step="0.1" value={temperature} onChange={(e) => setTemperature(Number(e.target.value))} className="w-full h-1.5 bg-gray-200 dark:bg-gray-700 rounded appearance-none cursor-pointer accent-primary-600" /></div><div><label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">أقصى توكنز: {maxTokens}</label><input type="range" min="64" max="512" step="64" value={maxTokens} onChange={(e) => setMaxTokens(Number(e.target.value))} className="w-full h-1.5 bg-gray-200 dark:bg-gray-700 rounded appearance-none cursor-pointer accent-primary-600" /></div><div><label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">برمجية النظام</label><textarea value={systemPrompt} onChange={(e) => setSystemPrompt(e.target.value)} rows={3} className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded text-xs text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-1 focus:ring-primary-500" placeholder="You are a helpful AI assistant." /></div></div></div>
      </aside>
    </>
  );
}