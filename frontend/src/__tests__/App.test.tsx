import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import App from '../App'
import { useSettingsStore } from '../stores/settingsStore'

describe('App', () => {
  it('renders shell with navigation tabs and header', async () => {
    render(<App />)
    expect(await screen.findByText('Vortex AI')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /الشات/ })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /لوحة التحكم/ })).toBeInTheDocument()
    expect(screen.getByRole('banner')).toBeInTheDocument()
  })

  it('switches main heading between chat and dashboard via sidebar', async () => {
    render(<App />)
    expect(await screen.findByText('الشات الذكي')).toBeInTheDocument()
    screen.getByRole('tab', { name: /لوحة التحكم/ }).click()
    expect(await screen.findByText('لوحة التحكم')).toBeInTheDocument()
  })

  it('opens and closes sidebar via header menu and overlay', async () => {
    render(<App />)
    await screen.findByText('Vortex AI')
    expect(document.querySelector('.fixed.inset-0.bg-black\\/50')).not.toBeInTheDocument()

    fireEvent.click(screen.getByLabelText('فتح القائمة'))
    const overlay = document.querySelector('.fixed.inset-0.bg-black\\/50') as HTMLElement
    expect(overlay).toBeInTheDocument()

    fireEvent.click(overlay)
    expect(document.querySelector('.fixed.inset-0.bg-black\\/50')).not.toBeInTheDocument()
  })

  it('applies ltr direction for english', async () => {
    useSettingsStore.setState({ language: 'en' } as never)
    render(<App />)
    await screen.findByText('Vortex AI')
    expect(document.documentElement.dir).toBe('ltr')
    useSettingsStore.setState({ language: 'ar' } as never)
  })
})
