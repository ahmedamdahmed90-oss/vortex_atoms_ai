import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { fireEvent } from '@testing-library/react'
import { useNetworkStatus, useKeyboardShortcut } from '../useNetworkStatus'

describe('useNetworkStatus', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('reports server reachable with latency after successful health check', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ ok: true })

    const { result } = renderHook(() => useNetworkStatus(60000))

    await waitFor(() => expect(result.current.isServerReachable).toBe(true))
    expect(result.current.isOnline).toBe(true)
    expect(result.current.latency).toBeGreaterThanOrEqual(0)
    expect(result.current.lastChecked).toBeInstanceOf(Date)
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://127.0.0.1:8080/v1/health',
      expect.objectContaining({ method: 'GET' })
    )
  })

  it('marks server unreachable when fetch fails', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      new TypeError('Failed to fetch')
    )

    const { result } = renderHook(() => useNetworkStatus(60000))

    await waitFor(() => expect(result.current.isChecking).toBe(false))
    expect(result.current.isServerReachable).toBe(false)
    expect(result.current.latency).toBeNull()
  })

  it('manual checkServerConnection refreshes state', async () => {
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true })

    const { result } = renderHook(() => useNetworkStatus(60000))
    await waitFor(() => expect(result.current.isServerReachable).toBe(true))

    act(() => {
      result.current.checkServerConnection()
    })
    await waitFor(() => expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThanOrEqual(2))
  })

  it('reports offline without fetching when navigator is offline', async () => {
    const onLine = vi.spyOn(window.navigator, 'onLine', 'get').mockReturnValue(false)
    try {
      const { result } = renderHook(() => useNetworkStatus(60000))
      await waitFor(() => expect(result.current.isOnline).toBe(false))
      expect(result.current.isServerReachable).toBe(false)
      expect(globalThis.fetch).not.toHaveBeenCalled()
    } finally {
      onLine.mockRestore()
    }
  })

  it('reacts to browser online/offline events', async () => {    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true })

    const { result } = renderHook(() => useNetworkStatus(60000))
    await waitFor(() => expect(result.current.isServerReachable).toBe(true))

    act(() => {
      window.dispatchEvent(new Event('offline'))
    })
    expect(result.current.isOnline).toBe(false)
    expect(result.current.isServerReachable).toBe(false)

    act(() => {
      window.dispatchEvent(new Event('online'))
    })
    expect(result.current.isOnline).toBe(true)
    await waitFor(() => expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThanOrEqual(2))
  })

  it('aborts hanging health checks after timeout', async () => {
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
      const { result } = renderHook(() => useNetworkStatus(60000))
      await act(async () => { await vi.advanceTimersByTimeAsync(6000) })
      expect(result.current.isServerReachable).toBe(false)
      expect(result.current.isChecking).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('useKeyboardShortcut', () => {
  it('fires callback on matching key combo', () => {
    const cb = vi.fn()
    renderHook(() => useKeyboardShortcut('k', cb, { ctrl: true }))

    fireEvent.keyDown(document, { key: 'k', ctrlKey: true })
    expect(cb).toHaveBeenCalledTimes(1)
  })

  it('ignores non-matching combos', () => {
    const cb = vi.fn()
    renderHook(() => useKeyboardShortcut('k', cb, { ctrl: true }))

    fireEvent.keyDown(document, { key: 'k' })
    fireEvent.keyDown(document, { key: 'j', ctrlKey: true })
    expect(cb).not.toHaveBeenCalled()
  })

  it('is case-insensitive for shifted letters', () => {
    const cb = vi.fn()
    renderHook(() => useKeyboardShortcut('d', cb, { ctrl: true, shift: true }))

    fireEvent.keyDown(document, { key: 'D', ctrlKey: true, shiftKey: true })
    expect(cb).toHaveBeenCalledTimes(1)
  })

  it('does nothing when disabled and cleans up on unmount', () => {
    const cb = vi.fn()
    const { unmount } = renderHook(() => useKeyboardShortcut('k', cb, { enabled: false }))
    fireEvent.keyDown(document, { key: 'k' })
    expect(cb).not.toHaveBeenCalled()

    unmount()
    fireEvent.keyDown(document, { key: 'k', ctrlKey: false })
    expect(cb).not.toHaveBeenCalled()
  })

  it('supports meta modifier and optional preventDefault', () => {
    const cb = vi.fn()
    renderHook(() => useKeyboardShortcut('s', cb, { meta: true, preventDefault: false }))

    fireEvent.keyDown(document, { key: 's', metaKey: true })
    expect(cb).toHaveBeenCalledTimes(1)

    fireEvent.keyDown(document, { key: 's' })
    expect(cb).toHaveBeenCalledTimes(1)
  })
})
