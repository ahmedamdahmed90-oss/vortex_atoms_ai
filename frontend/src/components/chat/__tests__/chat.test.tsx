import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, act } from '@testing-library/react'
import { MessageBubble, safeMarkdownUrl } from '../MessageBubble'
import { ChatSidebar } from '../ChatSidebar'
import { ChatInput } from '../ChatInput'
import { ToastProvider } from '../../ui/Toast'
import { useChatStore } from '../../../stores/chatStore'
import { useSettingsStore } from '../../../stores/settingsStore'

const hoisted = vi.hoisted(() => ({
  setCurrentMessage: vi.fn(),
  handleKeyDown: vi.fn(),
  sendMessage: vi.fn(),
  chatState: { currentMessage: '', isStreaming: false, isConnected: true },
}))

vi.mock('../../../hooks/useChat', () => ({
  useChat: () => ({
    get currentMessage() { return hoisted.chatState.currentMessage },
    setCurrentMessage: hoisted.setCurrentMessage,
    get isStreaming() { return hoisted.chatState.isStreaming },
    get isConnected() { return hoisted.chatState.isConnected },
    sendMessage: hoisted.sendMessage,
    handleKeyDown: hoisted.handleKeyDown,
    temperature: 0.7,
    maxTokens: 1024,
    systemPrompt: '',
  }),
}))

const baseMessage = {
  id: 'm1',
  content: 'مرحبا بالعالم',
  timestamp: Date.now(),
}

const renderWithToast = (ui: React.ReactElement) => render(<ToastProvider>{ui}</ToastProvider>)

