import type { WsEvent, WsGenerateRequest } from '../types';
import { getApiTokenSync } from './api';

type WsEventHandler = (event: WsEvent) => void;
type ConnectionHandler = (connected: boolean, reason?: string) => void;

interface QueuedMessage {
  data: string;
  resolve: () => void;
  reject: (error: Error) => void;
}

export class WebSocketService {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private baseReconnectDelay = 1000;
  private maxReconnectDelay = 30000;
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  private handlers: Set<WsEventHandler> = new Set();
  private connectionHandlers: Set<ConnectionHandler> = new Set();
  private isIntentionalClose = false;
  private messageQueue: QueuedMessage[] = [];
  private pendingPings = new Map<number, ReturnType<typeof setTimeout>>();
  private pingId = 0;

  constructor(url?: string) {
    this.url =
      url ||
      import.meta.env.VITE_WS_URL ||
      (import.meta.env.PROD
        ? typeof location !== 'undefined'
          ? `ws://${location.host}/ws`
          : 'ws://localhost:8080/ws'
        : 'ws://localhost:8080/ws');
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        resolve();
        return;
      }

      this.isIntentionalClose = false;
      // Bearer auth for WS: browsers cannot set headers on upgrade, so the
      // token travels as a query param (same trust as the Authorization
      // header; the URL never leaves the loopback machine in default setup).
      // Read synchronously: connect() must construct the socket immediately.
      const token = getApiTokenSync();
      const url =
        token && !/[?&]token=/.test(this.url)
          ? `${this.url}${this.url.includes('?') ? '&' : '?'}token=${encodeURIComponent(token)}`
          : this.url;
      this.ws = new WebSocket(url);

      const connectTimeout = setTimeout(() => {
        // Fires only if onopen/onerror did not clear it first, so the
        // socket is guaranteed to still be non-open here.
        this.ws?.close();
        reject(new Error('Connection timeout'));
      }, 10000);

      this.ws.onopen = () => {
        clearTimeout(connectTimeout);
        this.reconnectAttempts = 0;
        this.flushQueue();
        this.startHeartbeat();
        this.notifyConnection(true, 'connected');
        resolve();
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as WsEvent & { type?: string; id?: number; timestamp?: number };
          if (data.type === 'pong') {
            this.handlePong(data);
            return;
          }
          this.notifyHandlers(data);
        } catch (error) {
          console.error('Failed to parse WS message:', error);
        }
      };

      this.ws.onclose = (event) => {
        clearTimeout(connectTimeout);
        this.stopHeartbeat();
        this.notifyConnection(false, event.reason || 'disconnected');

        if (!this.isIntentionalClose) {
          this.scheduleReconnect();
        }
      };

      this.ws.onerror = (error) => {
        if (this.ws?.readyState !== WebSocket.OPEN) {
          clearTimeout(connectTimeout);
          reject(error);
        }
      };
    });
  }

  private handlePong(data: WsEvent & { id?: number; timestamp?: number }): void {
    if (data.id !== undefined) {
      const timeout = this.pendingPings.get(data.id);
      if (timeout) {
        clearTimeout(timeout);
        this.pendingPings.delete(data.id);
      }
    }
  }

  private scheduleReconnect(): void {
    this.reconnectAttempts++;
    const delay = Math.min(
      this.baseReconnectDelay * Math.pow(1.5, this.reconnectAttempts - 1) + Math.random() * 1000,
      this.maxReconnectDelay
    );

    console.log(`[WS] Scheduling reconnect attempt ${this.reconnectAttempts} in ${Math.round(delay)}ms`);

    setTimeout(() => {
      if (!this.isIntentionalClose) {
        this.connect().catch((error) => {
          console.error('[WS] Reconnect failed:', error);
        });
      }
    }, delay);
  }

  private startHeartbeat(): void {
    this.heartbeatInterval = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.sendPing();
      }
    }, 25000);
  }

  private sendPing(): void {
    if (this.ws?.readyState !== WebSocket.OPEN) return;

    const id = ++this.pingId;
    const timestamp = Date.now();

    const timeout = setTimeout(() => {
      this.pendingPings.delete(id);
      console.warn('[WS] Ping timeout, closing connection');
      this.ws?.close(1000, 'Ping timeout');
    }, 10000);

    this.pendingPings.set(id, timeout);

    this.ws.send(JSON.stringify({ type: 'ping', id, timestamp }));
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
    this.pendingPings.forEach((timeout) => clearTimeout(timeout));
    this.pendingPings.clear();
  }

  private flushQueue(): void {
    while (this.messageQueue.length > 0) {
      const { data, resolve, reject } = this.messageQueue.shift()!;
      try {
        this.ws?.send(data);
        resolve();
      } catch (error) {
        reject(error as Error);
      }
    }
  }

  private queueMessage(data: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.messageQueue.push({ data, resolve, reject });
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.flushQueue();
      }
    });
  }

  sendGenerate(request: WsGenerateRequest): void {
    const message = JSON.stringify(request);
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(message);
    } else {
      this.queueMessage(message).catch((error) => {
        console.error('[WS] Failed to queue message:', error);
      });
    }
  }

  disconnect(): void {
    this.isIntentionalClose = true;
    this.stopHeartbeat();
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    this.messageQueue.forEach(({ reject }) => {
      reject(new Error('WebSocket disconnected'));
    });
    this.messageQueue = [];
  }

  onEvent(handler: WsEventHandler): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  onConnectionChange(handler: ConnectionHandler): () => void {
    this.connectionHandlers.add(handler);
    return () => this.connectionHandlers.delete(handler);
  }

  private notifyHandlers(event: WsEvent): void {
    this.handlers.forEach((handler) => {
      try {
        handler(event);
      } catch (error) {
        console.error('[WS] Handler error:', error);
      }
    });
  }

  private notifyConnection(connected: boolean, reason?: string): void {
    this.connectionHandlers.forEach((handler) => {
      try {
        handler(connected, reason);
      } catch (error) {
        console.error('[WS] Connection handler error:', error);
      }
    });
  }

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  get connectionState(): 'connecting' | 'open' | 'closing' | 'closed' {
    if (!this.ws) return 'closed';
    switch (this.ws.readyState) {
      case WebSocket.CONNECTING: return 'connecting';
      case WebSocket.OPEN: return 'open';
      case WebSocket.CLOSING: return 'closing';
      case WebSocket.CLOSED: return 'closed';
      default: return 'closed';
    }
  }
}

export const wsService = new WebSocketService();