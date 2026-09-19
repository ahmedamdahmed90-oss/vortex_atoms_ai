import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, act } from '@testing-library/react'
import { ErrorBoundary, withErrorBoundary } from '../ErrorBoundary'
import { Skeleton, SkeletonText, SkeletonCard, SkeletonChatMessage, SkeletonDashboardStats, SkeletonTabContent } from '../Skeleton'
import { ConnectionStatus, ErrorWithRetry } from '../ConnectionStatus'
import { ToastProvider, useToast } from '../Toast'
import { useSettingsStore } from '../../../stores/settingsStore'

const Bomb = ({ shouldThrow = true }: { shouldThrow?: boolean }) => {
  if (shouldThrow) throw new Error('boom')
  return <div>ok</div>
}

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    render(<ErrorBoundary><div>child</div></ErrorBoundary>)
    expect(screen.getByText('child')).toBeInTheDocument()
  })

  it('shows fallback UI when child throws', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<ErrorBoundary><Bomb /></ErrorBoundary>)
    expect(screen.getByText(/حدث خطأ غير متوقع|Something went wrong/)).toBeInTheDocument()
    spy.mockRestore()
  })

  it('retries and recovers when wrapped child stops throwing', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    let shouldThrow = true
    const DynamicBomb = () => {
      if (shouldThrow) throw new Error('boom2')
      return <div>recovered</div>
    }
    const { rerender } = render(<ErrorBoundary><DynamicBomb /></ErrorBoundary>)
    expect(screen.getByText(/حدث خطأ/)).toBeInTheDocument()
    shouldThrow = false
    fireEvent.click(screen.getByText(/إعادة المحاولة|Try Again/))
    rerender(<ErrorBoundary><DynamicBomb /></ErrorBoundary>)
    expect(screen.getByText('recovered')).toBeInTheDocument()
    spy.mockRestore()
  })

  it('renders custom fallback when provided', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<ErrorBoundary fallback={<div>custom-fallback</div>}><Bomb /></ErrorBoundary>)
    expect(screen.getByText('custom-fallback')).toBeInTheDocument()
    spy.mockRestore()
  })

  it('home button requests navigation to root', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<ErrorBoundary><Bomb /></ErrorBoundary>)
    fireEvent.click(screen.getByText(/الصفحة الرئيسية|Home Page/))
    expect(screen.getByText(/حدث خطأ غير متوقع|Something went wrong/)).toBeInTheDocument()
    spy.mockRestore()
  })

  it('withErrorBoundary wraps components with protection', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const Safe = withErrorBoundary(Bomb)
    const { unmount } = render(<Safe />)
    expect(screen.getByText(/حدث خطأ غير متوقع|Something went wrong/)).toBeInTheDocument()
    unmount()
    const Ok = withErrorBoundary(() => <div>fine-component</div>)
    render(<Ok />)
    expect(screen.getByText('fine-component')).toBeInTheDocument()
    spy.mockRestore()
  })

  it('renders english fallback when language is en', () => {
    useSettingsStore.setState({ language: 'en' } as never)
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(<ErrorBoundary><Bomb /></ErrorBoundary>)
    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
    expect(screen.getByText('Try Again')).toBeInTheDocument()
    expect(screen.getByText('Error Details (for developers)')).toBeInTheDocument()
    expect(screen.getByText('Home Page')).toBeInTheDocument()
    spy.mockRestore()
    useSettingsStore.setState({ language: 'ar' } as never)
  })
})

describe('Skeleton', () => {
  it('renders with default and custom variants', () => {
    const { container: c1 } = render(<Skeleton />)
    expect(c1.firstChild).toHaveClass('animate-pulse')
    const { container: c2 } = render(<Skeleton variant="circular" animation="none" width={20} height={20} />)
    expect(c2.firstChild).toHaveClass('rounded-full')
    expect(c2.firstChild).not.toHaveClass('animate-pulse')
    const el = c2.firstChild as HTMLElement
    expect(el.style.width).toBe('20px')
    expect(el.style.height).toBe('20px')
  })

  it('SkeletonText renders N lines with last at 60%', () => {
    const { container } = render(<SkeletonText lines={3} />)
    expect(container.querySelectorAll('[aria-hidden="true"]')).toHaveLength(3)
  })

  it('accepts string dimensions', () => {
    const { container } = render(<Skeleton width="50%" height="10px" />)
    const el = container.firstChild as HTMLElement
    expect(el.style.width).toBe('50%')
    expect(el.style.height).toBe('10px')
  })

  it('SkeletonCard and related composites render without crashing', () => {
    const { container: a } = render(<SkeletonCard />)
    expect(a.firstChild).toBeInTheDocument()
    const { container: b } = render(<SkeletonChatMessage />)
    expect(b.firstChild).toBeInTheDocument()
    const { container: c } = render(<SkeletonChatMessage isUser />)
    expect(c.firstChild).toBeInTheDocument()
    const { container: d } = render(<SkeletonDashboardStats />)
    expect(d.querySelectorAll('[aria-hidden="true"]').length).toBeGreaterThan(4)
    const { container: e } = render(<SkeletonTabContent />)
    expect(e.firstChild).toBeInTheDocument()
  })
})

