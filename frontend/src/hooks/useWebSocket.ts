import { useEffect, useCallback } from 'react';
import { useChatStore } from '../stores/chatStore';
import { wsService } from '../services/ws';
import { api } from '../services/api';
import type { WsGenerateRequest } from '../types';

export function useWebSocket() {
  const { handleWsEvent, setStreaming, isStreaming } = useChatStore();

  useEffect(() => {
    const unsubscribeEvent = wsService.onEvent(handleWsEvent);
    const unsubscribeConnection = wsService.onConnectionChange((connected) => {
      if (!connected && isStreaming) {
        setStreaming(false);
      }
    });

    // Fire-and-forget: warms the bearer-token cache so the socket below
    // (and its automatic reconnects) carry `?token=`. connect() itself stays
    // synchronous as before.
    api.health().catch(() => null);
    wsService.connect().catch(console.error);

    return () => {
      unsubscribeEvent();
      unsubscribeConnection();
    };
  }, [handleWsEvent, isStreaming, setStreaming]);

  const sendGenerate = useCallback((request: WsGenerateRequest) => {
    wsService.sendGenerate(request);
  }, []);

  const disconnect = useCallback(() => {
    wsService.disconnect();
  }, []);

  return {
    isConnected: wsService.isConnected,
    sendGenerate,
    disconnect,
  };
}