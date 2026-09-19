import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { SecurityTab } from '../SecurityTab'
import { setTokens, resetAuth } from '../../../services/api'

const fullStatus = {
  auth_enabled: true,
  allowed_origins: ['https://ops.local'],
  knowledge_dirs: ['knowledge'],
  allow_custom_models: false,
  max_prompt_chars: 32768,
  max_batch_prompts: 32,
  max_import_chars: 200000,
  temperature_range: [0, 2],
  rate_limit_per_10s: 600,
  read_only: false,
  admin_loopback_only: true,
  sessions_retention_days: 30,
  requests_total: 11,
  denied_401: 1,
  denied_403: 2,
  denied_429: 3,
  uptime_seconds: 60,
}

const fullMetrics = {
  requests_total: 100,
  denied_401: 1,
  denied_403: 2,
  denied_429: 3,
  uptime_seconds: 60,
}

type Failure = { status: number; body: unknown } | Error

// Mount GETs are inexhaustible; failures are injected per-URL so remounts
// and retries under load can never starve a call.
function mockAdminApi(opts: {
  status?: unknown
  auditEntries?: string[]
  metrics?: unknown
  rotate?: unknown
  reload?: unknown
  purgeRemoved?: number
  failures?: Record<string, Failure>
} = {}) {
  const calls: Array<{ url: string; method: string; body?: string }> = []
  const failures = opts.failures ?? {}
  const failResponse = (f: Failure) => {
    if (f instanceof Error) throw f
    return { ok: false, status: f.status, json: async () => f.body }
  }
  globalThis.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
    const u = String(url)
    const method = ((init?.method ?? 'GET') as string).toUpperCase()
    if (method !== 'GET') calls.push({ url: u, method, body: String(init?.body ?? '') })
    for (const [key, f] of Object.entries(failures)) {
      if (u.includes(key)) return failResponse(f as Failure)
    }
    if (u.includes('/admin/status')) {
      return { ok: true, status: 200, json: async () => opts.status ?? fullStatus }
    }
    if (u.includes('/admin/audit')) {
      return { ok: true, status: 200, json: async () => ({ entries: opts.auditEntries ?? ['a1', 'a2'] }) }
    }
    if (u.includes('/admin/metrics')) {
      return { ok: true, status: 200, json: async () => opts.metrics ?? fullMetrics }
    }
    if (u.includes('/admin/rotate')) {
      return { ok: true, status: 200, json: async () => opts.rotate ?? { status: 'ok', admin_token: 'vxa_new' } }
    }
    if (u.includes('/admin/reload')) {
      return { ok: true, status: 200, json: async () => opts.reload ?? { status: 'ok' } }
    }
    if (u.includes('/admin/sessions/purge')) {
      return { ok: true, status: 200, json: async () => ({ status: 'ok', removed: opts.purgeRemoved ?? 4 }) }
    }
    return { ok: true, status: 200, json: async () => ({}) }
  }) as unknown as typeof fetch
  return { calls }
}

