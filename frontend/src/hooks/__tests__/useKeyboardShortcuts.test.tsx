import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import { useKeyboardShortcuts, KeyboardShortcutsHelp, useChatKeyboardShortcuts } from '../useKeyboardShortcuts'
import { useSettingsStore } from '../../stores/settingsStore'

describe('useKeyboardShortcuts', () => {
  it('fires matching shortcut and prevents default', () => {
    const action = vi.fn()
    renderHook(() => useKeyboardShortcuts([{ key: 'k', ctrl: true, description: 'x', action }]))
    const e = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })
    const preventDefault = vi.spyOn(e, 'preventDefault')
    const stopPropagation = vi.spyOn(e, 'stopPropagation')
    document.dispatchEvent(e)
    expect(action).toHaveBeenCalledTimes(1)
    expect(preventDefault).toHaveBeenCalledTimes(1)
    expect(stopPropagation).toHaveBeenCalledTimes(1)
  })

  it('is case-insensitive and respects modifiers', () => {
    const action = vi.fn()
    renderHook(() => useKeyboardShortcuts([{ key: 'd', ctrl: true, shift: true, description: 'x', action }]))
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'D', ctrlKey: true, shiftKey: true, bubbles: true }))
    expect(action).toHaveBeenCalledTimes(1)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, bubbles: true }))
    expect(action).toHaveBeenCalledTimes(1)
  })

  it('ignores non-matching keys and cleans up on unmount', () => {
    const action = vi.fn()
    const { unmount } = renderHook(() => useKeyboardShortcuts([{ key: 'k', ctrl: true, description: 'x', action }]))
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true, bubbles: true }))
    expect(action).not.toHaveBeenCalled()
    unmount()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }))
    expect(action).not.toHaveBeenCalled()
  })

  it('matches plain keys without modifiers and rejects extra modifiers', () => {
    const action = vi.fn()
    renderHook(() => useKeyboardShortcuts([{ key: 'Escape', description: 'x', action }]))
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(action).toHaveBeenCalledTimes(1)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', ctrlKey: true, bubbles: true }))
    expect(action).toHaveBeenCalledTimes(1)
  })

  it('renders english descriptions when language is en', () => {
    useSettingsStore.setState({ language: 'en' } as never)
    render(<KeyboardShortcutsHelp isOpen onClose={() => {}} />)
    expect(screen.getByText('Keyboard Shortcuts', { selector: 'h2' })).toBeInTheDocument()
    expect(screen.getByText('New chat')).toBeInTheDocument()
    const actions = { onNewChat: vi.fn(), onSendMessage: vi.fn(), onClearChat: vi.fn(), onToggleTheme: vi.fn(), onShowShortcuts: vi.fn() }
    renderHook(() => useChatKeyboardShortcuts(actions))
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true }))
    expect(actions.onNewChat).toHaveBeenCalledTimes(1)
    useSettingsStore.setState({ language: 'ar' } as never)
  })
})

describe('KeyboardShortcutsHelp', () => {
  it('renders nothing when closed', () => {
    const { container } = render(<KeyboardShortcutsHelp isOpen={false} onClose={() => {}} />)
    expect(container.textContent).toBe('')
  })

  it('renders shortcut list and closes via button', () => {
    const onClose = vi.fn()
    render(<KeyboardShortcutsHelp isOpen onClose={onClose} />)
    expect(screen.getByText(/اختصارات لوحة المفاتيح|Keyboard Shortcuts/, { selector: 'h2' })).toBeInTheDocument()
    expect(screen.getAllByText('Ctrl').length).toBeGreaterThan(0)
    fireEvent.click(screen.getByRole('button', { name: /إغلاق|Close/ }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('overlay click closes without propagating', () => {
    const onClose = vi.fn()
    const { container } = render(<KeyboardShortcutsHelp isOpen onClose={onClose} />)
    fireEvent.click(container.firstChild as HTMLElement)
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})

describe('useChatKeyboardShortcuts', () => {
  it('wires Ctrl+N, Ctrl+Enter, Ctrl+Shift+L, Ctrl+Shift+D, Ctrl+/', () => {
    const onNewChat = vi.fn()
    const onSendMessage = vi.fn()
    const onClearChat = vi.fn()
    const onToggleTheme = vi.fn()
    const onShowShortcuts = vi.fn()
    renderHook(() => useChatKeyboardShortcuts({ onNewChat, onSendMessage, onClearChat, onToggleTheme, onShowShortcuts }))

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true }))
    expect(onNewChat).toHaveBeenCalledTimes(1)

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true }))
    expect(onSendMessage).toHaveBeenCalledTimes(1)

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'l', ctrlKey: true, shiftKey: true, bubbles: true }))
    expect(onClearChat).toHaveBeenCalledTimes(1)

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, shiftKey: true, bubbles: true }))
    expect(onToggleTheme).toHaveBeenCalledTimes(1)

    document.dispatchEvent(new KeyboardEvent('keydown', { key: '/', ctrlKey: true, bubbles: true }))
    expect(onShowShortcuts).toHaveBeenCalledTimes(1)
  })
})
