import { clsx } from 'clsx';

interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'circular' | 'rectangular';
  width?: string | number;
  height?: string | number;
  animation?: 'pulse' | 'wave' | 'none';
}

export function Skeleton({
  className,
  variant = 'text',
  width,
  height,
  animation = 'pulse',
}: SkeletonProps) {
  const baseStyles = 'bg-gray-200 dark:bg-gray-700 rounded';

  const variants = {
    text: 'h-4 rounded',
    circular: 'rounded-full',
    rectangular: 'rounded-lg',
  };

  const animations = {
    pulse: 'animate-pulse',
    wave: 'animate-[wave_1.5s_ease-in-out_infinite]',
    none: '',
  };

  const styles = {
    width: width ? (typeof width === 'number' ? `${width}px` : width) : undefined,
    height: height ? (typeof height === 'number' ? `${height}px` : height) : undefined,
  };

  return (
    <div
      className={clsx(baseStyles, variants[variant], animations[animation], className)}
      style={styles}
      aria-hidden="true"
    />
  );
}

export function SkeletonText({ lines = 3, className }: { lines?: number; className?: string }) {
  return (
    <div className={clsx('space-y-3', className)}>
      {Array.from({ length: lines }, (_, i) => (
        <Skeleton key={i} variant="text" width={i === lines - 1 ? '60%' : '100%'} />
      ))}
    </div>
  );
}

export function SkeletonCard({ className }: { className?: string }) {
  return (
    <div className={clsx('space-y-4 p-6 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700', className)}>
      <Skeleton variant="circular" width={48} height={48} className="w-12 h-12" />
      <SkeletonText lines={3} />
      <div className="flex gap-2">
        <Skeleton variant="rectangular" width={80} height={32} />
        <Skeleton variant="rectangular" width={80} height={32} />
      </div>
    </div>
  );
}

export function SkeletonChatMessage({ isUser = false }: { isUser?: boolean }) {
  return (
    <div className={`flex items-start gap-3 animate-pulse ${isUser ? 'justify-end' : ''}`}>
      {!isUser && <Skeleton variant="circular" width={32} height={32} className="w-8 h-8 flex-shrink-0" />}
      <div className={`flex-1 max-w-[85%] ${isUser ? 'text-right' : 'text-left'}`}>
        <Skeleton variant="rectangular" className={`rounded-2xl p-4 ${isUser ? 'rounded-tr-none' : 'rounded-tl-none'}`} width="100%" height={60} />
        <Skeleton variant="text" width="40%" height={12} className={`mt-2 ${isUser ? 'ml-auto' : ''}`} />
      </div>
      {isUser && <Skeleton variant="circular" width={32} height={32} className="w-8 h-8 flex-shrink-0" />}
    </div>
  );
}

export function SkeletonDashboardStats() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      {Array.from({ length: 4 }, (_, i) => (
        <div key={i} className="p-4 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
          <Skeleton variant="circular" width={40} height={40} className="w-10 h-10 mb-3" />
          <Skeleton variant="text" width="40%" height={16} />
          <Skeleton variant="text" width="60%" height={24} className="mt-2" />
        </div>
      ))}
    </div>
  );
}

export function SkeletonTabContent() {
  return (
    <div className="space-y-6 animate-pulse">
      <SkeletonCard />
      <SkeletonCard />
      <SkeletonCard />
    </div>
  );
}