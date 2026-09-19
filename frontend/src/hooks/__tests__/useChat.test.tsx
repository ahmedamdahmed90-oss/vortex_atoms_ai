import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useChat } from '../useChat'
import { useChatStore } from '../../stores/chatStore'

const hoisted = vi.hoisted(() => ({
  isConnected: false,
  sendGenerate: vi.fn(),
  apiChat: vi.fn(),
}))

vi.mock('../useWebSocket', () => ({
  useWebSocket: () => ({ isConnected: hoisted.isConnected, sendGenerate: hoisted.sendGenerate }),
}))

vi.mock('../../services/api', () => ({
  api: { chat: (...args: unknown[]) => hoisted.apiChat(...args) },
}))

describe('useChat', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.isConnected = false
    useChatStore.setState({
      messages: [],
      currentMessage: '',
      isStreaming: false,
      streamingMessageId: null,
      streamingContent: '',
      error: null,
    })
  })

  it('routes to WebSocket when connected', async () => {
    hoisted.isConnected = true
    useChatStore.setState({ currentMessage: 'مرحبا', isStreaming: false })
    const { result } = renderHook(() => useChat())
    await act(async () => { await result.current.sendMessage() })
    expect(hoisted.sendGenerate).toHaveBeenCalledWith({ prompt: 'مرحبا', max_tokens: expect.any(Number), temperature: expect.any(Number) })
    expect(hoisted.apiChat).not.toHaveBeenCalled()
  })

  it('falls back to HTTP API when offline and appends both messages', async () => {
    hoisted.apiChat.mockResolvedValue({ text: 'رد', usage: { total_tokens: 7 } })
    useChatStore.setState({ currentMessage: 'سؤال', systemPrompt: 'sys' })
    const { result } = renderHook(() => useChat())
    await act(async () => { await result.current.sendMessage() })
    expect(hoisted.apiChat).toHaveBeenCalledWith({
      messages: [{ role: 'system', content: 'sys' }, { role: 'user', content: 'سؤال' }],
      max_tokens: expect.any(Number),
      temperature: expect.any(Number),
    })
    const st = useChatStore.getState()
    expect(st.messages).toHaveLength(2)
    expect(st.messages[0].role).toBe('user')
    expect(st.messages[1].content).toBe('رد')
    expect(st.tokensUsed).toBe(7)
    expect(st.isStreaming).toBe(false)
  })

  it('does nothing on empty message or while streaming', async () => {
    const { result } = renderHook(() => useChat())
    useChatStore.setState({ currentMessage: '   ', isStreaming: false })
    await act(async () => { await result.current.sendMessage() })
    expect(hoisted.sendGenerate).not.toHaveBeenCalled()

    useChatStore.setState({ currentMessage: 'hi', isStreaming: true })
    await act(async () => { await result.current.sendMessage() })
    expect(hoisted.sendGenerate).not.toHaveBeenCalled()
  })

  it('surfaces API errors via setError', async () => {
    hoisted.apiChat.mockRejectedValue(new Error('boom'))
    useChatStore.setState({ currentMessage: 'q' })
    const { result } = renderHook(() => useChat())
    await act(async () => { await result.current.sendMessage() })
    expect(useChatStore.getState().error).toBe('boom')
  })

  it('surfaces non-Error failures with default message', async () => {
    hoisted.apiChat.mockRejectedValue('plain-failure')
    useChatStore.setState({ currentMessage: 'q' })
    const { result } = renderHook(() => useChat())
    await act(async () => { await result.current.sendMessage() })
    expect(useChatStore.getState().error).toBe('فشل في إرسال الرسالة')
  })

  it('handleKeyDown Enter without Shift sends, with Shift does not', async () => {
    hoisted.isConnected = true
    useChatStore.setState({ currentMessage: 'hi', isStreaming: false })
    const { result } = renderHook(() => useChat())
    const preventDefault = vi.fn()
    await act(async () => { result.current.handleKeyDown({ key: 'Enter', shiftKey: false, preventDefault } as unknown as React.KeyboardEvent) })
    expect(preventDefault).toHaveBeenCalledTimes(1)
    await waitFor(() => expect(hoisted.sendGenerate).toHaveBeenCalledTimes(1))

    const prevent2 = vi.fn()
    act(() => result.current.handleKeyDown({ key: 'Enter', shiftKey: true, preventDefault: prevent2 } as unknown as React.KeyboardEvent))
    expect(prevent2).not.toHaveBeenCalled()
  })

  it('regenerate restores last user message and resends', async () => {
    hoisted.isConnected = true
    useChatStore.setState({
      isStreaming: false,
      currentMessage: '',
      messages: [
        { id: 'a', role: 'assistant', content: 'r', timestamp: 1 },
        { id: 'b', role: 'user', content: 'last-q', timestamp: 2 },
      ],
    })
    const { result } = renderHook(() => useChat())
    await act(async () => { await result.current.regenerate() })
    expect(useChatStore.getState().currentMessage).toBe('last-q')
    expect(hoisted.sendGenerate).toHaveBeenCalledTimes(1)
  })

  it('clearChat empties messages', () => {
    useChatStore.setState({ messages: [{ id: 'x', role: 'user', content: 'hi', timestamp: 1 }] })
    const { result } = renderHook(() => useChat())
    act(() => result.current.clearChat())
    expect(useChatStore.getState().messages).toHaveLength(0)
  })

  it('regenerate does nothing without user messages or while streaming', async () => {
    hoisted.isConnected = true
    useChatStore.setState({
      isStreaming: false,
      currentMessage: '',
      messages: [{ id: 'a', role: 'assistant', content: 'r', timestamp: 1 }],
    })
    const { result } = renderHook(() => useChat())
    const calls = (hoisted.sendGenerate as ReturnType<typeof vi.fn>).mock.calls.length
    await act(async () => { await result.current.regenerate() })
    expect((hoisted.sendGenerate as ReturnType<typeof vi.fn>).mock.calls.length).toBe(calls)
    expect(useChatStore.getState().currentMessage).toBe('')

    useChatStore.setState({
      isStreaming: true,
      messages: [{ id: 'b', role: 'user', content: 'q', timestamp: 2 }],
    })
    await act(async () => { await result.current.regenerate() })
    expect((hoisted.sendGenerate as ReturnType<typeof vi.fn>).mock.calls.length).toBe(calls)
    useChatStore.setState({ isStreaming: false, messages: [] })
  })
})
