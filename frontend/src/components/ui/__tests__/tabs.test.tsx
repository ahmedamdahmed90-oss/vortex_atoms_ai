import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Tabs, TabPanel } from '../Tabs'

const tabs = [
  { id: 'overview', label: 'نظرة عامة' },
  { id: 'models', label: 'النماذج' },
  { id: 'tools', label: 'الأدوات', icon: <span>🔧</span> },
]

describe('Tabs', () => {
  it('renders tablist with labels and marks active via aria-selected', () => {
    render(<Tabs tabs={tabs} activeTab="models" onChange={() => {}} />)
    expect(screen.getByRole('tablist')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /النماذج/ })).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByRole('tab', { name: /نظرة عامة/ })).toHaveAttribute('aria-selected', 'false')
  })

  it('calls onChange with tab id on click', () => {
    const onChange = vi.fn()
    render(<Tabs tabs={tabs} activeTab="overview" onChange={onChange} />)
    fireEvent.click(screen.getByRole('tab', { name: /الأدوات/ }))
    expect(onChange).toHaveBeenCalledWith('tools')
  })

  it('supports variant classes and icon rendering', () => {
    const { container } = render(<Tabs tabs={tabs} activeTab="overview" onChange={() => {}} variant="pills" className="extra" />)
    expect(container.firstChild).toHaveClass('extra')
    expect(screen.getByText('🔧')).toBeInTheDocument()
  })

  it('sets aria-controls and id linkage', () => {
    render(<Tabs tabs={tabs} activeTab="overview" onChange={() => {}} />)
    const tab = screen.getByRole('tab', { name: /نظرة عامة/ })
    expect(tab).toHaveAttribute('id', 'tab-overview')
    expect(tab).toHaveAttribute('aria-controls', 'panel-overview')
  })
})

describe('TabPanel', () => {
  it('renders children only when active', () => {
    const { rerender } = render(<TabPanel id="models" activeTab="overview"><div>panel-content</div></TabPanel>)
    expect(screen.queryByText('panel-content')).not.toBeInTheDocument()

    rerender(<TabPanel id="overview" activeTab="overview"><div>panel-content</div></TabPanel>)
    const panel = screen.getByRole('tabpanel')
    expect(panel).toHaveAttribute('id', 'panel-overview')
    expect(panel).toHaveAttribute('aria-labelledby', 'tab-overview')
    expect(screen.getByText('panel-content')).toBeInTheDocument()
  })
})
