import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { KnowledgeTab } from '../KnowledgeTab'
import { ToolsTab } from '../ToolsTab'
import { ToastProvider } from '../../ui/Toast'

const hoisted = vi.hoisted(() => ({
  dash: {
    knowledgeChunks: 12,
    searchResults: null as Array<{ id: string; score: number; text: string }> | null,
    searchKnowledge: vi.fn(),
    importKnowledge: vi.fn(),
    tools: null as Array<{ name: string; description: string }> | null,
    executeTool: vi.fn(),
    fetchTools: vi.fn(),
    refreshAll: vi.fn(),
  },
}))

vi.mock('../../../hooks/useDashboard', () => ({ useDashboard: () => hoisted.dash }))

const renderWithToast = (ui: React.ReactElement) => render(<ToastProvider>{ui}</ToastProvider>)

describe('KnowledgeTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.dash.searchResults = null
    hoisted.dash.knowledgeChunks = 12
  })

  it('renders title, search input and knowledge stats', () => {
    renderWithToast(<KnowledgeTab />)
    expect(screen.getByText('قاعدة المعرفة')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('ابحث في المعرفة...')).toBeInTheDocument()
    expect(screen.getByText('12')).toBeInTheDocument()
  })

  it('triggers searchKnowledge on button and Enter key', async () => {
    renderWithToast(<KnowledgeTab />)
    const input = screen.getByPlaceholderText('ابحث في المعرفة...') as HTMLInputElement

    fireEvent.change(input, { target: { value: 'استعلام' } })
    fireEvent.click(screen.getByRole('button', { name: /^بحث/ }))
    expect(hoisted.dash.searchKnowledge).toHaveBeenCalledWith('استعلام')

    fireEvent.change(input, { target: { value: 'سؤال ثان' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(hoisted.dash.searchKnowledge).toHaveBeenCalledWith('سؤال ثان')
  })

  it('ignores empty search queries', () => {
    renderWithToast(<KnowledgeTab />)
    fireEvent.click(screen.getByRole('button', { name: /^بحث/ }))
    expect(hoisted.dash.searchKnowledge).not.toHaveBeenCalled()
  })

  it('shows results with scores formatted to 3 decimals', () => {
    hoisted.dash.searchResults = [{ id: 'c1', score: 0.98765, text: 'نتيجة تجريبية' }]
    renderWithToast(<KnowledgeTab />)
    expect(screen.getByText('0.988')).toBeInTheDocument()
    expect(screen.getByText('نتيجة تجريبية')).toBeInTheDocument()
  })

  it('shows no-results hint when search returns empty', () => {
    hoisted.dash.searchResults = []
    renderWithToast(<KnowledgeTab />)
    expect(screen.getByText('لا توجد نتائج للبحث')).toBeInTheDocument()
  })

  it('opens import modal, validates, and calls importKnowledge', async () => {
    hoisted.dash.importKnowledge.mockResolvedValue(undefined)
    renderWithToast(<KnowledgeTab />)

    fireEvent.click(screen.getByText('استيراد معرفة'))
    expect(screen.getByText('استيراد معرفة جديدة')).toBeInTheDocument()

    const submit = screen.getByRole('button', { name: 'استيراد' })
    expect(submit).toBeDisabled()

    const ta = screen.getByPlaceholderText('أدخل النص الذي تريد إضافته إلى قاعدة المعرفة...') as HTMLTextAreaElement
    fireEvent.change(ta, { target: { value: 'نص للاستيراد' } })
    expect(submit).toBeEnabled()

    fireEvent.click(submit)
    await waitFor(() => expect(hoisted.dash.importKnowledge).toHaveBeenCalledWith('نص للاستيراد'))
  })

  it('ignores import submit with empty text', () => {
    hoisted.dash.importKnowledge.mockResolvedValue(undefined)
    renderWithToast(<KnowledgeTab />)

    fireEvent.click(screen.getByText('استيراد معرفة'))
    fireEvent.click(screen.getByRole('button', { name: 'استيراد' }))
    expect(hoisted.dash.importKnowledge).not.toHaveBeenCalled()
  })

  it('survives import rejection and keeps modal open', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    hoisted.dash.importKnowledge.mockRejectedValue(new Error('imp-fail'))
    renderWithToast(<KnowledgeTab />)

    fireEvent.click(screen.getByText('استيراد معرفة'))
    fireEvent.change(screen.getByPlaceholderText('أدخل النص الذي تريد إضافته إلى قاعدة المعرفة...'), { target: { value: 'نص' } })
    fireEvent.click(screen.getByRole('button', { name: 'استيراد' }))

    await waitFor(() => expect(hoisted.dash.importKnowledge).toHaveBeenCalledWith('نص'))
    expect(spy).toHaveBeenCalled()
    expect(screen.getByText('استيراد معرفة جديدة')).toBeInTheDocument()
    spy.mockRestore()
  })

  it('closes import modal via cancel and X', () => {    renderWithToast(<KnowledgeTab />)

    fireEvent.click(screen.getByText('استيراد معرفة'))
    expect(screen.getByText('استيراد معرفة جديدة')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'إلغاء' }))
    expect(screen.queryByText('استيراد معرفة جديدة')).not.toBeInTheDocument()

    fireEvent.click(screen.getByText('استيراد معرفة'))
    fireEvent.click(screen.getByLabelText('إغلاق'))
    expect(screen.queryByText('استيراد معرفة جديدة')).not.toBeInTheDocument()
  })

  it('refresh button re-fetches dashboard stats', () => {
    hoisted.dash.refreshAll.mockResolvedValue(undefined)
    renderWithToast(<KnowledgeTab />)
    fireEvent.click(screen.getByRole('button', { name: 'تحديث' }))
    expect(hoisted.dash.refreshAll).toHaveBeenCalledTimes(1)
  })
})