describe('ConnectionStatus', () => {
  it('compact mode shows icon only with live region', () => {
    const { container } = render(<ConnectionStatus isOnline={false} isServerReachable={false} latency={null} compact />)
    expect(container.firstChild).toHaveAttribute('aria-live', 'polite')
  })

  it('shows offline, server unreachable, and connected states', () => {
    const { rerender } = render(<ConnectionStatus isOnline={false} isServerReachable={false} latency={null} />)
    expect(screen.getByText(/غير متصل|Offline/)).toBeInTheDocument()

    rerender(<ConnectionStatus isOnline isServerReachable={false} latency={null} />)
    expect(screen.getByText(/الخادم غير متاح|Server unreachable/)).toBeInTheDocument()

    rerender(<ConnectionStatus isOnline isServerReachable latency={42} />)
    expect(screen.getByText(/متصل|Connected/)).toBeInTheDocument()
    expect(screen.getByText(/\(42ms\)/)).toBeInTheDocument()
  })

  it('wires onRetry callback', () => {
    const onRetry = vi.fn()
    render(<ConnectionStatus isOnline isServerReachable={false} latency={null} onRetry={onRetry} />)
    fireEvent.click(screen.getByText(/إعادة المحاولة|Retry/))
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('renders english labels across states', () => {
    const onRetry = vi.fn()
    useSettingsStore.setState({ language: 'en' } as never)
    const { rerender } = render(<ConnectionStatus isOnline={false} isServerReachable={false} latency={null} />)
    expect(screen.getByText('Offline')).toBeInTheDocument()

    rerender(<ConnectionStatus isOnline isServerReachable={false} latency={null} />)
    expect(screen.getByText('Server unreachable')).toBeInTheDocument()

    rerender(<ConnectionStatus isOnline isServerReachable latency={42} onRetry={onRetry} />)
    expect(screen.getByText(/Connected/)).toBeInTheDocument()
    expect(screen.getByText(/\(42ms\)/)).toBeInTheDocument()
    fireEvent.click(screen.getByText('Retry'))
    expect(onRetry).toHaveBeenCalledTimes(1)
    useSettingsStore.setState({ language: 'ar' } as never)
  })

  it('shows spinner icon while checking', () => {
    const { container } = render(<ConnectionStatus isOnline isServerReachable latency={null} onRetry={() => {}} isChecking />)
    expect(container.querySelector('.animate-spin')).toBeInTheDocument()
  })
})

describe('ErrorWithRetry', () => {
  it('renders error title/message and retries', () => {
    const onRetry = vi.fn()
    render(<ErrorWithRetry error="فشل تحميل" onRetry={onRetry} title="خطأ مخصص" />)
    expect(screen.getByText('خطأ مخصص')).toBeInTheDocument()
    expect(screen.getByText('فشل تحميل')).toBeInTheDocument()
    fireEvent.click(screen.getByText(/إعادة المحاولة|Try Again/))
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('falls back to default titles', () => {
    const { rerender } = render(<ErrorWithRetry error="e" onRetry={() => {}} />)
    expect(screen.getByText('حدث خطأ')).toBeInTheDocument()
    useSettingsStore.setState({ language: 'en' } as never)
    rerender(<ErrorWithRetry error="e" onRetry={() => {}} />)
    expect(screen.getByText('An error occurred')).toBeInTheDocument()
    expect(screen.getByText('Try Again')).toBeInTheDocument()
    useSettingsStore.setState({ language: 'ar' } as never)
  })
})

function ToastFire({ t, title, msg, dur }: { t: 'success' | 'error' | 'info' | 'warning'; title: string; msg?: string; dur?: number }) {
  const { addToast } = useToast()
  return <button onClick={() => addToast({ type: t, title, ...(msg ? { message: msg } : {}), ...(dur !== undefined ? { duration: dur } : {}) })}>fire-{t}</button>
}

describe('Toast', () => {
  it('throws when used outside provider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => render(<ToastFire t="info" title="x" />)).toThrow('useToast must be used within a ToastProvider')
    spy.mockRestore()
  })

  it('renders toast with message and dismisses via close button', () => {
    render(<ToastProvider><ToastFire t="error" title="عنوان خطأ" msg="تفاصيل الخطأ" /></ToastProvider>)
    fireEvent.click(screen.getByText('fire-error'))
    const alert = screen.getByRole('alert')
    expect(alert).toHaveTextContent('عنوان خطأ')
    expect(alert).toHaveTextContent('تفاصيل الخطأ')
    fireEvent.click(screen.getByLabelText('إغلاق'))
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('renders all variants without message', () => {
    render(<ToastProvider><><ToastFire t="success" title="ok" /><ToastFire t="info" title="inf" /><ToastFire t="warning" title="warn" /></></ToastProvider>)
    fireEvent.click(screen.getByText('fire-success'))
    fireEvent.click(screen.getByText('fire-info'))
    fireEvent.click(screen.getByText('fire-warning'))
    expect(screen.getAllByRole('alert')).toHaveLength(3)
  })

  it('auto-dismisses toast after duration', async () => {
    vi.useFakeTimers()
    try {
      render(<ToastProvider><ToastFire t="info" title="temp-toast" /></ToastProvider>)
      fireEvent.click(screen.getByText('fire-info'))
      expect(screen.getByRole('alert')).toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(6000) })
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })

  it('keeps zero-duration toast until manually closed', async () => {
    vi.useFakeTimers()
    try {
      render(<ToastProvider><ToastFire t="info" title="sticky-toast" dur={0} /></ToastProvider>)
      fireEvent.click(screen.getByText('fire-info'))
      expect(screen.getByRole('alert')).toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(10000) })
      expect(screen.getByRole('alert')).toBeInTheDocument()
      fireEvent.click(screen.getByLabelText('إغلاق'))
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })
})