describe('MessageBubble', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders user message content', () => {
    renderWithToast(<MessageBubble message={{ ...baseMessage, role: 'user' }} />)
    expect(screen.getByText('مرحبا بالعالم')).toBeInTheDocument()
  })

  it('renders markdown bold in assistant messages', () => {
    renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', content: '**مهم** جداً' }}
      />
    )
    expect(screen.getByText('مهم').closest('strong')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'نسخ الرسالة' })).toBeInTheDocument()
  })

  it('neutralizes javascript: links from model output (XSS)', () => {
    expect(safeMarkdownUrl('javascript:alert(1)')).toBe('')
    expect(safeMarkdownUrl('  JaVaScRiPt:alert(1)')).toBe('')
    expect(safeMarkdownUrl('data:text/html,<script>alert(1)</script>')).toBe('')
    expect(safeMarkdownUrl('vbscript:msgbox(1)')).toBe('')
    expect(safeMarkdownUrl('https://example.com/x')).toBe('https://example.com/x')
    expect(safeMarkdownUrl('http://localhost:8080/v1/health')).toBe('http://localhost:8080/v1/health')
    expect(safeMarkdownUrl('mailto:a@b.c')).toBe('mailto:a@b.c')
    expect(safeMarkdownUrl('/v1/models')).toBe('/v1/models')
    expect(safeMarkdownUrl('#section')).toBe('#section')
  })

  it('renders injected javascript: links as non-clickable', () => {
    renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', content: '[انقر هنا](javascript:alert(1))' }}
      />
    )
    const link = screen.getByText('انقر هنا').closest('a')
    expect(link).toBeInTheDocument()
    expect(link?.getAttribute('href')).not.toContain('javascript:')
  })

  it('renders system messages without action buttons', () => {
    renderWithToast(<MessageBubble message={{ ...baseMessage, role: 'system' }} />)
    expect(screen.getByText('مرحبا بالعالم')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'نسخ الرسالة' })).not.toBeInTheDocument()
  })

  it('shows usage badges for tokens', () => {
    renderWithToast(
      <MessageBubble
        message={{
          ...baseMessage,
          role: 'assistant',
          tokensUsed: 42,
          tokensPerSecond: 30.5,
        }}
      />
    )
    expect(screen.getByText('توكنز')).toBeInTheDocument()
    expect(screen.getByText(/30\.5/)).toBeInTheDocument()
  })

  it('calls onCopy and shows success toast on copy click', async () => {
    const onCopy = vi.fn()
    renderWithToast(
      <MessageBubble message={{ ...baseMessage, role: 'assistant' }} onCopy={onCopy} />
    )

    fireEvent.click(screen.getByRole('button', { name: 'نسخ الرسالة' }))
    expect(onCopy).toHaveBeenCalledWith('مرحبا بالعالم')
    expect(await screen.findByText('تم النسخ')).toBeInTheDocument()
  })

  it('shows regenerate only for finished assistant messages and wires it', () => {
    const onRegenerate = vi.fn()
    const { rerender } = renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', isStreaming: false }}
        onRegenerate={onRegenerate}
      />
    )
    fireEvent.click(screen.getByRole('button', { name: 'إعادة التوليد' }))
    expect(onRegenerate).toHaveBeenCalledTimes(1)

    rerender(
      <ToastProvider>
        <MessageBubble
          message={{ ...baseMessage, role: 'assistant', isStreaming: true }}
          onRegenerate={onRegenerate}
        />
      </ToastProvider>
    )
    expect(screen.queryByRole('button', { name: 'إعادة التوليد' })).not.toBeInTheDocument()
  })

  it('renders fenced code block with language header', async () => {
    renderWithToast(
      <MessageBubble message={{ ...baseMessage, role: 'assistant', content: '```python\nprint("hi")\n```' }} />
    )
    expect(screen.getByText('python')).toBeInTheDocument()
    expect(await screen.findByText(/print/)).toBeInTheDocument()
  })

  it('renders inline code', () => {
    renderWithToast(
      <MessageBubble message={{ ...baseMessage, role: 'assistant', content: 'use `const x=1` here' }} />
    )
    expect(screen.getByText('const x=1')).toBeInTheDocument()
  })

  it('renders toolCalls with args length and expandable result', () => {
    renderWithToast(
      <MessageBubble
        message={{
          ...baseMessage,
          role: 'assistant',
          content: 'hi',
          toolCalls: [{ id: 't1', name: 'calc', arguments: { x: 1 }, result: '{"ok":1}' } as never],
        }}
      />
    )
    expect(screen.getByText('calc')).toBeInTheDocument()
    expect(screen.getByText(/\(7 chars\)/)).toBeInTheDocument()
    fireEvent.click(screen.getByText('نتيجة الأداة'))
    expect(screen.getByText('{"ok":1}')).toBeInTheDocument()
  })

  it('falls back to navigator.clipboard when onCopy not provided', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    renderWithToast(<MessageBubble message={{ ...baseMessage, role: 'assistant' }} />)
    fireEvent.click(screen.getByRole('button', { name: 'نسخ الرسالة' }))
    expect(writeText).toHaveBeenCalledWith('مرحبا بالعالم')
    expect(await screen.findByText('تم النسخ')).toBeInTheDocument()
  })

  it('renders lists, quote, link and emphasis', () => {
    renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', content: '- أ\n- ب\n\n1. واحد\n\n> اقتباس مهم\n\n*مائل* و [رابط](https://x.test)' }}
      />
    )
    expect(screen.getByText('أ')).toBeInTheDocument()
    expect(screen.getByText('واحد')).toBeInTheDocument()
    expect(screen.getByText('اقتباس مهم')).toBeInTheDocument()
    expect(screen.getByText('مائل').closest('em')).toBeInTheDocument()
    expect(screen.getByText('رابط').closest('a')).toHaveAttribute('href', 'https://x.test')
  })

  it('maps code aliases and plain blocks to highlighter languages', () => {
    renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', content: '```js\nconst a=1\n```\n\n```\nplain\n```' }}
      />
    )
    expect(screen.getByText('js')).toBeInTheDocument()
    expect(screen.getByText('text')).toBeInTheDocument()
  })

  it('copies code block via its own copy button', () => {
    const onCopy = vi.fn()
    renderWithToast(
      <MessageBubble
        message={{ ...baseMessage, role: 'assistant', content: '```python\nprint(1)\n```' }}
        onCopy={onCopy}
      />
    )
    fireEvent.click(screen.getByLabelText('نسخ الكود'))
    expect(onCopy).toHaveBeenCalled()
  })

  it('renders empty code block with text header', () => {
    renderWithToast(
      <MessageBubble message={{ ...baseMessage, role: 'assistant', content: '```\n```' }} />
    )
    expect(screen.getByText('text')).toBeInTheDocument()
  })

  it('resets copy indicator after timeout', async () => {
    vi.useFakeTimers()
    try {
      const { container } = renderWithToast(
        <MessageBubble message={{ ...baseMessage, role: 'assistant' }} onCopy={() => {}} />
      )
      fireEvent.click(screen.getByRole('button', { name: 'نسخ الرسالة' }))
      expect(container.querySelector('.text-green-400')).toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(2000) })
      expect(container.querySelector('.text-green-400')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('ChatSidebar', () => {
  it('renders conversations header and fires onNewChat/onClose', () => {
    const onNewChat = vi.fn()
    const onClose = vi.fn()
    render(<ChatSidebar isOpen onNewChat={onNewChat} onClose={onClose} />)

    expect(screen.getByText('المحادثات')).toBeInTheDocument()
    expect(screen.getByText('المحادثة الحالية')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /محادثة جديدة/ }))
    expect(onNewChat).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByRole('button', { name: 'إغلاق الشريط الجانبي' }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('temperature slider writes back to chatStore', () => {
    render(<ChatSidebar isOpen onNewChat={() => {}} onClose={() => {}} />)

    const slider = screen.getAllByRole('slider')[0] as HTMLInputElement
    fireEvent.change(slider, { target: { value: '1.5' } })
    expect(useChatStore.getState().temperature).toBe(1.5)

    useChatStore.getState().setTemperature(0.7)
  })

  it('overlay click and close button call onClose when open', () => {
    const onClose = vi.fn()
    const { container } = render(<ChatSidebar isOpen onNewChat={() => {}} onClose={onClose} />)
    const overlay = container.querySelector('.absolute.inset-0, .fixed.inset-0') as HTMLElement
    expect(overlay).toBeInTheDocument()
    fireEvent.click(overlay)
    expect(onClose).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByLabelText('إغلاق الشريط الجانبي'))
    expect(onClose).toHaveBeenCalledTimes(2)
  })

  it('renders english labels when language is en', () => {
    useSettingsStore.setState({ language: 'en' } as never)
    render(<ChatSidebar isOpen onNewChat={() => {}} onClose={() => {}} />)
    expect(screen.getByText('Conversations')).toBeInTheDocument()
    expect(screen.getByText('New Chat')).toBeInTheDocument()
    expect(screen.getByText('Current Chat')).toBeInTheDocument()
    expect(screen.getByText('Chat Settings')).toBeInTheDocument()
    useSettingsStore.setState({ language: 'ar' } as never)
  })

  it('maxTokens slider and system prompt write back to chatStore', () => {
    render(<ChatSidebar isOpen onNewChat={() => {}} onClose={() => {}} />)
    const sliders = screen.getAllByRole('slider')
    fireEvent.change(sliders[1], { target: { value: '448' } })
    expect(useChatStore.getState().maxTokens).toBe(448)

    const sys = screen.getByPlaceholderText('You are a helpful AI assistant.')
    fireEvent.change(sys, { target: { value: 'كن مختصرا' } })
    expect(useChatStore.getState().systemPrompt).toBe('كن مختصرا')

    useChatStore.getState().setMaxTokens(1024)
    useChatStore.getState().setSystemPrompt('')
  })
})

describe('ChatInput', () => {
  it('renders arabic placeholder textarea', () => {
    render(<ChatInput />)
    expect(screen.getByLabelText('رسالة الشات')).toBeInTheDocument()
  })

  it('typing delegates to setCurrentMessage', () => {
    render(<ChatInput />)
    fireEvent.change(screen.getByLabelText('رسالة الشات'), { target: { value: 'نص' } })
    expect(hoisted.setCurrentMessage).toHaveBeenCalledWith('نص')
  })

  it('key events delegate to handleKeyDown', () => {
    render(<ChatInput />)
    fireEvent.keyDown(screen.getByLabelText('رسالة الشات'), { key: 'Enter' })
    expect(hoisted.handleKeyDown).toHaveBeenCalledTimes(1)
  })

  it('toggles generation settings panel', () => {
    render(<ChatInput />)

    expect(screen.queryByText('إعدادات التوليد')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'خيارات إضافية' }))
    expect(screen.getByText('إعدادات التوليد')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'خيارات إضافية' }))
    expect(screen.queryByText('إعدادات التوليد')).not.toBeInTheDocument()
  })

  it('disables send while the draft message is empty', () => {
    hoisted.chatState.currentMessage = ''
    hoisted.chatState.isStreaming = false
    hoisted.chatState.isConnected = true
    render(<ChatInput />)
    const send = screen.getByRole('button', { name: 'إرسال' })
    expect(send).toBeDisabled()
  })

  it('submits non-empty message via sendMessage', () => {
    hoisted.chatState.currentMessage = 'مرحبا'
    render(<ChatInput />)
    fireEvent.click(screen.getByRole('button', { name: 'إرسال' }))
    expect(hoisted.sendMessage).toHaveBeenCalledTimes(1)
    hoisted.chatState.currentMessage = ''
  })

  it('pasting appends clipboard text to draft', () => {
    hoisted.chatState.currentMessage = 'أ'
    render(<ChatInput />)
    const box = screen.getByLabelText('رسالة الشات')
    fireEvent.paste(box, { clipboardData: { getData: () => 'ب' } })
    expect(hoisted.setCurrentMessage).toHaveBeenCalledWith('أب')
    hoisted.chatState.currentMessage = ''
  })

  it('generation sliders write back to chatStore and X closes panel', () => {
    render(<ChatInput />)
    fireEvent.click(screen.getByRole('button', { name: 'خيارات إضافية' }))
    expect(screen.getByText('إعدادات التوليد')).toBeInTheDocument()

    const temp = screen.getByText(/درجة الحرارة/).parentElement!.querySelector('input')!
    fireEvent.change(temp, { target: { value: '1.5' } })
    expect(useChatStore.getState().temperature).toBe(1.5)

    const tokens = screen.getByText(/أقصى توكنز/).parentElement!.querySelector('input')!
    fireEvent.change(tokens, { target: { value: '448' } })
    expect(useChatStore.getState().maxTokens).toBe(448)

    const sys = screen.getByPlaceholderText('You are a helpful AI assistant.')
    fireEvent.change(sys, { target: { value: 'كن مفيدا' } })
    expect(useChatStore.getState().systemPrompt).toBe('كن مفيدا')

    fireEvent.click(screen.getByText('إعدادات التوليد').parentElement!.querySelector('button')!)
    expect(screen.queryByText('إعدادات التوليد')).not.toBeInTheDocument()

    useChatStore.getState().setTemperature(0.7)
    useChatStore.getState().setMaxTokens(1024)
    useChatStore.getState().setSystemPrompt('')
  })
})
