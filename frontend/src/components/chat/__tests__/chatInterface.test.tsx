import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ChatInterface } from '../ChatInterface'
import { ToastProvider } from '../../ui/Toast'
import { useSettingsStore } from '../../../stores/settingsStore'

const hoisted = vi.hoisted(() => ({
  chat: {
    messages: [] as Array<{ id: string; role: string; content: string; timestamp: number }>,
    currentMessage: '',
    setCurrentMessage: vi.fn(),
    isStreaming: false,
    isConnected: true,
    sendMessage: vi.fn(),
    handleKeyDown: vi.fn(),
    temperature: 0.7,
    maxTokens: 1024,
    systemPrompt: '',
    selectedModel: 'qwen2.5-0.5b',
    tokensUsed: 0,
    tokensPerSecond: 0,
    clearChat: vi.fn(),
    regenerate: vi.fn(),
    error: null as string | null,
  },
  wsConnected: true,
  network: {
    isOnline: true,
    isServerReachable: true,
    latency: 12,
    checkServerConnection: vi.fn(),
    isChecking: false,
  },
  kbd: vi.fn(),
}))

vi.mock('../../../hooks/useChat', () => ({ useChat: () => hoisted.chat }))
vi.mock('../../../hooks/useWebSocket', () => ({ useWebSocket: () => ({ isConnected: hoisted.wsConnected }) }))
vi.mock('../../../hooks', () => ({
  useNetworkStatus: () => hoisted.network,
  useChatKeyboardShortcuts: hoisted.kbd,
}))

const renderWithToast = (ui: React.ReactElement) => render(<ToastProvider>{ui}</ToastProvider>)

