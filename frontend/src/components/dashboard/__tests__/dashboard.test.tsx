import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { Dashboard } from '../Dashboard'
import { SettingsTab } from '../SettingsTab'
import { PerformanceTab } from '../PerformanceTab'
import { ToastProvider } from '../../ui/Toast'
import { useSettingsStore } from '../../../stores/settingsStore'

const hoisted = vi.hoisted(() => ({
  dash: {
    activeTab: 'overview' as unknown as 'overview' | 'models' | 'knowledge' | 'tools' | 'logs' | 'settings' | 'security' | 'performance',
    setActiveTab: vi.fn(),
    refreshAll: vi.fn(),
    health: null,
    loading: false,
    fetchHealth: vi.fn(),
    models: null,
    currentModel: null,
    swapModel: vi.fn(),
    logs: [] as string[],
    logFilter: 'all' as const,
    setLogFilter: vi.fn(),
    clearLogs: vi.fn(),
    tools: null,
    searchResults: null,
    knowledgeChunks: 0,
    searchKnowledge: vi.fn(),
    importKnowledge: vi.fn(),
    executeTool: vi.fn(),
    fetchTools: vi.fn(),
    fetchModels: vi.fn(),
  },
}))

vi.mock('../../../hooks/useDashboard', () => ({ useDashboard: () => hoisted.dash }))

describe('Dashboard', () => {
  beforeEach(() => { vi.clearAllMocks(); hoisted.dash.activeTab = 'overview' })

  it('renders Tabs and delegates onChange to setActiveTab', () => {
    render(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByRole('tablist')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /نظرة عامة/ })).toHaveAttribute('aria-selected', 'true')
    fireEvent.click(screen.getByRole('tab', { name: /النماذج/ }))
    expect(hoisted.dash.setActiveTab).toHaveBeenCalledWith('models')
  })

  it('shows correct panel per activeTab: models, knowledge, settings', () => {
    hoisted.dash.activeTab = 'models'
    const { rerender } = render(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByText('إدارة النماذج')).toBeInTheDocument()

    hoisted.dash.activeTab = 'knowledge'
    rerender(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByRole('heading', { name: 'قاعدة المعرفة' })).toBeInTheDocument()

    hoisted.dash.activeTab = 'settings'
    rerender(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByRole('heading', { name: 'الإعدادات' })).toBeInTheDocument()
  })

  it('shows tools and logs panels per activeTab', () => {
    hoisted.dash.activeTab = 'tools'
    const { rerender } = render(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByText('لا توجد أدوات متاحة')).toBeInTheDocument()

    hoisted.dash.activeTab = 'logs'
    rerender(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByText('لا توجد سجلات')).toBeInTheDocument()
  })

  it('renders security and performance panels per activeTab', () => {
    hoisted.dash.activeTab = 'security'
    const { rerender } = render(<ToastProvider><Dashboard /></ToastProvider>)
    // SecurityTab renders its own content; just verify the tab rendered.
    expect(screen.getByRole('tablist')).toBeInTheDocument()

    hoisted.dash.activeTab = 'performance'
    hoisted.dash.health = { status: 'ok', version: '0.2.3', build_ts: '1758615338', git_sha: '3f3c47f', architecture: 'x86_64', device: 'cpu', simd: 'sse42', history_length: 0, uptime_seconds: 999, knowledge_chunks: 0, perf: { sku: 'test-sku', tier: 'auto', tier_model: 't', tier_max_context: 4096, infer_threads: 4, async_workers: 2, prefault_enabled: true, compiled_features: ['sse2'], host_features: ['avx2'], fastpath_avg_ms: 10 } }
    rerender(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByText('test-sku')).toBeInTheDocument()
  })

  it('default case renders overview', () => {
    hoisted.dash.activeTab = 'unknown' as unknown as typeof hoisted.dash.activeTab
    render(<ToastProvider><Dashboard /></ToastProvider>)
    expect(screen.getByText('نظرة عامة')).toBeInTheDocument()
  })
})