describe('SecurityTab', () => {
  const origFetch = globalThis.fetch
  const origConfirm = window.confirm

  beforeEach(() => {
    vi.clearAllMocks()
    setTokens('t', 'a')
  })

  afterEach(() => {
    globalThis.fetch = origFetch
    window.confirm = origConfirm
    resetAuth()
    vi.restoreAllMocks()
  })

  it('loads protection card with badges, metrics and caps', async () => {
    mockAdminApi()
    render(<SecurityTab />)
    expect(screen.getByText('الأمان')).toBeInTheDocument()
    expect(await screen.findByText('المصادقة مفعّلة (user / admin)')).toBeInTheDocument()
    expect(screen.getByText('النماذج من القائمة المعتمدة فقط')).toBeInTheDocument()
    expect(screen.getByText('الإدارة loopback فقط')).toBeInTheDocument()
    expect(screen.getByText('100')).toBeInTheDocument()
    expect(screen.getByText('32768')).toBeInTheDocument()
    expect(screen.getByText(/أصول CORS إضافية/)).toBeInTheDocument()
    expect(screen.getByText('a1')).toBeInTheDocument()
  })

  it('renders warning badges for lax configuration', async () => {
    mockAdminApi({
      status: {
        ...fullStatus,
        auth_enabled: false,
        allow_custom_models: true,
        read_only: true,
        admin_loopback_only: false,
        allowed_origins: [],
      },
      auditEntries: [],
    })
    render(<SecurityTab />)
    expect(await screen.findByText('المصادقة معطّلة — ثقة محلية فقط')).toBeInTheDocument()
    expect(screen.getByText('نماذج مخصصة مسموحة')).toBeInTheDocument()
    expect(screen.getByText('وضع القراءة فقط')).toBeInTheDocument()
    expect(screen.getByText('الإدارة مسموحة عن بُعد')).toBeInTheDocument()
    expect(screen.getByText('لا إدخالات بعد')).toBeInTheDocument()
  })

  it('shows admin hint on 403 and retries on demand', async () => {
    const first = vi.fn(async (url: unknown) => {
      if (String(url).includes('/admin/')) {
        return { ok: false, status: 403, json: async () => ({ error: 'forbidden' }) }
      }
      return { ok: true, status: 200, json: async () => ({}) }
    })
    globalThis.fetch = first as unknown as typeof fetch
    render(<SecurityTab />)
    expect(await screen.findByText(/تحتاج صلاحية الإدارة/)).toBeInTheDocument()

    const second = vi.fn(async (url: unknown) => {
      const u = String(url)
      if (u.includes('/admin/status')) return { ok: true, status: 200, json: async () => fullStatus }
      if (u.includes('/admin/audit')) return { ok: true, status: 200, json: async () => ({ entries: [] }) }
      if (u.includes('/admin/metrics')) return { ok: true, status: 200, json: async () => fullMetrics }
      return { ok: true, status: 200, json: async () => ({}) }
    })
    globalThis.fetch = second as unknown as typeof fetch
    fireEvent.click(screen.getByRole('button', { name: /إعادة المحاولة/ }))
    expect(await screen.findByText('المصادقة مفعّلة (user / admin)')).toBeInTheDocument()
  })

  it('shows connection error on non-403 failure', async () => {
    // 400 (non-retryable) exercises the generic branch deterministically;
    // retryable 5xx backoff is covered in api.test.ts.
    mockAdminApi({ failures: { '/admin/status': { status: 400, body: { error: 'bad' } } } })
    render(<SecurityTab />)
    expect(await screen.findByText('تعذّر الاتصال بالخادم')).toBeInTheDocument()
  })

  it('rotate posts, confirms and refreshes', async () => {
    window.confirm = vi.fn(() => true)
    const { calls } = mockAdminApi()
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تدوير توكن الإدارة/ }))
    expect(await screen.findByText(/تم التدوير وحُفظ تلقائياً/)).toBeInTheDocument()
    const post = calls.find((c) => c.url.includes('/admin/rotate'))
    expect(post?.method).toBe('POST')
  })

  it('rotate does nothing when confirmation is cancelled', async () => {
    window.confirm = vi.fn(() => false)
    const { calls } = mockAdminApi()
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تدوير توكن الإدارة/ }))
    await waitFor(() => expect(calls.filter((c) => c.url.includes('/admin/rotate'))).toHaveLength(0))
    expect(screen.queryByText(/تم التدوير/)).not.toBeInTheDocument()
  })

  it('rotate surfaces failure', async () => {
    window.confirm = vi.fn(() => true)
    mockAdminApi({ failures: { '/admin/rotate': new Error('down') } })
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تدوير توكن الإدارة/ }))
    expect(await screen.findByText('فشل التدوير')).toBeInTheDocument()
  })

  it('purge posts and reports removed count', async () => {
    window.confirm = vi.fn(() => true)
    const { calls } = mockAdminApi({ purgeRemoved: 7 })
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تنظيف الجلسات القديمة/ }))
    expect(await screen.findByText(/حُذفت 7 جلسة/)).toBeInTheDocument()
    const post = calls.find((c) => c.url.includes('/admin/sessions/purge'))
    expect(post?.method).toBe('POST')
  })

  it('purge does nothing when confirmation is cancelled', async () => {
    window.confirm = vi.fn(() => false)
    const { calls } = mockAdminApi()
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تنظيف الجلسات القديمة/ }))
    await waitFor(() => expect(calls.filter((c) => c.url.includes('/admin/sessions/purge'))).toHaveLength(0))
  })

  it('purge surfaces failure', async () => {
    window.confirm = vi.fn(() => true)
    mockAdminApi({ failures: { '/admin/sessions/purge': new Error('down') } })
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /تنظيف الجلسات القديمة/ }))
    expect(await screen.findByText('فشل التنظيف')).toBeInTheDocument()
  })

  it('reload posts and reload failure surfaces', async () => {
    const { calls } = mockAdminApi()
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /إعادة تحميل الإعدادات/ }))
    await waitFor(() => expect(calls.filter((c) => c.url.includes('/admin/reload'))).toHaveLength(1))
  })

  it('reload failure surfaces error', async () => {
    mockAdminApi({ failures: { '/admin/reload': new Error('down') } })
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    fireEvent.click(screen.getByRole('button', { name: /إعادة تحميل الإعدادات/ }))
    expect(await screen.findByText('فشل إعادة التحميل')).toBeInTheDocument()
  })

  it('refresh button re-fetches status', async () => {
    mockAdminApi()
    render(<SecurityTab />)
    await screen.findByText('المصادقة مفعّلة (user / admin)')
    const before = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.filter(([u]) =>
      String(u).includes('/admin/status'),
    ).length
    fireEvent.click(screen.getByRole('button', { name: /^تحديث/ }))
    await waitFor(() =>
      expect(
        (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.filter(([u]) =>
          String(u).includes('/admin/status'),
        ).length,
      ).toBeGreaterThan(before),
    )
  })

  it('clears confirmation flags after their timeouts', async () => {
    window.confirm = vi.fn(() => true)
    mockAdminApi({ purgeRemoved: 7 })
    vi.useFakeTimers()
    try {
      render(<SecurityTab />)
      // No findBy* under fake timers: pump microtasks until settled.
      for (let i = 0; i < 40 && !screen.queryByText('المصادقة مفعّلة (user / admin)'); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      expect(screen.getByText('المصادقة مفعّلة (user / admin)')).toBeInTheDocument()
      fireEvent.click(screen.getByRole('button', { name: /تدوير توكن الإدارة/ }))
      for (let i = 0; i < 40 && !screen.queryByText(/تم التدوير وحُفظ تلقائياً/); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      fireEvent.click(screen.getByRole('button', { name: /تنظيف الجلسات القديمة/ }))
      for (let i = 0; i < 40 && !screen.queryByText(/حُذفت 7 جلسة/); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      await act(async () => { await vi.advanceTimersByTimeAsync(8000); });
      expect(screen.queryByText(/تم التدوير وحُفظ تلقائياً/)).not.toBeInTheDocument()
      expect(screen.queryByText(/حُذفت 7 جلسة/)).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })
})
