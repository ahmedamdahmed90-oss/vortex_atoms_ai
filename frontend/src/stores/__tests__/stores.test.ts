import { describe, it, expect, beforeEach } from 'vitest'
import { useSettingsStore } from '../settingsStore'
import { useChatStore, chatStoreGetState } from '../chatStore'
import { useDashboardStore } from '../dashboardStore'
import { DEFAULT_LANGUAGE, STORAGE_KEY } from '../../i18n/config'

// Captured at import time: settingsStore registers a 'change' listener on the
// prefers-color-scheme media query when the module loads (before any mock reset).
const systemThemeChangeHandler = (() => {
  const mm = window.matchMedia as unknown as { mock: { results: Array<{ value: { addEventListener: { mock: { calls: unknown[][] } } } }> } }
  for (const r of mm.mock.results) {
    const calls = r.value?.addEventListener?.mock?.calls ?? []
    const found = calls.find((c) => c[0] === 'change')
    if (found) return found[1] as () => void
  }
  return null
})()

describe('settingsStore', () => {
  beforeEach(() => {
    const s = useSettingsStore.getState()
    s.resetToDefaults()
  })

  it('starts with default settings', () => {
    const s = useSettingsStore.getState()
    expect(s.language).toBe(DEFAULT_LANGUAGE)
    expect(s.theme).toBe('system')
    expect(s.animationsEnabled).toBe(true)
    expect(s.compactMode).toBe(false)
    expect(s.apiUrl).toContain('8080')
  })

  it('setLanguage updates state, DOM attributes and storage', () => {
    useSettingsStore.getState().setLanguage('en')

    const s = useSettingsStore.getState()
    expect(s.language).toBe('en')
    expect(document.documentElement.lang).toBe('en')
    expect(document.documentElement.dir).toBe('ltr')
    expect(localStorage.setItem).toHaveBeenCalledWith(STORAGE_KEY, 'en')

    useSettingsStore.getState().setLanguage('ar')
    expect(document.documentElement.dir).toBe('rtl')
  })

  it('setTheme toggles the dark class on the root element', () => {
    useSettingsStore.getState().setTheme('dark')
    expect(document.documentElement.classList.contains('dark')).toBe(true)

    useSettingsStore.getState().setTheme('light')
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('setCompactMode toggles compact-mode class', () => {
    useSettingsStore.getState().setCompactMode(true)
    expect(document.documentElement.classList.contains('compact-mode')).toBe(true)

    useSettingsStore.getState().setCompactMode(false)
    expect(document.documentElement.classList.contains('compact-mode')).toBe(false)
  })

  it('resetToDefaults restores defaults and clears persisted keys', () => {
    useSettingsStore.getState().setApiUrl('http://changed:9999/v1')
    useSettingsStore.getState().setTheme('dark')
    expect(useSettingsStore.getState().apiUrl).toBe('http://changed:9999/v1')

    useSettingsStore.getState().resetToDefaults()

    const s = useSettingsStore.getState()
    expect(s.theme).toBe('system')
    expect(s.apiUrl).not.toBe('http://changed:9999/v1')
    expect(localStorage.removeItem).toHaveBeenCalledWith(STORAGE_KEY)
    expect(localStorage.removeItem).toHaveBeenCalledWith('vortex-settings-store')
  })

  it('setApiUrl/setWsUrl update endpoints', () => {
    useSettingsStore.getState().setApiUrl('http://x:9999/v1')
    useSettingsStore.getState().setWsUrl('ws://x:9999/ws')
    expect(useSettingsStore.getState().apiUrl).toBe('http://x:9999/v1')
    expect(useSettingsStore.getState().wsUrl).toBe('ws://x:9999/ws')
  })

  it('setAnimationsEnabled toggles animations-disabled class', () => {
    useSettingsStore.getState().setAnimationsEnabled(false)
    expect(document.documentElement.classList.contains('animations-disabled')).toBe(true)

    useSettingsStore.getState().setAnimationsEnabled(true)
    expect(document.documentElement.classList.contains('animations-disabled')).toBe(false)
  })

  it('applyTheme resolves system theme via matchMedia', () => {
    useSettingsStore.getState().setTheme('system')
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('system theme follows OS changes via media listener', () => {
    expect(systemThemeChangeHandler).not.toBeNull()
    useSettingsStore.getState().setTheme('system')
    systemThemeChangeHandler!()
    expect(document.documentElement.classList.contains('dark')).toBe(false)

    useSettingsStore.getState().setTheme('dark')
    systemThemeChangeHandler!()
    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })

  it('migrate fills missing keys with defaults', () => {
    const migrate = useSettingsStore.persist.getOptions().migrate as unknown as (s: unknown) => Record<string, unknown>
    const out = migrate({ theme: 'dark' })
    expect(out.theme).toBe('dark')
    expect(out.animationsEnabled).toBe(true)
    expect(out.compactMode).toBe(false)
  })
})

const makeMessage = (id: string) => ({
  id,
  role: 'user' as const,
  content: `content-${id}`,
  timestamp: Date.now(),
})

describe('chatStore', () => {
  beforeEach(() => {
    const st = useChatStore.getState()
    st.clearMessages()
    st.setCurrentMessage('')
    st.setStreaming(false)
    st.setStreamingMessage(null)
    st.setError(null)
    useChatStore.setState({ streamingContent: '' })
  })

  it('adds, updates and removes messages', () => {
    const st = useChatStore.getState()
    st.addMessage(makeMessage('a'))
    st.addMessage({ ...makeMessage('b'), role: 'assistant' as const })
    expect(useChatStore.getState().messages.map(m => m.id)).toEqual(['a', 'b'])

    useChatStore.getState().updateMessage('b', { content: 'updated' })
    expect(useChatStore.getState().messages.find(m => m.id === 'b')?.content).toBe('updated')

    useChatStore.getState().removeMessage('a')
    expect(useChatStore.getState().messages.map(m => m.id)).toEqual(['b'])
  })

  it('appendStreamingContent accumulates chunks', () => {
    useChatStore.getState().appendStreamingContent('مرحبا')
    useChatStore.getState().appendStreamingContent(' بك')
    expect(useChatStore.getState().streamingContent).toBe('مرحبا بك')
  })

  it('handleWsEvent ready creates user+assistant pair from currentMessage', () => {
    useChatStore.getState().setCurrentMessage('سؤال تجريبي')

    useChatStore.getState().handleWsEvent({ type: 'ready' } as never)

    const s = useChatStore.getState()
    expect(s.messages).toHaveLength(2)
    expect(s.messages[0].role).toBe('user')
    expect(s.messages[0].content).toBe('سؤال تجريبي')
    expect(s.messages[1].role).toBe('assistant')
    expect(s.messages[1].isStreaming).toBe(true)
    expect(s.isStreaming).toBe(true)
    expect(s.currentMessage).toBe('')
    expect(s.streamingMessageId).toBe(s.messages[1].id)
  })

  it('handleWsEvent token streams into the assistant message', () => {
    const st = useChatStore.getState()
    st.setCurrentMessage('q')
    st.handleWsEvent({ type: 'ready' } as never)

    useChatStore.getState().handleWsEvent({ type: 'token', text: 'answer' } as never)
    useChatStore.getState().handleWsEvent({ type: 'token', text: '!' } as never)

    const s = useChatStore.getState()
    expect(s.streamingContent).toBe('answer!')
    expect(s.messages.find(m => m.role === 'assistant')?.content).toBe('answer!')
  })

  it('handleWsEvent batch appends coalesced tokens in order', () => {
    const st = useChatStore.getState()
    st.setCurrentMessage('q')
    st.handleWsEvent({ type: 'ready' } as never)

    useChatStore.getState().handleWsEvent({ type: 'batch', events: [{ type: 'token', token_id: 1, text: 'coalesced', position: 0 }, { type: 'token', token_id: 2, text: ' tokens', position: 1 }] } as never)

    const s = useChatStore.getState()
    expect(s.streamingContent).toBe('coalesced tokens')
    expect(s.messages.find(m => m.role === 'assistant')?.content).toBe('coalesced tokens')
  })

  it('handleWsEvent done finalizes message with usage stats', () => {
    const st = useChatStore.getState()
    st.setCurrentMessage('q')
    st.handleWsEvent({ type: 'ready' } as never)
    st.handleWsEvent({ type: 'token', text: 'final' } as never)

    st.handleWsEvent({
      type: 'done',
      full_text: 'final answer',
      total_tokens: 42,
      tokens_per_second: 30.5,
    } as never)

    const s = useChatStore.getState()
    expect(s.isStreaming).toBe(false)
    expect(s.streamingMessageId).toBeNull()
    expect(s.streamingContent).toBe('')
    expect(s.tokensUsed).toBe(42)
    expect(s.tokensPerSecond).toBe(30.5)
    const assistant = s.messages.find(m => m.role === 'assistant')
    expect(assistant?.content).toBe('final answer')
    expect(assistant?.isStreaming).toBe(false)
  })

  it('handleWsEvent error surfaces message and stops streaming', () => {
    const st = useChatStore.getState()
    st.setCurrentMessage('q')
    st.handleWsEvent({ type: 'ready' } as never)

    st.handleWsEvent({ type: 'error', message: 'model failed' } as never)

    const s = useChatStore.getState()
    expect(s.error).toBe('model failed')
    expect(s.isStreaming).toBe(false)
    expect(s.messages.some(m => m.content.includes('خطأ'))).toBe(true)
  })

  it('ignores token/done events when not streaming', () => {
    useChatStore.getState().addMessage(makeMessage('solo'))
    useChatStore.getState().handleWsEvent({ type: 'token', text: 'x' } as never)
    useChatStore.getState().handleWsEvent({
      type: 'done', full_text: 'y', total_tokens: 1, tokens_per_second: 1,
    } as never)

    const s = useChatStore.getState()
    expect(s.messages).toHaveLength(1)
    expect(s.messages[0].content).toBe('content-solo')
  })

  it('setMessages replaces the list wholesale', () => {
    useChatStore.getState().setMessages([makeMessage('x'), makeMessage('y')])
    expect(useChatStore.getState().messages.map(m => m.id)).toEqual(['x', 'y'])
  })

  it('setTokensPerSecond and setSelectedModel update metrics', () => {
    useChatStore.getState().setTokensPerSecond(44.5)
    useChatStore.getState().setSelectedModel('other-model')
    const s = useChatStore.getState()
    expect(s.tokensPerSecond).toBe(44.5)
    expect(s.selectedModel).toBe('other-model')
  })

  it('chatStoreGetState exposes current state', () => {
    expect(chatStoreGetState().messages).toEqual(useChatStore.getState().messages)
  })

  it('ignores ready event with blank current message', () => {
    useChatStore.getState().setCurrentMessage('   ')
    useChatStore.getState().handleWsEvent({ type: 'ready' } as never)
    expect(useChatStore.getState().messages).toHaveLength(0)
    expect(useChatStore.getState().isStreaming).toBe(false)
  })

  it('ignores error event when not streaming', () => {
    useChatStore.getState().handleWsEvent({ type: 'error', message: 'x' } as never)
    expect(useChatStore.getState().error).toBeNull()
    expect(useChatStore.getState().isStreaming).toBe(false)
  })
})

describe('dashboardStore', () => {
  beforeEach(() => {
    useDashboardStore.getState().clearLogs()
    useDashboardStore.getState().setActiveTab('overview')
    useDashboardStore.getState().setSidebarOpen(true)
    useDashboardStore.getState().setLogFilter('all')
  })

  it('switches active tab and sidebar state', () => {
    useDashboardStore.getState().setActiveTab('models')
    expect(useDashboardStore.getState().activeTab).toBe('models')

    useDashboardStore.getState().toggleSidebar()
    expect(useDashboardStore.getState().sidebarOpen).toBe(false)
    useDashboardStore.getState().toggleSidebar()
    expect(useDashboardStore.getState().sidebarOpen).toBe(true)
  })

  it('stores health, models, tools and search results', () => {
    const st = useDashboardStore.getState()
    st.setHealth({ status: 'ok', version: '0.2.3', build_ts: '1758615338', git_sha: '3f3c47f', architecture: '5km', device: 'cpu', simd: 'avx2', history_length: 0, uptime_seconds: 12, knowledge_chunks: 7, perf: { sku: 'avx1', tier: 'std', tier_model: 'qwen', tier_max_context: 4096, infer_threads: 2, async_workers: 2, prefault_enabled: true, compiled_features: ['sse2'], host_features: ['sse2'], fastpath_avg_ms: 1.5 } })
    st.setModels([{ name: 'm1', architecture: 'qwen', max_seq_len: 32768, max_generation_tokens: 8192 }])
    st.setTools([{ name: 'calc', description: 'calculator' }])
    st.setSearchResults([{ id: 'c1', score: 0.9, text: 'chunk' }])
    st.setCurrentModel('m1')
    st.setKnowledgeChunks(99)

    const s = useDashboardStore.getState()
    expect(s.health?.status).toBe('ok')
    expect(s.models).toHaveLength(1)
    expect(s.tools?.[0].name).toBe('calc')
    expect(s.searchResults?.[0].score).toBeCloseTo(0.9)
    expect(s.currentModel).toBe('m1')
    expect(s.knowledgeChunks).toBe(99)
  })

  it('addLog prepends and caps at 1000 entries', () => {
    for (let i = 0; i < 1005; i++) useDashboardStore.getState().addLog(`log-${i}`)
    const logs = useDashboardStore.getState().logs
    expect(logs).toHaveLength(1000)
    expect(logs[0]).toBe('log-1004')
  })

  it('clearLogs empties and setLogFilter filters value persists in state', () => {
    useDashboardStore.getState().addLog('x')
    useDashboardStore.getState().clearLogs()
    expect(useDashboardStore.getState().logs).toHaveLength(0)

    useDashboardStore.getState().setLogFilter('error')
    expect(useDashboardStore.getState().logFilter).toBe('error')
  })
})
