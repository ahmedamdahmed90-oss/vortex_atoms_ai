import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useWebSocket } from '../useWebSocket'
import { useChatStore } from '../../stores/chatStore'

const hoisted = vi.hoisted(() => ({
  onEvent: vi.fn(() => vi.fn()),
  onConnectionChange: vi.fn(() => vi.fn()),
  connect: vi.fn(() => Promise.resolve()),
  sendGenerate: vi.fn(),
  disconnect: vi.fn(),
  isConnected: false,
}))

vi.mock('../../services/ws', () => ({
  wsService: {
    get isConnected() { return hoisted.isConnected },
    onEvent: hoisted.onEvent,
    onConnectionChange: hoisted.onConnectionChange,
    connect: hoisted.connect,
    sendGenerate: hoisted.sendGenerate,
    disconnect: hoisted.disconnect,
  },
}))

describe('useWebSocket', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.isConnected = false
    useChatStore.setState({ isStreaming: false })
  })

  it('connects and subscribes on mount, unsubscribes on unmount', () => {
    const { unmount } = renderHook(() => useWebSocket())
    expect(hoisted.connect).toHaveBeenCalledTimes(1)
    expect(hoisted.onEvent).toHaveBeenCalledTimes(1)
    expect(hoisted.onConnectionChange).toHaveBeenCalledTimes(1)
    const offEvent = (hoisted.onEvent.mock.results[0] as unknown as { value: () => void }).value
    const offConn = (hoisted.onConnectionChange.mock.results[0] as unknown as { value: () => void }).value
    unmount()
    expect(offEvent).toHaveBeenCalledTimes(1)
    expect(offConn).toHaveBeenCalledTimes(1)
  })

  it('clears streaming when connection drops while streaming', () => {
    useChatStore.setState({ isStreaming: true })
    renderHook(() => useWebSocket())
    const connHandler = (hoisted.onConnectionChange.mock.calls as unknown as Array<[(c: boolean) => void]>)[0][0] as (connected: boolean) => void
    act(() => connHandler(false))
    expect(useChatStore.getState().isStreaming).toBe(false)
  })

  it('does not clear streaming when not streaming', () => {
    useChatStore.setState({ isStreaming: false })
    renderHook(() => useWebSocket())
    const connHandler = (hoisted.onConnectionChange.mock.calls as unknown as Array<[(c: boolean) => void]>)[0][0] as (connected: boolean) => void
    act(() => connHandler(false))
    expect(useChatStore.getState().isStreaming).toBe(false)
  })

  it('delegates sendGenerate and disconnect to wsService', () => {
    const { result } = renderHook(() => useWebSocket())
    act(() => result.current.sendGenerate({ prompt: 'hi', max_tokens: 10, temperature: 0.7 } as never))
    expect(hoisted.sendGenerate).toHaveBeenCalledWith({ prompt: 'hi', max_tokens: 10, temperature: 0.7 })
    act(() => result.current.disconnect())
    expect(hoisted.disconnect).toHaveBeenCalledTimes(1)
  })

it('exposes isConnected from wsService', () => {
     hoisted.isConnected = true
     const { result } = renderHook(() => useWebSocket())
     expect(result.current.isConnected).toBe(true)
   })

   it('streaming state transitions through the socket handler', () => {
     useChatStore.setState({ isStreaming: true })
     const { unmount } = renderHook(() => useWebSocket())
     const connHandler = (hoisted.onConnectionChange.mock.calls as unknown as Array<[(c: boolean) => void]>)[0][0] as (connected: boolean) => void
     act(() => connHandler(false))
     expect(useChatStore.getState().isStreaming).toBe(false)
     unmount()
   })
})
