import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, act } from '@testing-library/react'
import { renderHook, waitFor } from '@testing-library/react'
import { useFocusTrap, useKeyboardNavigation, useAnnouncer } from '../useAccessibility'

describe('useFocusTrap', () => {
  it('focuses first element and traps Tab navigation', async () => {
    const Wrapper = ({ enabled = true }: { enabled?: boolean }) => {
      const ref = useFocusTrap(enabled)
      return (
        <div ref={ref}>
          <button>first</button>
          <button>second</button>
          <button>last</button>
        </div>
      )
    }

    render(<Wrapper />)
    await waitFor(() => expect(document.activeElement?.textContent).toBe('first'))

    const last = screen.getByText('last') as HTMLElement
    last.focus()
    fireEvent.keyDown(last.parentElement!, { key: 'Tab' })
    expect(document.activeElement?.textContent).toBe('first')

    const first = screen.getByText('first') as HTMLElement
    first.focus()
    fireEvent.keyDown(first.parentElement!, { key: 'Tab', shiftKey: true })
    expect(document.activeElement?.textContent).toBe('last')
  })

  it('does nothing when disabled', () => {
    const Wrapper = () => {
      const ref = useFocusTrap(false)
      return <div ref={ref}><button>only</button></div>
    }
    render(<Wrapper />)
    expect(screen.getByText('only')).toBeInTheDocument()
    expect(document.activeElement).toBe(document.body)
  })

  it('ignores non-Tab keys and leaves mid-list focus alone', async () => {
    const Wrapper = () => {
      const ref = useFocusTrap(true)
      return (
        <div ref={ref}>
          <button>first</button>
          <button>second</button>
          <button>last</button>
        </div>
      )
    }
    const { container } = render(<Wrapper />)
    await waitFor(() => expect(document.activeElement?.textContent).toBe('first'))

    fireEvent.keyDown(container.firstChild as HTMLElement, { key: 'Enter' })
    expect(document.activeElement?.textContent).toBe('first')

    const second = screen.getByText('second') as HTMLElement
    second.focus()
    fireEvent.keyDown(container.firstChild as HTMLElement, { key: 'Tab' })
    expect(document.activeElement?.textContent).toBe('second')
    fireEvent.keyDown(container.firstChild as HTMLElement, { key: 'Tab', shiftKey: true })
    expect(document.activeElement?.textContent).toBe('second')
  })
})

describe('useKeyboardNavigation', () => {
  it('moves focusedIndex with arrows, Home/End, and wraps when loop=true', () => {
    const onSelect = vi.fn()
    const { result } = renderHook(() => useKeyboardNavigation(3, onSelect, { loop: true }))

    expect(result.current.focusedIndex).toBe(0)

    act(() => result.current.onKeyDown({ key: 'ArrowDown', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(1)

    act(() => result.current.onKeyDown({ key: 'ArrowDown', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(2)

    act(() => result.current.onKeyDown({ key: 'ArrowDown', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(0)

    act(() => result.current.onKeyDown({ key: 'End', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(2)

    act(() => result.current.onKeyDown({ key: 'Home', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(0)
  })

  it('calls onSelect on Enter/Space with current index', () => {
    const onSelect = vi.fn()
    const { result } = renderHook(() => useKeyboardNavigation(3, onSelect))
    act(() => result.current.setFocusedIndex(2))
    act(() => result.current.onKeyDown({ key: 'Enter', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(onSelect).toHaveBeenCalledWith(2)
    act(() => result.current.onKeyDown({ key: ' ', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(onSelect).toHaveBeenCalledWith(2)
  })

it('calls onEscape and respects loop=false and horizontal orientation', () => {
     const onEscape = vi.fn()
     const onSelect = vi.fn()
     const { result } = renderHook(() => useKeyboardNavigation(3, onSelect, { loop: false, orientation: 'horizontal', onEscape }))
     act(() => result.current.onKeyDown({ key: 'Escape' } as unknown as KeyboardEvent))
     expect(onEscape).toHaveBeenCalledTimes(1)

     act(() => result.current.setFocusedIndex(0))
     act(() => result.current.onKeyDown({ key: 'ArrowLeft', preventDefault: vi.fn() } as unknown as KeyboardEvent))
     expect(result.current.focusedIndex).toBe(0)

     act(() => result.current.onKeyDown({ key: 'ArrowRight', preventDefault: vi.fn() } as unknown as KeyboardEvent))
     expect(result.current.focusedIndex).toBe(1)
   })

   it('escape handler invokes the LATEST onSelect after parent re-render', () => {
     const onSelect1 = vi.fn()
     const onSelect2 = vi.fn()
     const { result, rerender } = renderHook(
       ({ onSelect }) => useKeyboardNavigation(3, onSelect, { loop: true }),
       { initialProps: { onSelect: onSelect1 } }
     )
     act(() => result.current.onKeyDown({ key: 'Enter', preventDefault: vi.fn() } as unknown as KeyboardEvent))
     expect(onSelect1).toHaveBeenCalledWith(0)
     expect(onSelect2).not.toHaveBeenCalled()

     rerender({ onSelect: onSelect2 })
     act(() => result.current.onKeyDown({ key: 'Enter', preventDefault: vi.fn() } as unknown as KeyboardEvent))
     expect(onSelect2).toHaveBeenCalledWith(0)
   })

  it('wraps to last on ArrowUp at start when loop=true', () => {
    const { result } = renderHook(() => useKeyboardNavigation(3, vi.fn(), { loop: true }))
    act(() => result.current.onKeyDown({ key: 'ArrowUp', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(2)
    act(() => result.current.onKeyDown({ key: 'ArrowUp', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(1)
  })

  it('clamps at end without loop and ignores Escape/unknown keys', () => {
    const { result } = renderHook(() => useKeyboardNavigation(3, vi.fn(), { loop: false }))
    act(() => result.current.setFocusedIndex(2))
    act(() => result.current.onKeyDown({ key: 'ArrowDown', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(2)
    act(() => result.current.onKeyDown({ key: 'Escape' } as unknown as KeyboardEvent))
    act(() => result.current.onKeyDown({ key: 'x', preventDefault: vi.fn() } as unknown as KeyboardEvent))
    expect(result.current.focusedIndex).toBe(2)
  })
})

describe('useAnnouncer', () => {
  it('creates live region and announce schedules message', async () => {
    const { result } = renderHook(() => useAnnouncer())
    expect(document.querySelector('[aria-live="polite"]')).toBeTruthy()

    act(() => result.current.announce('hello'))
    await waitFor(() => expect(document.body.textContent).toContain('hello'), { timeout: 200 })
  })

  it('announce after unmount does not crash', async () => {
    const { result, unmount } = renderHook(() => useAnnouncer())
    unmount()
    act(() => result.current.announce('late'))
    await new Promise((r) => setTimeout(r, 10))
  })
})
