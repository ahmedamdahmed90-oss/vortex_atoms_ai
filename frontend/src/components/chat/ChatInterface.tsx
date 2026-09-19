import { useState, useRef, useCallback, useEffect } from 'react';
import { MessageSquare, Sparkles, Trash2, Command } from 'lucide-react';
import { useChat } from '../../hooks/useChat';
import { useSettingsStore } from '../../stores/settingsStore';
import { useWebSocket } from '../../hooks/useWebSocket';
import { useNetworkStatus, useChatKeyboardShortcuts } from '../../hooks';
import { KeyboardShortcutsHelp } from '../../hooks/useKeyboardShortcuts';
import { MessageBubble } from './MessageBubble';
import { ChatInput } from './ChatInput';
import { ChatSidebar } from './ChatSidebar';
import { ConnectionStatus } from '../ui/ConnectionStatus';


export function ChatInterface() {
  const { messages, isStreaming, isConnected, sendMessage, clearChat, regenerate, error, setCurrentMessage } = useChat();
  const { language, theme, setTheme } = useSettingsStore();
  const { isConnected: wsConnected } = useWebSocket();
  const { isOnline, isServerReachable, latency, checkServerConnection, isChecking } = useNetworkStatus(15000);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showWelcome, setShowWelcome] = useState(messages.length === 0);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const chatContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (messages.length > 0) setShowWelcome(false);
  }, [messages.length]);

  useEffect(() => {
    const timer = setTimeout(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, 100);
    return () => clearTimeout(timer);
  }, [messages, isStreaming]);

  const handleNewChat = useCallback(() => {
    clearChat();
    setShowWelcome(true);
    setSidebarOpen(false);
  }, [clearChat]);

  const handleSendMessage = useCallback(() => {
    const textarea = document.querySelector('textarea[aria-label="رسالة الشات"]') as HTMLTextAreaElement;
    if (textarea && textarea.value.trim()) {
      sendMessage();
    }
  }, [sendMessage]);

  const handleClearChat = useCallback(() => {
    if (confirm(language === 'ar' ? 'هل تريد مسح المحادثة؟' : 'Clear chat?')) clearChat();
  }, [clearChat, language]);

  const handleToggleTheme = useCallback(() => {
    const newTheme = theme === 'dark' ? 'light' : 'dark';
    setTheme(newTheme);
  }, [theme, setTheme]);

  const handleShowShortcuts = useCallback(() => {
    setShowShortcuts(true);
  }, []);

  useChatKeyboardShortcuts({
    onNewChat: handleNewChat,
    onSendMessage: handleSendMessage,
    onClearChat: handleClearChat,
    onToggleTheme: handleToggleTheme,
    onShowShortcuts: handleShowShortcuts,
  });

  const _connected = isConnected && wsConnected;
  const isArabic = language === 'ar';

  const examplePrompts = isArabic
    ? [
        'اكتب دالة بايثون لحساب فيبوناتشي',
        'اشرح لي مفهوم البرمجة الوظيفية',
        'ما الفرق بين React و Vue؟',
        'كيف يعمل الذكاء الاصطناعي؟',
      ]
    : [
        'Write a Python function for Fibonacci',
        'Explain functional programming',
        'What is the difference between React and Vue?',
        'How does AI work?',
      ];

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Header */}
      <header className="flex items-center justify-between h-14 px-4 border-b border-gray-200 dark:border-gray-700 bg-white/50 dark:bg-gray-900/50 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="lg:hidden p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            aria-label={isArabic ? 'فتح الشريط الجانبي' : 'Open sidebar'}
          >
            <MessageSquare className="h-5 w-5" />
          </button>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
              <Sparkles className="h-4 w-4 text-primary-600 dark:text-primary-400" />
            </div>
            <div>
              <h1 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {isArabic ? 'الشات الذكي' : 'Smart Chat'}
              </h1>
              <ConnectionStatus
                isOnline={isOnline}
                isServerReachable={isServerReachable}
                latency={latency}
                onRetry={checkServerConnection}
                isChecking={isChecking}
                compact
              />
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleNewChat}
            className="hidden sm:flex items-center gap-2 px-3 py-1.5 text-sm rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
            title={isArabic ? 'محادثة جديدة (Ctrl+N)' : 'New chat (Ctrl+N)'}
          >
            <Sparkles className="h-4 w-4" />
            {isArabic ? 'جديد' : 'New'}
          </button>
          <button
            onClick={handleClearChat}
            className="p-2 rounded-lg text-gray-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            title={isArabic ? 'مسح المحادثة' : 'Clear chat'}
          >
            <Trash2 className="h-4 w-4" />
          </button>
          <button
            onClick={handleShowShortcuts}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            title={isArabic ? 'اختصارات لوحة المفاتيح (Ctrl+/)' : 'Keyboard shortcuts (Ctrl+/)'}
            aria-label={isArabic ? 'اختصارات لوحة المفاتيح' : 'Keyboard shortcuts'}
          >
            <Command className="h-4 w-4" />
          </button>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        <ChatSidebar isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} onNewChat={handleNewChat} />
        <div className="flex-1 flex flex-col min-w-0 lg:ml-0">
          <div
            ref={chatContainerRef}
            className="flex-1 overflow-y-auto p-4 space-y-4 scrollbar-thin"
            role="log"
            aria-live="polite"
            aria-label={isArabic ? 'رسائل الشات' : 'Chat messages'}
          >
            {/* Connection Banner */}
            {(!isOnline || !isServerReachable) && (
              <div className="sticky top-0 z-10 -mx-4 px-4 pb-2">
                <ConnectionStatus
                  isOnline={isOnline}
                  isServerReachable={isServerReachable}
                  latency={latency}
                  onRetry={checkServerConnection}
                  isChecking={isChecking}
                />
              </div>
            )}

            {/* Error Display */}
            {error && (
              <div className="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800" role="alert">
                <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
              </div>
            )}

            {/* Welcome Screen */}
            {showWelcome && messages.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full min-h-[400px] text-center">
                <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-primary-400 to-primary-600 flex items-center justify-center mb-6 mx-auto shadow-lg shadow-primary-500/20">
                  <Sparkles className="h-10 w-10 text-white" />
                </div>
                <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                  {isArabic ? 'مرحباً بك في Vortex AI' : 'Welcome to Vortex AI'}
                </h2>
                <p className="text-gray-500 dark:text-gray-400 max-w-md mx-auto mb-8">
                  {isArabic
                    ? 'مساعد ذكي يدعم البث المباشر، الأدوات المتقدمة، وقاعدة المعرفة. ابدأ بكتابة رسالة أو اختر أحد الأمثلة.'
                    : 'A smart assistant with streaming, advanced tools, and knowledge base. Start typing or choose an example.'}
                </p>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 w-full max-w-lg">
                  {examplePrompts.map((prompt, index) => (
                    <button
                      key={index}
                      onClick={() => {
                        setCurrentMessage(prompt);
                        const textarea = document.querySelector('textarea[aria-label="رسالة الشات"]') as HTMLTextAreaElement | null;
                        textarea?.focus();
                      }}
                      className="p-3 rounded-xl bg-gray-100 dark:bg-gray-800 text-right text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 hover:ring-2 hover:ring-primary-300 dark:hover:ring-primary-700 transition-all"
                    >
                      {prompt}
                    </button>
                  ))}
                </div>
              </div>
            ) : (
              <>
                {/* Messages */}
                {messages.map((message) => (
                  <MessageBubble
                    key={message.id}
                    message={message}
                    onRegenerate={message.role === 'assistant' ? () => regenerate() : undefined}
                  />
                ))}

                {/* Streaming Indicator */}
                {isStreaming && (
                  <div className="flex items-start gap-3 animate-in">
                    <div className="flex-shrink-0 w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                      <Sparkles className="h-4 w-4 text-primary-600 dark:text-primary-400 animate-pulse" />
                    </div>
                    <div className="bg-white dark:bg-gray-800 rounded-2xl rounded-tl-none p-4 shadow-sm border border-gray-200 dark:border-gray-700">
                      <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400">
                        <div className="flex gap-1">
                          <span className="w-2 h-2 bg-primary-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                          <span className="w-2 h-2 bg-primary-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                          <span className="w-2 h-2 bg-primary-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                        </div>
                        <span className="text-sm">{isArabic ? 'يكتب...' : 'Thinking...'}</span>
                      </div>
                    </div>
                  </div>
                )}

                <div ref={messagesEndRef} />
              </>
            )}
          </div>

          {/* Input Area */}
          <div className="border-t border-gray-200 dark:border-gray-700 bg-white/50 dark:bg-gray-900/50 backdrop-blur-sm">
            <ChatInput />
          </div>
        </div>
      </div>

      <KeyboardShortcutsHelp isOpen={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </div>
  );
}