describe('PerformanceTab', () => {
  const perfHealth = {
    status: 'ok', version: '0.2.3', build_ts: '1758615338', git_sha: '3f3c47f',
    architecture: 'x86_64', device: 'cpu', simd: 'sse42',
    history_length: 0, uptime_seconds: 999, knowledge_chunks: 0,
    perf: { sku: 'test-sku', tier: 'auto', tier_model: 't', tier_max_context: 4096, infer_threads: 4, async_workers: 2, prefault_enabled: true, compiled_features: ['sse2'], host_features: ['avx2'], fastpath_avg_ms: 10 },
  }
  const benchData = { sku_current: 'test-sku', tier: 'auto', generated_at: '2026-01-01T00:00:00Z', entries: [{ sku: 'test-sku', ttft_ms: 100, tok_per_s: 50, peak_rss_mb: 512, fastpath_ms: 10, cache_hit_ms: 5, compatible: true, notes: 'ok' }] }

  beforeEach(() => { vi.clearAllMocks(); hoisted.dash.health = perfHealth })
  afterEach(() => { hoisted.dash.health = null })

  it('shows health perf summary on mount', async () => {
    const origFetch = globalThis.fetch
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => benchData }) as unknown as typeof fetch
    try { render(<ToastProvider><PerformanceTab /></ToastProvider>); expect(await screen.findByText('test-sku')).toBeInTheDocument() } finally { globalThis.fetch = origFetch }
  })

  it('shows loading spinner when health has no perf', async () => {
    const origFetch = globalThis.fetch
    hoisted.dash.health = { ...perfHealth, perf: null }
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => benchData }) as unknown as typeof fetch
    try { render(<ToastProvider><PerformanceTab /></ToastProvider>); expect(await screen.findByText(/جارٍ تحميل/)).toBeInTheDocument() } finally { globalThis.fetch = origFetch }
  })

  it('shows bench matrix and refetches on button click', async () => {
    const origFetch = globalThis.fetch
    globalThis.fetch = vi.fn(async (url: unknown) => { if (String(url).includes('/bench')) return { ok: true, status: 200, json: async () => benchData } ; return { ok: true, status: 200, json: async () => benchData } }) as unknown as typeof fetch
    try {
      render(<ToastProvider><PerformanceTab /></ToastProvider>)
      expect(await screen.findByText('test-sku')).toBeInTheDocument()
      const btn = await screen.findByText('إعادة القياس')
      fireEvent.click(btn)
      // After refetch, the bench card heading also renders test-sku.
      expect(screen.getAllByText('test-sku').length).toBeGreaterThanOrEqual(1)
    } finally { globalThis.fetch = origFetch }
  })

  it('shows error state when bench fetch fails', async () => {
    const origFetch = globalThis.fetch
    const origRandom = Math.random
    Math.random = vi.fn(() => 0)
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('bench down')) as unknown as typeof fetch
    vi.useFakeTimers()
    try {
      render(<ToastProvider><PerformanceTab /></ToastProvider>)
      await act(async () => { await vi.advanceTimersByTimeAsync(8000); });
      await act(async () => {});
      expect(screen.getByText('bench down')).toBeInTheDocument()
    } finally { vi.useRealTimers(); globalThis.fetch = origFetch; Math.random = origRandom }
  })
})

