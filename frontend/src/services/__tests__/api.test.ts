import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { api, ApiError, fetchWithTimeout, setTokens, resetAuth, getApiTokenSync } from '../api'

const okResponse = (data: unknown) => ({
  ok: true,
  status: 200,
  json: async () => data,
})

const errResponse = (status: number, data: unknown = {}) => ({
  ok: false,
  status,
  json: async () => data,
})

describe('api client', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    vi.restoreAllMocks()
    globalThis.fetch = vi.fn()
    // Pre-seed tokens so tests exercise request logic, not the bootstrap.
    setTokens('test-api-token', 'test-admin-token')
  })

  afterEach(() => {
    resetAuth()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('performs GET health and returns parsed data', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ status: 'ok', version: '0.1.0' })
    )

    const data = await api.health()

    expect(data).toEqual({ status: 'ok', version: '0.1.0' })
    expect(globalThis.fetch).toHaveBeenCalledTimes(1)
    const [url, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(String(url)).toMatch(/\/v1\/health$/)
    expect((init as RequestInit).method).toBeUndefined()
    expect((init as RequestInit).headers).toMatchObject({ 'Content-Type': 'application/json' })
  })

  it('reads performance knobs from the admin endpoint', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ ws_coalesce_ms: 50, tier_policy: 'auto' })
    )

    const data = await api.admin.performance.get()

    expect(data).toEqual({ ws_coalesce_ms: 50, tier_policy: 'auto' })
    const [url] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(String(url)).toMatch(/\/v1\/admin\/performance$/)
  })

  it('posts a coalescing-window patch and returns the saved knobs', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ ws_coalesce_ms: 120, tier_policy: 'auto' })
    )

    const data = await api.admin.performance.update({ ws_coalesce_ms: 120 })

    expect(data.ws_coalesce_ms).toBe(120)
    const [url, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(String(url)).toMatch(/\/v1\/admin\/performance$/)
    expect((init as RequestInit).method).toBe('POST')
    expect((init as RequestInit).body).toBe(JSON.stringify({ ws_coalesce_ms: 120 }))
  })

  it('throws non-retryable ApiError on 400 without retrying', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      errResponse(400, { error: 'bad request' })
    )

    await expect(fetchWithTimeout('/x', { retries: 3 })).rejects.toMatchObject({
      name: 'ApiError',
      status: 400,
      message: 'bad request',
      retryable: false,
    })
    expect(globalThis.fetch).toHaveBeenCalledTimes(1)
  })

  it('retries a retryable 500 and succeeds on next attempt', async () => {
    vi.useFakeTimers()
    try {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(errResponse(500))
        .mockResolvedValueOnce(okResponse({ fine: true }))

      const pending = fetchWithTimeout<{ fine: boolean }>('/x', { retries: 1, timeout: 5000 })
      const assertion = expect(pending).resolves.toEqual({ fine: true })
      await vi.advanceTimersByTimeAsync(10000)
      await assertion
      expect(globalThis.fetch).toHaveBeenCalledTimes(2)
    } finally {
      vi.useRealTimers()
    }
  })

  it('exhausts retries on persistent 500 and throws ApiError', async () => {
    vi.useFakeTimers()
    try {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(errResponse(500))

      const pending = fetchWithTimeout('/x', { retries: 2 })
      const assertion = expect(pending).rejects.toMatchObject({
        name: 'ApiError',
        status: 500,
        retryable: true,
      })
      await vi.advanceTimersByTimeAsync(30000)
      await assertion
      expect(globalThis.fetch).toHaveBeenCalledTimes(3)
    } finally {
      vi.useRealTimers()
    }
  })

  it('maps AbortError to timeout ApiError 408', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValue(
      new DOMException('aborted', 'AbortError')
    )

    await expect(fetchWithTimeout('/x', { retries: 0 })).rejects.toMatchObject({
      name: 'ApiError',
      status: 408,
      message: 'Request timeout',
      retryable: true,
    })
  })

  it('wraps network errors into ApiError status 0', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValue(
      new TypeError('Failed to fetch')
    )

    await expect(fetchWithTimeout('/x', { retries: 0 })).rejects.toMatchObject({
      name: 'ApiError',
      status: 0,
      message: 'Failed to fetch',
      retryable: true,
    })
  })

  it('serializes POST body through createApiMethod', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ response: 'hi' })
    )

    const payload = { messages: [{ role: 'user' as const, content: 'hello' }] }
    const data = await api.chat(payload)

    expect(data).toEqual({ response: 'hi' })
    const [url, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(String(url)).toMatch(/\/v1\/chat$/)
    expect((init as RequestInit).method).toBe('POST')
    expect((init as RequestInit).body).toBe(JSON.stringify(payload))
  })

  it('exposes ApiError constructor metadata', () => {
    const err = new ApiError('oops', 418, { x: 1 }, true)
    expect(err.status).toBe(418)
    expect(err.data).toEqual({ x: 1 })
    expect(err.retryable).toBe(true)
    expect(err.toString()).toContain('oops')
  })

  it('falls back to empty object when body is not JSON', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => { throw new Error('no json') },
    })

    const data = await fetchWithTimeout('/x', { retries: 0 })
    expect(data).toEqual({})
  })

  it('aborts on timeout and maps to 408', async () => {
    vi.useFakeTimers()
    try {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockImplementation(
        (_url: unknown, init?: { signal?: AbortSignal }) =>
          new Promise((_resolve, reject) => {
            init?.signal?.addEventListener('abort', () =>
              reject(new DOMException('aborted', 'AbortError'))
            )
          })
      )

      const pending = fetchWithTimeout('/slow', { timeout: 50, retries: 0 })
      const assertion = expect(pending).rejects.toMatchObject({ status: 408, message: 'Request timeout' })
      await vi.advanceTimersByTimeAsync(100)
      await assertion
    } finally {
      vi.useRealTimers()
    }
  })

  it('calls device, models and tools list endpoints', async () => {
    const f = globalThis.fetch as ReturnType<typeof vi.fn>
    f.mockResolvedValueOnce(okResponse({ device: 'cpu' }))
      .mockResolvedValueOnce(okResponse({ architecture: 'q' }))
      .mockResolvedValueOnce(okResponse({ tools: [] }))

    await api.device()
    await api.models()
    await api.tools.list()

    const urls = f.mock.calls.map((c) => String(c[0]))
    expect(urls[0]).toMatch(/\/v1\/device$/)
    expect(urls[1]).toMatch(/\/v1\/models$/)
    expect(urls[2]).toMatch(/\/v1\/tools$/)
  })

  it('retries network errors and succeeds on next attempt', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>)
      .mockRejectedValueOnce(new TypeError('Failed to fetch'))
      .mockResolvedValueOnce(okResponse({ recovered: true }))

    const data = await fetchWithTimeout<{ recovered: boolean }>('/x', { retries: 1, timeout: 5000 })

    expect(data).toEqual({ recovered: true })
    expect(globalThis.fetch).toHaveBeenCalledTimes(2)
  }, 15000)

  it('labels non-Error rejections as Network error', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValue('string-fail')

    await expect(fetchWithTimeout('/x', { retries: 0 })).rejects.toMatchObject({
      name: 'ApiError',
      status: 0,
      message: 'Network error',
    })
  })

  it('omits body for GET-style api methods without payload', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ response: 'empty' })
    )

    await api.batch()

    const [, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect((init as RequestInit).body).toBeUndefined()
  })

  it('sends the user bearer token on regular endpoints', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ status: 'ok' })
    )

    await api.health()

    const [, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: 'Bearer test-api-token',
    })
  })

  it('sends the admin bearer token on privileged endpoints', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      okResponse({ status: 'ok' })
    )

    await api.admin.status()

    const [url, init] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(String(url)).toMatch(/\/v1\/admin\/status$/)
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: 'Bearer test-admin-token',
    })
  })

  it('does not retry failed POST requests (no side-effect replay)', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      errResponse(500, { error: 'boom' })
    )

    await expect(
      api.chat({ messages: [{ role: 'user', content: 'hi' }] })
    ).rejects.toMatchObject({ status: 500 })
    expect(globalThis.fetch).toHaveBeenCalledTimes(1)
  })

  it('does not retry 503 even on GET (fail-fast back-off)', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      errResponse(503, { error: 'engine busy' })
    )

    await expect(fetchWithTimeout('/x', { retries: 3 })).rejects.toMatchObject({
      status: 503,
    })
    expect(globalThis.fetch).toHaveBeenCalledTimes(1)
  })

  it('bootstraps tokens from /auth/bootstrap on first unauthenticated call', async () => {
    resetAuth()
    const f = globalThis.fetch as ReturnType<typeof vi.fn>
    f.mockResolvedValueOnce(
      okResponse({ auth_enabled: true, api_token: 'boot-api', admin_token: 'boot-admin' })
    ).mockResolvedValueOnce(okResponse({ status: 'ok' }))

    await api.health()

    expect(String(f.mock.calls[0][0])).toMatch(/\/auth\/bootstrap$/)
    expect(f.mock.calls[1][1]).toMatchObject({
      headers: expect.objectContaining({ Authorization: 'Bearer boot-api' }),
    })
  })

  it('getApiTokenSync returns seeded token', () => {
    setTokens('my-token', 'my-admin')
    expect(getApiTokenSync()).toBe('my-token')
  })

  it('ensureTokens skips bootstrap when already done', async () => {
    setTokens('cached', 'cached-admin')
    const f = globalThis.fetch as ReturnType<typeof vi.fn>
    f.mockResolvedValueOnce(okResponse({ status: 'ok' }))
    await api.health()
    expect(f).toHaveBeenCalledTimes(1)
    expect(String(f.mock.calls[0][0])).toMatch(/\/v1\/health$/)
    expect(String(f.mock.calls[0][0])).not.toMatch(/\/auth\/bootstrap/)
  })
})
