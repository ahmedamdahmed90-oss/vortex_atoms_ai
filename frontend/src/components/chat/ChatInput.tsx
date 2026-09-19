import { useState, useRef, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { Mic, X } from 'lucide-react';
import { useChat } from '../../hooks/useChat';
import { useSettingsStore } from '../../stores/settingsStore';
import { useChatStore } from '../../stores/chatStore';
import { Button } from '../ui/Button';
import { Textarea } from '../ui/Input';

export function ChatInput() {
  const { currentMessage, setCurrentMessage, isStreaming, isConnected, sendMessage, handleKeyDown, temperature, maxTokens, systemPrompt } = useChat();
  const { language } = useSettingsStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [showOptions, setShowOptions] = useState(false);

  const adjustHeight = useCallback(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`;
    }
  }, []);

  useEffect(() => { adjustHeight(); }, [adjustHeight]);

  const handleSubmit = (e: React.FormEvent) => { e.preventDefault(); if (currentMessage.trim() && !isStreaming) sendMessage(); };

  const handlePaste = (e: React.ClipboardEvent) => { const text = e.clipboardData.getData('text'); setCurrentMessage(currentMessage + text); };

  return (
    <form onSubmit={handleSubmit} className="w-full">
      <div className="relative">
        <div className="flex items-end gap-2 p-3 bg-white dark:bg-gray-800 rounded-2xl border border-gray-200 dark:border-gray-700 shadow-sm">
          <Textarea ref={textareaRef} value={currentMessage} onChange={(e) => setCurrentMessage(e.target.value)} onKeyDown={handleKeyDown} onPaste={handlePaste} placeholder={language === 'ar' ? 'اكتب رسالتك هنا...' : 'Type your message...'} disabled={isStreaming} rows={1} className={clsx('resize-none bg-transparent border-0 focus:ring-0 text-gray-900 dark:text-gray-100 placeholder-gray-400', 'pr-12')} style={{ minHeight: '44px', maxHeight: '200px' }} aria-label={language === 'ar' ? 'رسالة الشات' : 'Chat message'} />
          <div className="flex items-center gap-1 flex-shrink-0">
            <button type="button" onClick={() => setShowOptions(!showOptions)} className="p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" aria-label="خيارات إضافية"><svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.172 7l-6.586 6.586a1 1 0 101.414 1.414l6.586-6.586a1 1 0 00-1.414-1.414z" /></svg></button>
            <button type="button" className="p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" aria-label="ميكروفون"><Mic className="h-5 w-5" /></button>
            <Button type="submit" variant="primary" size="sm" disabled={!currentMessage.trim() || isStreaming || !isConnected} className="ml-auto" aria-label={language === 'ar' ? 'إرسال' : 'Send'}>{isStreaming ? (<svg className="animate-spin h-5 w-5" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" /><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" /></svg>) : (<svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" /></svg>)}</Button>
          </div>
        </div>

        {showOptions && (
          <div className="absolute bottom-full left-0 right-0 mb-2 p-3 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-lg animate-in">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">إعدادات التوليد</span>
              <button onClick={() => setShowOptions(false)} className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"><X className="h-4 w-4" /></button>
            </div>
            <div className="grid gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">درجة الحرارة: {temperature}</label>
                <input type="range" min="0" max="2" step="0.1" value={temperature} onChange={(e) => useChatStore.getState().setTemperature(Number(e.target.value))} className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-primary-600" />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">أقصى توكنز: {maxTokens}</label>
                <input type="range" min="64" max="512" step="64" value={maxTokens} onChange={(e) => useChatStore.getState().setMaxTokens(Number(e.target.value))} className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-primary-600" />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">برمجية النظام</label>
                <textarea value={systemPrompt} onChange={(e) => useChatStore.getState().setSystemPrompt(e.target.value)} rows={3} className="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-primary-500" placeholder="You are a helpful AI assistant." />
              </div>
            </div>
          </div>
        )}
      </div>
    </form>
  );
}