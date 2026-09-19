import { useCallback } from 'react';
import { useChatStore } from '../stores/chatStore';
import { useWebSocket } from './useWebSocket';
import { api } from '../services/api';


export function useChat() {
  const {
    messages,
    currentMessage,
    isStreaming,
    temperature,
    maxTokens,
    systemPrompt,
    selectedModel,
    tokensUsed,
    tokensPerSecond,
    error,
    addMessage,
    setCurrentMessage,
    setStreaming,
    setTokensUsed,
    setTokensPerSecond: _setTokensPerSecond,
    setError,
    handleWsEvent: _handleWsEvent,
    clearMessages,
  } = useChatStore();

  const { isConnected, sendGenerate } = useWebSocket();

  const sendMessage = useCallback(async () => {
    const state = useChatStore.getState()
    const message = state.currentMessage.trim()
    if (!message || state.isStreaming) return

    if (isConnected) {
      sendGenerate({
        prompt: message,
        max_tokens: state.maxTokens,
        temperature: state.temperature,
      })
      return
    }

    try {
      setError(null)
      setStreaming(true)

      const userMessage = { role: 'user' as const, content: message }
      const response = await api.chat({
        messages: [{ role: 'system', content: state.systemPrompt }, userMessage],
        max_tokens: state.maxTokens,
        temperature: state.temperature,
      })

      addMessage({
        id: `${Date.now()}-user`,
        role: 'user',
        content: message,
        timestamp: Date.now(),
      });

      addMessage({
        id: `${Date.now()}-assistant`,
        role: 'assistant',
        content: response.text,
        timestamp: Date.now(),
        tokensUsed: response.usage.total_tokens,
        tokensPerSecond: 0,
      });

      setTokensUsed(response.usage.total_tokens);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'فشل في إرسال الرسالة');
    } finally {
      setStreaming(false);
    }
  }, [isConnected, sendGenerate, addMessage, setError, setStreaming, setTokensUsed]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }, [sendMessage]);

  const regenerate = useCallback(async () => {
    const lastUserMessage = [...messages].reverse().find((m) => m.role === 'user');
    if (!lastUserMessage || isStreaming) return;

    setCurrentMessage(lastUserMessage.content);
    await sendMessage();
  }, [messages, isStreaming, setCurrentMessage, sendMessage]);

  const clearChat = useCallback(() => {
    clearMessages();
  }, [clearMessages]);

  return {
    messages,
    currentMessage,
    setCurrentMessage,
    isStreaming,
    isConnected,
    temperature,
    maxTokens,
    systemPrompt,
    selectedModel,
    tokensUsed,
    tokensPerSecond,
    error,
    sendMessage,
    handleKeyDown,
    regenerate,
    clearChat,
    useChatStore,
  };
}