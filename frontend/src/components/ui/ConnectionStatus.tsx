import { Wifi, WifiOff, ServerOff, RefreshCw, Loader2 } from 'lucide-react';
import { Button } from './Button';
import { useSettingsStore } from '../../stores/settingsStore';

interface ConnectionStatusProps {
  isOnline: boolean;
  isServerReachable: boolean;
  latency: number | null;
  onRetry?: () => void;
  isChecking?: boolean;
  compact?: boolean;
}

export function ConnectionStatus({
  isOnline,
  isServerReachable,
  latency,
  onRetry,
  isChecking = false,
  compact = false,
}: ConnectionStatusProps) {
  const { language } = useSettingsStore();
  const isArabic = language === 'ar';

  if (compact) {
    return (
      <div className="flex items-center gap-2" aria-live="polite">
        {!isOnline ? (
          <WifiOff className="h-4 w-4 text-red-500" />
        ) : !isServerReachable ? (
          <ServerOff className="h-4 w-4 text-yellow-500" />
        ) : (
          <Wifi className="h-4 w-4 text-green-500" />
        )}
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-800" aria-live="polite">
      <div className="flex items-center gap-2">
        {!isOnline ? (
          <>
            <WifiOff className="h-4 w-4 text-red-500" />
            <span className="text-sm text-red-600 dark:text-red-400">
              {isArabic ? 'غير متصل' : 'Offline'}
            </span>
          </>
        ) : !isServerReachable ? (
          <>
            <ServerOff className="h-4 w-4 text-yellow-500" />
            <span className="text-sm text-yellow-600 dark:text-yellow-400">
              {isArabic ? 'الخادم غير متاح' : 'Server unreachable'}
            </span>
          </>
        ) : (
          <>
            <Wifi className="h-4 w-4 text-green-500" />
            <span className="text-sm text-green-600 dark:text-green-400">
              {isArabic ? 'متصل' : 'Connected'}
              {latency && ` (${latency}ms)`}
            </span>
          </>
        )}
      </div>

      {onRetry && (
        <Button
          variant="ghost"
          size="sm"
          onClick={onRetry}
          loading={isChecking}
          leftIcon={isChecking ? <Loader2 className="h-3 w-3 animate-spin" /> : <RefreshCw className="h-3 w-3" />}
          className="ml-auto"
        >
          {isArabic ? 'إعادة المحاولة' : 'Retry'}
        </Button>
      )}
    </div>
  );
}

interface ErrorWithRetryProps {
  error: string;
  onRetry: () => void;
  title?: string;
}

export function ErrorWithRetry({ error, onRetry, title }: ErrorWithRetryProps) {
  const { language } = useSettingsStore();
  const isArabic = language === 'ar';

  return (
    <div className="flex flex-col items-center justify-center py-8 px-4 text-center" role="alert">
      <div className="p-3 rounded-full bg-red-100 dark:bg-red-900/30 mb-4">
        <ServerOff className="h-8 w-8 text-red-600 dark:text-red-400" />
      </div>
      <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
        {title || (isArabic ? 'حدث خطأ' : 'An error occurred')}
      </h3>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-4 max-w-md">
        {error}
      </p>
      <Button onClick={onRetry} leftIcon={<RefreshCw className="h-4 w-4" />}>
        {isArabic ? 'إعادة المحاولة' : 'Try Again'}
      </Button>
    </div>
  );
}