describe('ChatInterface', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.chat.messages = []
    hoisted.chat.isStreaming = false
    hoisted.chat.error = null
    hoisted.wsConnected = true
    hoisted.network.isOnline = true
    hoisted.network.isServerReachable = true
  })

  it('shows welcome screen when no messages', () => {
    renderWithToast(<ChatInterface />)
    expect(screen.getByText('مرحباً بك في Vortex AI')).toBeInTheDocument()
    expect(screen.getByText('اكتب دالة بايثون لحساب فيبوناتشي')).toBeInTheDocument()
    expect(screen.getByRole('log', { name: /رسائل الشات/ })).toBeInTheDocument()
  })

  it('hides welcome and renders messages when present', () => {
    hoisted.chat.messages = [{ id: '1', role: 'user', content: 'مرحبا', timestamp: Date.now() }]
    renderWithToast(<ChatInterface />)
    expect(screen.queryByText('مرحباً بك في Vortex AI')).not.toBeInTheDocument()
    expect(screen.getByText('مرحبا')).toBeInTheDocument()
  })

  it('shows streaming indicator and error alert', () => {
    hoisted.chat.messages = [{ id: '1', role: 'user', content: 'hi', timestamp: 1 }]
    hoisted.chat.isStreaming = true
    const { rerender } = renderWithToast(<ChatInterface />)
    expect(screen.getByText(/يكتب|Thinking/)).toBeInTheDocument()

    hoisted.chat.isStreaming = false
    hoisted.chat.error = 'فشل'
    rerender(<ToastProvider><ChatInterface /></ToastProvider>)
    expect(screen.getByRole('alert')).toHaveTextContent('فشل')
  })

  it('new chat button calls clearChat', () => {
    hoisted.chat.messages = [{ id: '1', role: 'user', content: 'hi', timestamp: 1 }]
    renderWithToast(<ChatInterface />)
    fireEvent.click(screen.getByTitle(/محادثة جديدة|New chat/))
    expect(hoisted.chat.clearChat).toHaveBeenCalledTimes(1)
  })

  it('clear button confirms and clears', () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true)
    renderWithToast(<ChatInterface />)
    fireEvent.click(screen.getByTitle(/مسح المحادثة|Clear chat/))
    expect(confirmSpy).toHaveBeenCalledTimes(1)
    expect(hoisted.chat.clearChat).toHaveBeenCalledTimes(1)
    confirmSpy.mockReturnValue(false)
    fireEvent.click(screen.getByTitle(/مسح المحادثة|Clear chat/))
    expect(hoisted.chat.clearChat).toHaveBeenCalledTimes(1)
    confirmSpy.mockRestore()
  })

  it('shows offline banner when disconnected', () => {
    hoisted.network.isServerReachable = false
    renderWithToast(<ChatInterface />)
    expect(screen.getAllByText(/الخادم غير متاح|Server unreachable/).length).toBeGreaterThan(0)
  })

  it('shortcuts button opens and closes the help modal', () => {
    renderWithToast(<ChatInterface />)
    expect(screen.queryByText('اختصارات لوحة المفاتيح', { selector: 'h2' })).not.toBeInTheDocument()

    fireEvent.click(screen.getByLabelText('اختصارات لوحة المفاتيح'))
    expect(screen.getByText('اختصارات لوحة المفاتيح', { selector: 'h2' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'إغلاق' }))
    expect(screen.queryByText('اختصارات لوحة المفاتيح', { selector: 'h2' })).not.toBeInTheDocument()
  })

  it('keyboard shortcut actions send message and toggle theme', () => {
    renderWithToast(<ChatInterface />)
    const actions = hoisted.kbd.mock.calls[0][0] as {
      onSendMessage: () => void
      onToggleTheme: () => void
    }

    const ta = screen.getByLabelText('رسالة الشات') as HTMLTextAreaElement
    ta.value = 'hello'
    actions.onSendMessage()
    expect(hoisted.chat.sendMessage).toHaveBeenCalledTimes(1)

    actions.onToggleTheme()
    expect(useSettingsStore.getState().theme).toBe('dark')
    useSettingsStore.getState().setTheme('system')
  })

  it('sidebar toggle opens and closes via overlay', () => {
    renderWithToast(<ChatInterface />)
    fireEvent.click(screen.getByLabelText('فتح الشريط الجانبي'))
    const overlay = document.querySelector('.fixed.inset-0.bg-black\\/50') as HTMLElement
    expect(overlay).toBeInTheDocument()
    fireEvent.click(overlay)
    expect(document.querySelector('.fixed.inset-0.bg-black\\/50')).not.toBeInTheDocument()
  })

  it('example prompt fills the input', () => {
    renderWithToast(<ChatInterface />)
    fireEvent.click(screen.getByText('اكتب دالة بايثون لحساب فيبوناتشي'))
    expect(hoisted.chat.setCurrentMessage).toHaveBeenCalled()
  })

  it('regenerate button wires assistant messages', () => {
    hoisted.chat.messages = [{ id: 'a', role: 'assistant', content: 'رد', timestamp: 1 }]
    renderWithToast(<ChatInterface />)
    fireEvent.click(screen.getByRole('button', { name: 'إعادة التوليد' }))
    expect(hoisted.chat.regenerate).toHaveBeenCalledTimes(1)
  })

  it('renders english labels when language is en', () => {
    useSettingsStore.setState({ language: 'en' } as never)
    const { rerender } = renderWithToast(<ChatInterface />)
    expect(screen.getByText('Smart Chat')).toBeInTheDocument()
    expect(screen.getByText('Welcome to Vortex AI')).toBeInTheDocument()
    expect(screen.getByLabelText('Keyboard shortcuts')).toBeInTheDocument()

    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true)
    fireEvent.click(screen.getByTitle('Clear chat'))
    expect(hoisted.chat.clearChat).toHaveBeenCalledTimes(1)
    confirmSpy.mockRestore()

    hoisted.chat.messages = [{ id: '1', role: 'user', content: 'hi', timestamp: 1 }]
    hoisted.chat.isStreaming = true
    rerender(<ToastProvider><ChatInterface /></ToastProvider>)
    expect(screen.getByText('Thinking...')).toBeInTheDocument()

    useSettingsStore.setState({ language: 'ar' } as never)
  })

  it('keyboard toggle switches dark to light', () => {
    useSettingsStore.setState({ theme: 'dark' } as never)
    renderWithToast(<ChatInterface />)
    const actions = hoisted.kbd.mock.calls[0][0] as { onToggleTheme: () => void }
    actions.onToggleTheme()
    expect(useSettingsStore.getState().theme).toBe('light')
    useSettingsStore.setState({ theme: 'system' } as never)
  })

  it('shortcut send ignores empty drafts', () => {
    renderWithToast(<ChatInterface />)
    const actions = hoisted.kbd.mock.calls[0][0] as { onSendMessage: () => void }
    const ta = screen.getByLabelText('رسالة الشات') as HTMLTextAreaElement
    ta.value = ''
    actions.onSendMessage()
    expect(hoisted.chat.sendMessage).not.toHaveBeenCalled()
  })

  it('scrolls to bottom after messages render', async () => {    hoisted.chat.messages = [{ id: '1', role: 'user', content: 'hi', timestamp: 1 }]
    const proto = window.HTMLElement.prototype as unknown as Record<string, unknown>
    const orig = proto.scrollIntoView
    proto.scrollIntoView = vi.fn()
    try {
      renderWithToast(<ChatInterface />)
      await new Promise((r) => setTimeout(r, 200))
      expect(proto.scrollIntoView as ReturnType<typeof vi.fn>).toHaveBeenCalled()
    } finally {
      if (orig === undefined) delete proto.scrollIntoView
      else proto.scrollIntoView = orig
    }
  }, 10000)
})
