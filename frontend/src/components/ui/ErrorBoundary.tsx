import { Component, type ReactNode } from 'react';
import { AlertTriangle, RefreshCw, Home } from 'lucide-react';
import { Button } from './Button';
import { useSettingsStore } from '../../stores/settingsStore';

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, error: null, errorInfo: null };

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    this.setState({ error, errorInfo });
    console.error('ErrorBoundary caught:', error, errorInfo);
    this.props.onError?.(error, errorInfo);
  }

  handleRetry = (): void => {
    this.setState({ hasError: false, error: null, errorInfo: null });
  };

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      const { language } = useSettingsStore.getState();
      const isArabic = language === 'ar';

      return (
        <div className="min-h-screen flex items-center justify-center p-4 bg-gray-50 dark:bg-surface-dark">
          <div className="max-w-md w-full text-center">
            <div className="p-3 rounded-full bg-red-100 dark:bg-red-900/30 inline-flex mb-6">
              <AlertTriangle className="h-12 w-12 text-red-600 dark:text-red-400" />
            </div>
            <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-3">
              {isArabic ? 'حدث خطأ غير متوقع' : 'Something went wrong'}
            </h2>
            <p className="text-gray-500 dark:text-gray-400 mb-6">
              {isArabic
                ? 'نعتذر عن هذا الخطأ. يمكنك المحاولة مرة أخرى أو العودة للصفحة الرئيسية.'
                : 'We apologize for this error. You can try again or go back to the home page.'}
            </p>
            {this.state.error && (
              <details className="mb-6 text-left p-4 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm">
                <summary className="font-medium text-gray-700 dark:text-gray-300 mb-2 cursor-pointer">
                  {isArabic ? 'تفاصيل الخطأ (للمطورين)' : 'Error Details (for developers)'}
                </summary>
                <pre className="text-xs text-gray-600 dark:text-gray-400 overflow-auto max-h-40">
                  {this.state.error.toString()}
                  {this.state.errorInfo?.componentStack}
                </pre>
              </details>
            )}
            <div className="flex flex-col sm:flex-row gap-3 justify-center">
              <Button onClick={this.handleRetry} leftIcon={<RefreshCw className="h-4 w-4" />}>
                {isArabic ? 'إعادة المحاولة' : 'Try Again'}
              </Button>
              <Button variant="secondary" onClick={() => window.location.href = '/'} leftIcon={<Home className="h-4 w-4" />}>
                {isArabic ? 'الصفحة الرئيسية' : 'Home Page'}
              </Button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// eslint-disable-next-line react/only-export-components
export function withErrorBoundary<P extends object>(
  WrappedComponent: React.ComponentType<P>,
  errorBoundaryProps?: Omit<ErrorBoundaryProps, 'children'>
): React.FC<P> {
  return function WithErrorBoundary(props: P) {
    return (
      <ErrorBoundary {...errorBoundaryProps}>
        <WrappedComponent {...props} />
      </ErrorBoundary>
    );
  };
}