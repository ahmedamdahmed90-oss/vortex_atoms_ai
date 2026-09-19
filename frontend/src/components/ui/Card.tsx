import { forwardRef, type HTMLAttributes } from 'react';
import { clsx } from 'clsx';

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  hover?: boolean;
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, hover, padding = 'md', children, ...props }, ref) => {
    const paddings = { none: '', sm: 'p-4', md: 'p-6', lg: 'p-8' };
    return (
      <div ref={ref} className={clsx('bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden', hover && 'hover:shadow-md hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-200', paddings[padding], className)} {...props}>{children}</div>
    );
  }
);

interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {
  title: string;
  description?: string;
  action?: React.ReactNode;
  icon?: React.ReactNode;
}

export const CardHeader = ({ title, description, action, icon, className: _className, ...props }: CardHeaderProps) => (
  <div className="flex items-start justify-between gap-4 mb-4" {...props}>
    <div className="flex items-center gap-3">
      {icon && <div className="p-2 rounded-lg bg-primary-100 dark:bg-primary-900/30">{icon}</div>}
      <div>
        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
        {description && <p className="mt-0.5 text-sm text-gray-500 dark:text-gray-400">{description}</p>}
      </div>
    </div>
    {action && <div className="flex-shrink-0">{action}</div>}
  </div>
);

interface CardContentProps extends HTMLAttributes<HTMLDivElement> {}

export const CardContent = ({ children, className, ...props }: CardContentProps) => <div className={clsx(className)} {...props}>{children}</div>;

interface CardFooterProps extends HTMLAttributes<HTMLDivElement> {}

export const CardFooter = ({ children, className, ...props }: CardFooterProps) => <div className={clsx('mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 flex items-center gap-2', className)} {...props}>{children}</div>;