import { useState, useEffect, useCallback } from 'react';

interface NetworkStatus {
  isOnline: boolean;
  isServerReachable: boolean;
  latency: number | null;
  lastChecked: Date | null;
}

export function useNetworkStatus(checkInterval: number = 10000) {
  const [status, setStatus] = useState<NetworkStatus>({
    isOnline: navigator.onLine,
    isServerReachable: false,
    latency: null,
    lastChecked: null,
  });
  const [isChecking, setIsChecking] = useState(false);

  const checkServerConnection = useCallback(async () => {
    if (!navigator.onLine) {
      setStatus(prev => ({ ...prev, isOnline: false, isServerReachable: false }));
      return;
    }

    setIsChecking(true);
    const startTime = Date.now();

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch('http://127.0.0.1:8080/v1/health', {
        method: 'GET',
        signal: controller.signal,
      });

      clearTimeout(timeoutId);
      const latency = Date.now() - startTime;

      setStatus({
        isOnline: true,
        isServerReachable: response.ok,
        latency,
        lastChecked: new Date(),
      });
    } catch {
      setStatus({
        isOnline: navigator.onLine,
        isServerReachable: false,
        latency: null,
        lastChecked: new Date(),
      });
    } finally {
      setIsChecking(false);
    }
  }, []);

  useEffect(() => {
    const handleOnline = () => {
      setStatus(prev => ({ ...prev, isOnline: true }));
      checkServerConnection();
    };

    const handleOffline = () => {
      setStatus(prev => ({ ...prev, isOnline: false, isServerReachable: false }));
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    checkServerConnection();
    const interval = setInterval(checkServerConnection, checkInterval);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      clearInterval(interval);
    };
  }, [checkInterval, checkServerConnection]);

  return { ...status, isChecking, checkServerConnection };
}

export function useKeyboardShortcut(
  key: string,
  callback: () => void,
  options: {
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
    meta?: boolean;
    preventDefault?: boolean;
    enabled?: boolean;
  } = {}
) {
  const {
    ctrl = false,
    shift = false,
    alt = false,
    meta = false,
    preventDefault = true,
    enabled = true,
  } = options;

  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      const keyMatch = e.key.toLowerCase() === key.toLowerCase();
      const ctrlMatch = e.ctrlKey === ctrl;
      const shiftMatch = e.shiftKey === shift;
      const altMatch = e.altKey === alt;
      const metaMatch = e.metaKey === meta;

      if (keyMatch && ctrlMatch && shiftMatch && altMatch && metaMatch) {
        if (preventDefault) {
          e.preventDefault();
          e.stopPropagation();
        }
        callback();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [key, callback, ctrl, shift, alt, meta, preventDefault, enabled]);
}