describe('ToolsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.dash.tools = null
  })

  it('shows empty state when no tools', () => {
    renderWithToast(<ToolsTab />)
    expect(screen.getByText('لا توجد أدوات متاحة')).toBeInTheDocument()
    expect(screen.getByText('الأدوات')).toBeInTheDocument()
  })

  it('lists tools and opens execution modal', () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'calculator tool' }]
    renderWithToast(<ToolsTab />)

    expect(screen.getByText('calc')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    expect(screen.getByText('تنفيذ: calc')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('{"param": "value"}')).toBeInTheDocument()
  })

  it('executes a tool with parsed JSON args and shows result modal', async () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    hoisted.dash.executeTool.mockResolvedValue({ output: 42 })
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    const ta = screen.getByPlaceholderText('{"param": "value"}') as HTMLTextAreaElement
    fireEvent.change(ta, { target: { value: '{"x":1}' } })

    const execButtons = screen.getAllByRole('button', { name: 'تنفيذ' })
    fireEvent.click(execButtons[execButtons.length - 1])

    await waitFor(() => expect(hoisted.dash.executeTool).toHaveBeenCalledWith('calc', { x: 1 }))
    await waitFor(() => expect(screen.getByText('نتيجة التنفيذ')).toBeInTheDocument())
    expect(screen.getByText(/"output": 42/)).toBeInTheDocument()
  })

  it('handles invalid JSON args and shows error result', async () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    fireEvent.change(screen.getByPlaceholderText('{"param": "value"}'), { target: { value: '{bad' } })

    const execButtons = screen.getAllByRole('button', { name: 'تنفيذ' })
    fireEvent.click(execButtons[execButtons.length - 1])

    await waitFor(() => expect(screen.getByText('نتيجة التنفيذ')).toBeInTheDocument())
    expect(screen.getByText(/غير صالحة/)).toBeInTheDocument()
  })

  it('cancel button resets and closes the execution modal', () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    fireEvent.change(screen.getByPlaceholderText('{"param": "value"}'), { target: { value: '{"x":9}' } })
    fireEvent.click(screen.getByRole('button', { name: 'إلغاء' }))
    expect(screen.queryByText('تنفيذ: calc')).not.toBeInTheDocument()
  })

  it('shows error result when execution rejects and closes it', async () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    hoisted.dash.executeTool.mockRejectedValue(new Error('boom'))
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    const execButtons = screen.getAllByRole('button', { name: 'تنفيذ' })
    fireEvent.click(execButtons[execButtons.length - 1])

    await waitFor(() => expect(screen.getByText('نتيجة التنفيذ')).toBeInTheDocument())
    expect(screen.getByText(/خطأ:/)).toBeInTheDocument()

    const closers = screen.getAllByRole('button', { name: 'إغلاق' })
    fireEvent.click(closers[closers.length - 1])
    expect(screen.queryByText('نتيجة التنفيذ')).not.toBeInTheDocument()
  })

  it('modal X resets selection and args', () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    fireEvent.change(screen.getByPlaceholderText('{"param": "value"}'), { target: { value: '{"x":9}' } })
    fireEvent.click(screen.getByLabelText('إغلاق'))
    expect(screen.queryByText('تنفيذ: calc')).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    expect((screen.getByPlaceholderText('{"param": "value"}') as HTMLTextAreaElement).value).toBe('{}')
  })

  it('result modal X closes the result view', async () => {
    hoisted.dash.tools = [{ name: 'calc', description: 'd' }]
    hoisted.dash.executeTool.mockResolvedValue({ output: 1 })
    renderWithToast(<ToolsTab />)

    fireEvent.click(screen.getByRole('button', { name: 'تنفيذ' }))
    const execButtons = screen.getAllByRole('button', { name: 'تنفيذ' })
    fireEvent.click(execButtons[execButtons.length - 1])
    await waitFor(() => expect(screen.getByText('نتيجة التنفيذ')).toBeInTheDocument())

    const closers = screen.getAllByLabelText('إغلاق')
    fireEvent.click(closers[closers.length - 1])
    expect(screen.queryByText('نتيجة التنفيذ')).not.toBeInTheDocument()
  })
})
