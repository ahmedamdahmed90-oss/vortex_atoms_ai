import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Sidebar, Header } from '../Header'
import { RTLLayout } from '../RTLLayout'
import { useSettingsStore } from '../../../stores/settingsStore'

const tabs = [
  { id: 'chat', label: 'الشات', icon: <span>💬</span> },
  { id: 'dashboard', label: 'لوحة التحكم', icon: <span>📊</span> },
]

describe('Sidebar', () => {
  it('renders tabs, marks active, and fires onTabChange', () => {
    const onTabChange = vi.fn()
    const onClose = vi.fn()
    render(<Sidebar isOpen onClose={onClose} activeTab="chat" onTabChange={onTabChange} tabs={tabs} />)
    expect(screen.getByText('Vortex AI')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /الشات/ })).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByRole('tab', { name: /لوحة التحكم/ })).toHaveAttribute('aria-selected', 'false')
    fireEvent.click(screen.getByRole('tab', { name: /لوحة التحكم/ }))
    expect(onTabChange).toHaveBeenCalledWith('dashboard')
  })

  it('overlay click and close button call onClose', () => {
    const onClose = vi.fn()
    render(<Sidebar isOpen onClose={onClose} activeTab="chat" onTabChange={() => {}} tabs={tabs} />)
    const overlay = document.querySelector('.fixed.inset-0.bg-black\\/50') as HTMLElement
    expect(overlay).toBeInTheDocument()
    fireEvent.click(overlay)
    expect(onClose).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByLabelText('إغلاق الشريط الجانبي'))
    expect(onClose).toHaveBeenCalledTimes(2)
  })

  it('hides overlay when closed', () => {
    render(<Sidebar isOpen={false} onClose={() => {}} activeTab="chat" onTabChange={() => {}} tabs={tabs} />)
    expect(document.querySelector('.fixed.inset-0.bg-black\\/50')).not.toBeInTheDocument()
  })
})

describe('Header', () => {
  it('renders title and menu button', () => {
    const onMenuClick = vi.fn()
    render(<Header onMenuClick={onMenuClick} title="عنوان" />)
    expect(screen.getByText('عنوان')).toBeInTheDocument()
    fireEvent.click(screen.getByLabelText('فتح القائمة'))
    expect(onMenuClick).toHaveBeenCalledTimes(1)
  })

  it('language menu toggles and shows options', () => {
    render(<Header onMenuClick={() => {}} title="t" />)
    fireEvent.click(screen.getByLabelText('اللغة'))
    expect(screen.getAllByText('العربية').length).toBeGreaterThanOrEqual(2)
    expect(screen.getByText('English')).toBeInTheDocument()
  })

  it('positions sidebar on the left for english', () => {
    useSettingsStore.setState({ language: 'en' } as never)
    const { container } = render(<Sidebar isOpen onClose={() => {}} activeTab="chat" onTabChange={() => {}} tabs={tabs} />)
    expect(container.querySelector('aside')?.className).toContain('left-0')
    useSettingsStore.setState({ language: 'ar' } as never)
  })

  it('selecting a language option switches language and closes menu', () => {
    useSettingsStore.setState({ language: 'ar' } as never)
    render(<Header onMenuClick={() => {}} title="t" />)
    fireEvent.click(screen.getByLabelText('اللغة'))
    fireEvent.click(screen.getByText('English').closest('button')!)
    expect(useSettingsStore.getState().language).toBe('en')
    expect(screen.queryByText('🇺🇸')).not.toBeInTheDocument()
    useSettingsStore.setState({ language: 'ar' } as never)
  })

  it('closes language and theme menus via backdrop', () => {
    render(<Header onMenuClick={() => {}} title="t" />)
    fireEvent.click(screen.getByLabelText('اللغة'))
    expect(screen.getByText('English')).toBeInTheDocument()
    fireEvent.click(document.querySelector('.fixed.inset-0.z-40') as HTMLElement)
    expect(screen.queryByText('English')).not.toBeInTheDocument()

    fireEvent.click(screen.getByLabelText('الوضع الداكن'))
    expect(screen.getByText('النظام')).toBeInTheDocument()
    fireEvent.click(document.querySelector('.fixed.inset-0.z-40') as HTMLElement)
    expect(screen.queryByText('النظام')).not.toBeInTheDocument()
  })

  it('theme menu opens and selecting dark applies theme', () => {
    useSettingsStore.setState({ theme: 'light' } as never)
    render(<Header onMenuClick={() => {}} title="t" />)
    fireEvent.click(screen.getByLabelText('الوضع الداكن'))
    expect(screen.getByText('فاتح')).toBeInTheDocument()
    expect(screen.getByText('داكن')).toBeInTheDocument()
    expect(screen.getByText('النظام')).toBeInTheDocument()
    fireEvent.click(screen.getByText('داكن'))
    expect(useSettingsStore.getState().theme).toBe('dark')
    useSettingsStore.setState({ theme: 'system' } as never)
  })

  it('renders custom actions', () => {
    render(<Header onMenuClick={() => {}} title="t" actions={<button>إجراء</button>} />)
    expect(screen.getByText('إجراء')).toBeInTheDocument()
  })
})

describe('RTLLayout', () => {
  it('applies dir based on language and renders slots', () => {
    useSettingsStore.setState({ language: 'ar' } as never)
    const { container } = render(
      <RTLLayout sidebar={<div>side</div>} header={<div>head</div>} sidebarOpen={true} onSidebarToggle={() => {}}>
        <div>content</div>
      </RTLLayout>
    )
    expect(container.firstChild).toHaveAttribute('dir', 'rtl')
    expect(screen.getByText('side')).toBeInTheDocument()
    expect(screen.getByText('head')).toBeInTheDocument()
    expect(screen.getByText('content')).toBeInTheDocument()

    useSettingsStore.setState({ language: 'en' } as never)
    const { container: c2 } = render(
      <RTLLayout sidebar={<div>s2</div>} header={<div>h2</div>}>
        <div>c2</div>
      </RTLLayout>
    )
    expect(c2.firstChild).toHaveAttribute('dir', 'ltr')
    useSettingsStore.setState({ language: 'ar' } as never)
  })
})
