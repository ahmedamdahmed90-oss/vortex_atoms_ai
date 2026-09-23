import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { WebSocketService } from '../ws'

describe('WebSocketService', () => {
  let svc: WebSocketService

  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    svc = new WebSocketService('ws://test/ws')
  })

  afterEach(() => {
    svc.disconnect()
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  const openConnection = async () => {
    const p = svc.connect()
    const ws = (svc as unknown as { ws: MockWs }).ws
    ws.readyState = 1
    ws.onopen?.(new Event('open'))
    await p
    return ws
  }

  it('connects and becomes open', async () => {
    const p = svc.connect()
    const raw = (svc as unknown as { ws: MockWs }).ws
    expect(svc.connectionState).toBe('connecting')
    raw.readyState = 1
    raw.onopen?.(new Event('open'))
    await p
    expect(svc.isConnected).toBe(true)
    expect(svc.connectionState).toBe('open')
  })

  it('resolves immediately if already open', async () => {
    await openConnection()
    await expect(svc.connect()).resolves.toBeUndefined()
  })

  it('rejects on connection timeout', async () => {
    const p = svc.connect()
    vi.advanceTimersByTime(10000)
    await expect(p).rejects.toThrow('Connection timeout')
  })

  it('forwards non-pong events to handlers and isolates handler errors', async () => {
    const raw = await openConnection()
    const h1 = vi.fn()
    const hBad = vi.fn(() => { throw new Error('bad') })
    const h2 = vi.fn()
    svc.onEvent(h1)
    svc.onEvent(hBad)
    svc.onEvent(h2)

    raw.onmessage?.({ data: JSON.stringify({ type: 'token', text: 'hi' }) } as MessageEvent)

    expect(h1).toHaveBeenCalledWith(expect.objectContaining({ type: 'token' }))
    expect(h2).toHaveBeenCalledTimes(1)
  })

  it('does not forward pong but clears pending ping timeout', async () => {
    const raw = await openConnection()
    const handler = vi.fn()
    svc.onEvent(handler)

    vi.advanceTimersByTime(25000)
    expect((raw.send as ReturnType<typeof vi.fn>).mock.calls.some(c => String(c[0]).includes('"type":"ping"'))).toBe(true)
    const lastCall = (raw.send as ReturnType<typeof vi.fn>).mock.calls.at(-1)![0] as string
    const { id } = JSON.parse(lastCall)

    const before = handler.mock.calls.length
    raw.onmessage?.({ data: JSON.stringify({ type: 'pong', id }) } as MessageEvent)
    expect(handler).toHaveBeenCalledTimes(before)
  })

  it('queues messages while disconnected and flushes on open', async () => {
    svc.sendGenerate({ type: 'generate', prompt: 'hello' } as never)
    expect((svc as unknown as { messageQueue: unknown[] }).messageQueue).toHaveLength(1)

    const p = svc.connect()
    const raw = (svc as unknown as { ws: MockWs }).ws
    raw.readyState = 1
    raw.onopen?.(new Event('open'))
    await p
    expect((svc as unknown as { messageQueue: unknown[] }).messageQueue).toHaveLength(0)
    expect(raw.send).toHaveBeenCalledWith(expect.stringContaining('hello'))
  })

  it('disconnect rejects queued messages and stops heartbeat', async () => {
    svc.sendGenerate({ type: 'generate', prompt: 'queued' } as never)
    svc.disconnect()
    expect(svc.isConnected).toBe(false)
    expect(svc.connectionState).toBe('closed')
    expect((svc as unknown as { messageQueue: unknown[] }).messageQueue).toHaveLength(0)
  })

  it('notifies connection handlers and supports unsubscribe', async () => {
    const cb = vi.fn()
    const off = svc.onConnectionChange(cb)
    const raw = await openConnection()
    expect(cb).toHaveBeenCalledWith(true, 'connected')
    off()
    raw.readyState = 3
    raw.onclose?.({ reason: 'bye', code: 1000 } as unknown as CloseEvent)
    expect(cb).toHaveBeenCalledTimes(1)
  })

  it('exposes connectionState for each readyState', () => {
    expect(svc.connectionState).toBe('closed')
    const p = svc.connect()
    expect(svc.connectionState).toBe('connecting')
    const raw = (svc as unknown as { ws: MockWs }).ws
    raw.readyState = 1
    raw.onopen?.(new Event('open'))
    return p.then(() => {
      expect(svc.connectionState).toBe('open')
      raw.readyState = 3
      raw.onclose?.({ reason: '', code: 1000 } as unknown as CloseEvent)
      expect(svc.connectionState).toBe('closed')
    })
  })

  it('onEvent unsubscribe removes handler', async () => {
    const raw = await openConnection()
    const h = vi.fn()
    const off = svc.onEvent(h)
    off()
    raw.onmessage?.({ data: JSON.stringify({ type: 'token', text: 'x' }) } as MessageEvent)
    expect(h).not.toHaveBeenCalled()
  })

  it('ignores malformed messages with an error log', async () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {})
    const raw = await openConnection()
    raw.onmessage?.({ data: '{not-json' } as MessageEvent)
    expect(err).toHaveBeenCalledWith(expect.stringContaining('Failed to parse WS message'), expect.anything())
    err.mockRestore()
  })

  it('rejects connect on error before open and ignores errors while open', async () => {
    const p = svc.connect()
    const raw = (svc as unknown as { ws: MockWs }).ws
    raw.onerror?.(new Event('error'))
    await expect(p).rejects.toBeDefined()

    const p2 = svc.connect()
    const raw2 = (svc as unknown as { ws: MockWs }).ws
    raw2.readyState = 1
    raw2.onerror?.(new Event('error'))
    raw2.onopen?.(new Event('open'))
    await p2
    expect(svc.isConnected).toBe(true)
  })

  it('reconnects after unexpected close and logs when retry fails', async () => {
    const log = vi.spyOn(console, 'debug').mockImplementation(() => {})
    const err = vi.spyOn(console, 'error').mockImplementation(() => {})
    const raw = await openConnection()
    raw.readyState = 3
    raw.onclose?.({ reason: '', code: 1006 } as unknown as CloseEvent)
    expect(log).toHaveBeenCalledWith(expect.stringContaining('Scheduling reconnect'))
    expect(vi.getTimerCount()).toBe(1)

    await vi.advanceTimersByTimeAsync(3000)
    const mid = (svc as unknown as { ws: unknown }).ws
    expect(mid).not.toBe(raw)

    await vi.advanceTimersByTimeAsync(30000)
    expect(err).toHaveBeenCalledWith(expect.stringContaining('Reconnect failed'), expect.anything())
    log.mockRestore()
    err.mockRestore()
  })

  it('closes connection on ping timeout', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const raw = await openConnection()
    ;(svc as unknown as { sendPing: () => void }).sendPing()
    expect((svc as unknown as { pendingPings: Map<number, unknown> }).pendingPings.size).toBe(1)

    vi.advanceTimersByTime(10000)
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('Ping timeout'))
    expect(raw.close).toHaveBeenCalledWith(1000, 'Ping timeout')
    warn.mockRestore()
  })

  it('clears pending pings on disconnect', async () => {
    await openConnection()
    ;(svc as unknown as { sendPing: () => void }).sendPing()
    svc.disconnect()
    expect((svc as unknown as { pendingPings: Map<number, unknown> }).pendingPings.size).toBe(0)
  })

  it('sendPing no-ops without open connection', () => {
    ;(svc as unknown as { sendPing: () => void }).sendPing()
    expect((svc as unknown as { pendingPings: Map<number, unknown> }).pendingPings.size).toBe(0)
  })

  it('flushes queued messages immediately when already open', async () => {
    const raw = await openConnection()
    await (svc as unknown as { queueMessage: (d: string) => Promise<void> }).queueMessage('direct')
    expect(raw.send).toHaveBeenCalledWith('direct')
  })

  it('rejects queued messages when send throws during flush', async () => {
    const qp = (svc as unknown as { queueMessage: (d: string) => Promise<void> }).queueMessage('boom')
    const assertion = expect(qp).rejects.toThrow('send fail')
    const p = svc.connect()
    const raw = (svc as unknown as { ws: MockWs }).ws
    ;(raw.send as ReturnType<typeof vi.fn>).mockImplementation(() => { throw new Error('send fail') })
    raw.readyState = 1
    raw.onopen?.(new Event('open'))
    await p
    await assertion
  })

  it('sends generate requests directly when open', async () => {
    const raw = await openConnection()
    svc.sendGenerate({ type: 'generate', prompt: 'hi' } as never)
    expect(raw.send).toHaveBeenCalledWith(expect.stringContaining('hi'))
  })

  it('isolates connection handler errors', async () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {})
    svc.onConnectionChange(() => { throw new Error('cb-bad') })
    await openConnection()
    expect(err).toHaveBeenCalledWith(expect.stringContaining('Connection handler error'), expect.anything())
    err.mockRestore()
  })

  it('reports closing, closed and unknown states', async () => {
    const raw = await openConnection()
    raw.readyState = 2
    expect(svc.connectionState).toBe('closing')
    raw.readyState = 3
    expect(svc.connectionState).toBe('closed')
    ;(svc as unknown as { ws: unknown }).ws = { readyState: 99, close: vi.fn() }
    expect(svc.connectionState).toBe('closed')
  })

  it('does not reconnect after intentional disconnect', async () => {
    const raw = await openConnection()
    svc.disconnect()
    raw.onclose?.({ reason: 'done', code: 1000 } as unknown as CloseEvent)
    expect(vi.getTimerCount()).toBe(0)
  })

  it('ignores pong without id', async () => {
    const raw = await openConnection()
    const handler = vi.fn()
    svc.onEvent(handler)
    raw.onmessage?.({ data: JSON.stringify({ type: 'pong' }) } as MessageEvent)
    expect(handler).not.toHaveBeenCalled()
  })

  it('ignores pong for unknown ping ids', async () => {
    await openConnection()
    const raw = (svc as unknown as { ws: MockWs }).ws
    raw.onmessage?.({ data: JSON.stringify({ type: 'pong', id: 999 }) } as MessageEvent)
    expect((svc as unknown as { pendingPings: Map<number, unknown> }).pendingPings.size).toBe(0)
  })

  it('skips scheduled reconnect after disconnect', async () => {
    const raw = await openConnection()
    raw.readyState = 3
    raw.onclose?.({ reason: '', code: 1006 } as unknown as CloseEvent)
    svc.disconnect()
    await vi.advanceTimersByTimeAsync(31000)
    expect((svc as unknown as { ws: unknown }).ws).toBeNull()
  })

  it('heartbeat skips ping when socket is not open', async () => {
    const raw = await openConnection()
    const send = raw.send as ReturnType<typeof vi.fn>
    const pingsBefore = send.mock.calls.filter((c) => String(c[0]).includes('"type":"ping"')).length
    raw.readyState = 3
    vi.advanceTimersByTime(25000)
    expect(send.mock.calls.filter((c) => String(c[0]).includes('"type":"ping"')).length).toBe(pingsBefore)
  })
})

type MockWs = {
  send: ReturnType<typeof vi.fn>
  close: ReturnType<typeof vi.fn>
  readyState: number
  onopen: ((e: Event) => void) | null
  onclose: ((e: CloseEvent) => void) | null
  onmessage: ((e: MessageEvent) => void) | null
  onerror: ((e: Event) => void) | null
  simulateOpen: () => void
  simulateMessage: (data: unknown) => void
  simulateClose: (reason?: string) => void
}



declare global {
  interface MockWsInstance extends MockWs {}
}
