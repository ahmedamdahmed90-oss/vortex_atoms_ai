import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Message, WsEvent } from '../types';

interface ChatState {
  messages: Message[];
  currentMessage: string;
  isStreaming: boolean;
  streamingMessageId: string | null;
  streamingContent: string;
  tokensUsed: number;
  tokensPerSecond: number;
  temperature: number;
  maxTokens: number;
  systemPrompt: string;
  selectedModel: string;
  error: string | null;
  
  setMessages: (messages: Message[]) => void;
  addMessage: (message: Message) => void;
  updateMessage: (id: string, updates: Partial<Message>) => void;
  removeMessage: (id: string) => void;
  clearMessages: () => void;
  setCurrentMessage: (content: string) => void;
  setStreaming: (streaming: boolean) => void;
  setStreamingMessage: (id: string | null) => void;
  appendStreamingContent: (content: string) => void;
  setTokensUsed: (count: number) => void;
  setTokensPerSecond: (tps: number) => void;
  setTemperature: (temp: number) => void;
  setMaxTokens: (tokens: number) => void;
  setSystemPrompt: (prompt: string) => void;
  setSelectedModel: (model: string) => void;
  setError: (error: string | null) => void;
  
  handleWsEvent: (event: WsEvent) => void;
}

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      messages: [],
      currentMessage: '',
      isStreaming: false,
      streamingMessageId: null,
      streamingContent: '',
      tokensUsed: 0,
      tokensPerSecond: 0,
      temperature: 0.7,
      maxTokens: 512,
      systemPrompt: 'You are a helpful AI assistant.',
      selectedModel: 'qwen2.5-0.5b-instruct',
      error: null,

      setMessages: (messages) => set({ messages }),
      addMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),
      updateMessage: (id, updates) => set((state) => ({ messages: state.messages.map((m) => m.id === id ? { ...m, ...updates } : m) })),
      removeMessage: (id) => set((state) => ({ messages: state.messages.filter((m) => m.id !== id) })),
      clearMessages: () => set({ messages: [] }),
      setCurrentMessage: (content) => set({ currentMessage: content }),
      setStreaming: (streaming) => set({ isStreaming: streaming }),
      setStreamingMessage: (id) => set({ streamingMessageId: id }),
      appendStreamingContent: (content) => set((state) => ({ streamingContent: state.streamingContent + content })),
      setTokensUsed: (count) => set({ tokensUsed: count }),
      setTokensPerSecond: (tps) => set({ tokensPerSecond: tps }),
      setTemperature: (temp) => set({ temperature: temp }),
      setMaxTokens: (tokens) => set({ maxTokens: Math.min(512, Math.max(64, Math.round(tokens))) }),
      setSystemPrompt: (prompt) => set({ systemPrompt: prompt }),
      setSelectedModel: (model) => set({ selectedModel: model }),
      setError: (error) => set({ error }),

      handleWsEvent: (event) => {
        const state = get();
        switch (event.type) {
          case 'ready': {
            if (state.currentMessage.trim()) {
              const userMessage = { id: `${Date.now()}-user`, role: 'user' as const, content: state.currentMessage, timestamp: Date.now() };
              const assistantMessage = { id: `${Date.now()}-assistant`, role: 'assistant' as const, content: '', timestamp: Date.now(), isStreaming: true };
              set({ messages: [...state.messages, userMessage, assistantMessage], currentMessage: '', isStreaming: true, streamingMessageId: assistantMessage.id, streamingContent: '', tokensUsed: 0, tokensPerSecond: 0, error: null });
            }
            break;
          }
          case 'token': { if (state.streamingMessageId) set((s) => ({ streamingContent: s.streamingContent + event.text, messages: s.messages.map((m) => m.id === s.streamingMessageId ? { ...m, content: s.streamingContent + event.text } : m) })); break; }
          case 'batch': { if (state.streamingMessageId) { const chunk = event.events.map((e) => e.text).join(''); set((s) => ({ streamingContent: s.streamingContent + chunk, messages: s.messages.map((m) => m.id === s.streamingMessageId ? { ...m, content: s.streamingContent + chunk } : m) })); } break; }
          case 'done': { if (state.streamingMessageId) set((s) => ({ messages: s.messages.map((m) => m.id === s.streamingMessageId ? { ...m, content: event.full_text, isStreaming: false, tokensUsed: event.total_tokens, tokensPerSecond: event.tokens_per_second } : m), isStreaming: false, streamingMessageId: null, streamingContent: '', tokensUsed: event.total_tokens, tokensPerSecond: event.tokens_per_second })); break; }
          case 'error': { if (state.streamingMessageId) set((s) => ({ messages: s.messages.map((m) => m.id === s.streamingMessageId ? { ...m, content: `خطأ: ${event.message}`, isStreaming: false } : m), isStreaming: false, streamingMessageId: null, streamingContent: '', error: event.message })); break; }
        }
      },
    }),
    { name: 'vortex-chat-store', partialize: (state) => ({ temperature: state.temperature, maxTokens: state.maxTokens, systemPrompt: state.systemPrompt, selectedModel: state.selectedModel }) }
  )
);

export const chatStoreGetState = () => useChatStore.getState();