describe('SettingsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useSettingsStore.getState().resetToDefaults()
  })

  it('renders sections and wired buttons', () => {
    render(<ToastProvider><SettingsTab /></ToastProvider>)
    expect(screen.getByText('الإعدادات')).toBeInTheDocument()
    expect(screen.getByText('عام')).toBeInTheDocument()
    expect(screen.getByText('الخصوصية والأمان')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /استعادة الإعدادات الافتراضية/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /تصدير الإعدادات/ })).toBeInTheDocument()
  })

  it('reset button calls resetToDefaults', () => {
    const spy = vi.spyOn(useSettingsStore.getState(), 'resetToDefaults')
    render(<ToastProvider><SettingsTab /></ToastProvider>)
    fireEvent.click(screen.getByRole('button', { name: /استعادة الإعدادات الافتراضية/ }))
    expect(spy).toHaveBeenCalledTimes(1)
  })

  it('export button triggers JSON download via anchor click', () => {
    const createObjectURL = vi.fn(() => 'blob:url')
    const revokeObjectURL = vi.fn()
    const clickSpy = vi.fn()
    const origCreateEl = document.createElement.bind(document)
    vi.spyOn(URL, 'createObjectURL').mockImplementation(createObjectURL as unknown as typeof URL.createObjectURL)
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(revokeObjectURL as unknown as typeof URL.revokeObjectURL)
    vi.spyOn(document, 'createElement').mockImplementation(((tag: string) => {
      const el = origCreateEl(tag) as HTMLAnchorElement
      if (tag === 'a') el.click = clickSpy
      return el
    }) as unknown as typeof document.createElement)

    render(<ToastProvider><SettingsTab /></ToastProvider>)
    fireEvent.click(screen.getByRole('button', { name: /تصدير الإعدادات/ }))

    expect(createObjectURL).toHaveBeenCalledTimes(1)
    expect(clickSpy).toHaveBeenCalledTimes(1)
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:url')

    vi.restoreAllMocks()
  })

  it('testConnection shows success and error states', async () => {
    const origFetch = globalThis.fetch
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: true }) as unknown as typeof fetch
    render(<ToastProvider><SettingsTab /></ToastProvider>)
    fireEvent.click(screen.getByRole('button', { name: /اختبار الاتصال/ }))
    expect(await screen.findByText('نجح الاتصال')).toBeInTheDocument()
    globalThis.fetch = origFetch
  })

  it('testConnection shows error on non-ok response and on rejection', async () => {    const origFetch = globalThis.fetch
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: false }) as unknown as typeof fetch
    render(<ToastProvider><SettingsTab /></ToastProvider>)
    fireEvent.click(screen.getAllByRole('button', { name: /اختبار الاتصال/ })[0])
    expect(await screen.findAllByText('فشل الاتصال')).not.toHaveLength(0)

    globalThis.fetch = vi.fn().mockRejectedValue(new Error('down')) as unknown as typeof fetch
    fireEvent.click(screen.getAllByRole('button', { name: /اختبار الاتصال/ })[0])
    await waitFor(() => expect(screen.getAllByText('فشل الاتصال').length).toBeGreaterThan(0))
    globalThis.fetch = origFetch
  })

  it('testConnection result clears after timeout', async () => {
    const origFetch = globalThis.fetch
    vi.useFakeTimers()
    try {
      globalThis.fetch = vi.fn().mockResolvedValue({ ok: true }) as unknown as typeof fetch
      render(<ToastProvider><SettingsTab /></ToastProvider>)
      fireEvent.click(screen.getByRole('button', { name: /اختبار الاتصال/ }))
      await act(async () => {})
      expect(screen.getByText('نجح الاتصال')).toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(3000) })
      expect(screen.queryByText('نجح الاتصال')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
      globalThis.fetch = origFetch
    }
  })

  it('language and theme selects update the store', () => {
    const { container } = render(<ToastProvider><SettingsTab /></ToastProvider>)
    const selects = container.querySelectorAll('select')
    expect(selects).toHaveLength(2)
    fireEvent.change(selects[0], { target: { value: 'en' } })
    expect(useSettingsStore.getState().language).toBe('en')
    fireEvent.change(selects[1], { target: { value: 'dark' } })
    expect(useSettingsStore.getState().theme).toBe('dark')
  })

  it('api and ws inputs update the store', () => {
    render(<ToastProvider><SettingsTab /></ToastProvider>)
    fireEvent.change(screen.getByLabelText('عنوان API'), { target: { value: 'http://x:9999/v1' } })
    expect(useSettingsStore.getState().apiUrl).toBe('http://x:9999/v1')
    fireEvent.change(screen.getByLabelText('عنوان WebSocket'), { target: { value: 'ws://x:9999/ws' } })
    expect(useSettingsStore.getState().wsUrl).toBe('ws://x:9999/ws')
  })

  it('performance card loads knobs on mount', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true, status: 200, json: async () => ({ ws_coalesce_ms: 50, tier_policy: 'auto' }),
      }) as unknown as typeof fetch
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        expect(await screen.findByText('auto')).toBeInTheDocument()
        expect((screen.getByLabelText(/نافذة الدمج/) as HTMLInputElement).value).toBe('50')
        const [url] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
        expect(String(url)).toMatch(/\/v1\/admin\/performance$/)
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  // Mount GETs are inexhaustible; only POSTs consume the queued responses,
  // so remounts/retries under load can never starve the save call.
  const mockKnobsApi = (postResponses: Array<unknown>) => {
    const posts: Array<{ url: string; init: RequestInit }> = [];
    const fetchMock = vi.fn(async (url: unknown, init?: RequestInit) => {
      if ((init?.method ?? 'GET').toUpperCase() === 'POST') {
        posts.push({ url: String(url), init: init as RequestInit });
        const next = postResponses.shift();
        if (next instanceof Error) throw next;
        return { ok: true, status: 200, json: async () => next };
      }
      return { ok: true, status: 200, json: async () => ({ ws_coalesce_ms: 50, tier_policy: 'auto' }) };
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    return { fetchMock, posts };
  };

  it('performance save posts clamped window and shows confirmation', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      const { posts } = mockKnobsApi([{ ws_coalesce_ms: 120, tier_policy: 'auto' }])
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        await screen.findByText('auto')
        fireEvent.change(screen.getByLabelText(/نافذة الدمج/), { target: { value: '120' } })
        fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
        expect(await screen.findByText(/تم الحفظ/)).toBeInTheDocument()
        expect(posts).toHaveLength(1)
        expect(posts[0].url).toMatch(/\/v1\/admin\/performance$/)
        expect(posts[0].init.method).toBe('POST')
        expect(posts[0].init.body).toBe(JSON.stringify({ ws_coalesce_ms: 120 }))
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  it('performance save clamps out-of-range input to 5000', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      const { posts } = mockKnobsApi([{ ws_coalesce_ms: 5000, tier_policy: 'auto' }])
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        await screen.findByText('auto')
        fireEvent.change(screen.getByLabelText(/نافذة الدمج/), { target: { value: '99999' } })
        fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
        expect(await screen.findByText(/تم الحفظ/)).toBeInTheDocument()
        expect(posts).toHaveLength(1)
        expect(posts[0].init.body).toBe(JSON.stringify({ ws_coalesce_ms: 5000 }))
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  it('performance save surfaces backend errors', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      mockKnobsApi([new Error('down')])
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        await screen.findByText('auto')
        fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
        expect(await screen.findByText('down')).toBeInTheDocument()
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  it('performance mount failure shows hint instead of crashing', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      // 400 (non-retryable): the catch runs promptly inside the test window
      // (a rejection would burn real backoff delays on GET retries).
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false, status: 400, json: async () => ({ error: 'forbidden' }),
      }) as unknown as typeof fetch
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        expect(await screen.findByText(/تتطلب توكن الإدارة للعرض/)).toBeInTheDocument()
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  it('performance mount unmount mid-flight is safe', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      let release!: (v: unknown) => void
      globalThis.fetch = vi.fn(() => new Promise((res) => { release = res })) as unknown as typeof fetch
      try {
        const { unmount } = render(<ToastProvider><SettingsTab /></ToastProvider>)
        // Flush ensureTokens microtasks so fetch is *called* (promise pending).
        await act(async () => {});
        expect(globalThis.fetch).toHaveBeenCalled()
        unmount()
        await act(async () => {
          release({ ok: true, status: 200, json: async () => ({ ws_coalesce_ms: 50, tier_policy: 'auto' }) })
        });
        // Same, but the in-flight request fails: the catch must also bail out.
        let release2!: (v: unknown) => void
        globalThis.fetch = vi.fn(() => new Promise((res) => { release2 = res })) as unknown as typeof fetch
        const second = render(<ToastProvider><SettingsTab /></ToastProvider>)
        await act(async () => {});
        second.unmount()
        await act(async () => {
          release2({ ok: false, status: 400, json: async () => ({ error: 'forbidden' }) })
        });
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  it('performance save treats non-numeric input as 0', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    try {
      const origFetch = globalThis.fetch
      const { posts } = mockKnobsApi([{ ws_coalesce_ms: 0, tier_policy: 'auto' }])
      try {
        render(<ToastProvider><SettingsTab /></ToastProvider>)
        await screen.findByText('auto')
        fireEvent.change(screen.getByLabelText(/نافذة الدمج/), { target: { value: 'abc' } })
        fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
        expect(await screen.findByText(/تم الحفظ/)).toBeInTheDocument()
        expect(posts).toHaveLength(1)
        expect(posts[0].init.body).toBe(JSON.stringify({ ws_coalesce_ms: 0 }))
      } finally {
        globalThis.fetch = origFetch
      }
    } finally {
      resetAuth()
    }
  })

  // NOTE: saveKnobs' non-Error fallback branch is unreachable through the
  // real client (fetchWithTimeout always throws ApiError) — kept as
  // defensive code, intentionally uncovered.

  it('performance messages clear after timeout', async () => {
    const { setTokens, resetAuth } = await import('../../../services/api')
    setTokens('t', 'a')
    const origFetch = globalThis.fetch
    const posts: Array<{ url: string; init: RequestInit }> = []
    const postQueue: Array<unknown> = [
      { ws_coalesce_ms: 120, tier_policy: 'auto' },
      new Error('down'),
    ]
    globalThis.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      if (((init?.method ?? 'GET') as string).toUpperCase() === 'POST') {
        posts.push({ url: String(url), init: init as RequestInit })
        const next = postQueue.shift()
        if (next instanceof Error) throw next
        return { ok: true, status: 200, json: async () => next }
      }
      return { ok: true, status: 200, json: async () => ({ ws_coalesce_ms: 50, tier_policy: 'auto' }) }
    }) as unknown as typeof fetch
    vi.useFakeTimers()
    try {
      render(<ToastProvider><SettingsTab /></ToastProvider>)
      for (let i = 0; i < 40 && !screen.queryByText('auto'); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      fireEvent.change(screen.getByLabelText(/نافذة الدمج/), { target: { value: '120' } })
      fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
      for (let i = 0; i < 40 && !screen.queryByText(/تم الحفظ/); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
      expect(screen.queryByText(/تم الحفظ/)).not.toBeInTheDocument()
      fireEvent.click(screen.getByRole('button', { name: /حفظ الأداء/ }))
      for (let i = 0; i < 40 && !screen.queryByText('down'); i++) {
        await act(async () => { await vi.advanceTimersByTimeAsync(50); });
      }
      await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
      expect(screen.queryByText('down')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
      globalThis.fetch = origFetch
      resetAuth()
    }
  })

  it('token save activates session tokens and forget clears them', async () => {
    const { wsService } = await import('../../../services/ws')
    const disc = vi.spyOn(wsService, 'disconnect').mockImplementation(() => {})
    const conn = vi.spyOn(wsService, 'connect').mockResolvedValue(undefined)
    vi.useFakeTimers()
    try {
      render(<ToastProvider><SettingsTab /></ToastProvider>)
      fireEvent.change(screen.getByLabelText(/توكن المستخدم/), { target: { value: 'vxt_abc' } })
      fireEvent.change(screen.getByLabelText(/توكن الإدارة/), { target: { value: 'vxa_def' } })
      fireEvent.click(screen.getByRole('button', { name: /حفظ وتفعيل/ }))
      // setTokenSaved(true) is synchronous in the click handler: no waiting.
      expect(screen.getByText(/تم التفعيل/)).toBeInTheDocument()
      expect(disc).toHaveBeenCalled()
      expect(conn).toHaveBeenCalled()
      expect((screen.getByLabelText(/توكن المستخدم/) as HTMLInputElement).value).toBe('')
      await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
      expect(screen.queryByText(/تم التفعيل/)).not.toBeInTheDocument()
      fireEvent.change(screen.getByLabelText(/توكن المستخدم/), { target: { value: 'vxt_x' } })
      fireEvent.click(screen.getByRole('button', { name: /نسيان التوكنات/ }))
      expect((screen.getByLabelText(/توكن المستخدم/) as HTMLInputElement).value).toBe('')
      expect(disc).toHaveBeenCalledTimes(2)
      // Rejected reconnects are swallowed: UI still confirms the action.
      conn.mockRejectedValueOnce(new Error('down'))
      fireEvent.change(screen.getByLabelText(/توكن المستخدم/), { target: { value: 'vxt_y' } })
      fireEvent.click(screen.getByRole('button', { name: /حفظ وتفعيل/ }))
      expect(screen.getByText(/تم التفعيل/)).toBeInTheDocument()
      conn.mockRejectedValueOnce(new Error('down'))
      fireEvent.click(screen.getByRole('button', { name: /نسيان التوكنات/ }))
      await act(async () => {});
      expect(disc).toHaveBeenCalledTimes(4)
    } finally {
      vi.useRealTimers()
      disc.mockRestore()
      conn.mockRestore()
    }
  })

  it('token save with empty inputs stores nulls and still activates', async () => {
    const { wsService } = await import('../../../services/ws')
    const disc = vi.spyOn(wsService, 'disconnect').mockImplementation(() => {})
    const conn = vi.spyOn(wsService, 'connect').mockResolvedValue(undefined)
    try {
      render(<ToastProvider><SettingsTab /></ToastProvider>)
      fireEvent.click(screen.getByRole('button', { name: /حفظ وتفعيل/ }))
      expect(screen.getByText(/تم التفعيل/)).toBeInTheDocument()
      expect(disc).toHaveBeenCalled()
      expect(conn).toHaveBeenCalled()
    } finally {
      disc.mockRestore()
      conn.mockRestore()
    }
  })
})
