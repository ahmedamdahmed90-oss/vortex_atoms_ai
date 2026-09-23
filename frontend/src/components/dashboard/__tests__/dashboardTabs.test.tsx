import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { OverviewTab } from '../OverviewTab'
import { ModelsTab } from '../ModelsTab'
import { LogsTab } from '../LogsTab'
import { ToastProvider } from '../../ui/Toast'

const hoisted = vi.hoisted(() => ({
  dash: {
    health: null as Record<string, unknown> | null,
    loading: false,
    fetchHealth: vi.fn(),
    fetchModels: vi.fn(),
    models: null as Array<Record<string, unknown>> | null,
    currentModel: null as string | null,
    swapModel: vi.fn(),
    logs: [] as string[],
    logFilter: 'all' as string,
    setLogFilter: vi.fn(),
    clearLogs: vi.fn(),
  },
}))

vi.mock('../../../hooks/useDashboard', () => ({ useDashboard: () => hoisted.dash }))

describe('OverviewTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.dash.loading = false
    hoisted.dash.health = null
  })

  it('shows empty state with retry wired to fetchHealth', () => {
    render(<ToastProvider><OverviewTab /></ToastProvider>)
    expect(screen.getByText('لا توجد بيانات حالة الخادم')).toBeInTheDocument()

    fireEvent.click(screen.getByText('تحديث'))
    expect(hoisted.dash.fetchHealth).toHaveBeenCalledTimes(1)
  })

  it('shows loading spinner when loading without health', () => {
    hoisted.dash.loading = true
    hoisted.dash.health = null
    const { container } = render(<ToastProvider><OverviewTab /></ToastProvider>)
    expect(container.querySelector('.animate-spin')).toBeInTheDocument()
  })

  it('renders stats and connection info when health exists', () => {
    hoisted.dash.health = {
      status: 'ok',
      version: '0.2.3',
      build_ts: '1758615338',
      git_sha: '3f3c47f',
      architecture: '5km',
      device: 'cpu',
      simd: 'avx2',
      history_length: 2,
      uptime_seconds: 3660,
      knowledge_chunks: 5,
    }

    render(<ToastProvider><OverviewTab /></ToastProvider>)

    expect(screen.getByText('متصل')).toBeInTheDocument()
    expect(screen.getAllByText('5km').length).toBeGreaterThan(0)
    expect(screen.getAllByText('قطع المعرفة').length).toBeGreaterThan(0)
    expect(screen.getByText(/1س/)).toBeInTheDocument()
  })
})

describe('ModelsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.dash.models = null
    hoisted.dash.currentModel = null
    hoisted.dash.loading = false
  })

  it('renders the management title', () => {
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getByText('إدارة النماذج')).toBeInTheDocument()
  })

  it('lists available model with details', () => {
    hoisted.dash.models = [
      { name: 'qwen', architecture: 'qwen', max_seq_len: 32768, max_generation_tokens: 8192 },
    ]
    hoisted.dash.currentModel = 'qwen'

    render(<ToastProvider><ModelsTab /></ToastProvider>)

    expect(screen.getByText('النموذج الحالي')).toBeInTheDocument()
    expect(screen.getAllByText(/qwen/).length).toBeGreaterThan(0)
  })

  it('shows loading state when currentModel is null', () => {
    hoisted.dash.currentModel = null
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getByText('جاري تحميل معلومات النموذج...')).toBeInTheDocument()
  })

  it('shows empty message when no models available', () => {
    hoisted.dash.models = []
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getByText('لا توجد نماذج متاحة')).toBeInTheDocument()
  })

  it('shows active badge for current model', () => {
    hoisted.dash.models = [
      { name: 'qwen', architecture: 'qwen', max_seq_len: 1024, max_generation_tokens: 512 },
      { name: 'other', architecture: 'other', max_seq_len: 512, max_generation_tokens: 256 },
    ]
    hoisted.dash.currentModel = 'qwen'
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getByText('نشط')).toBeInTheDocument()
  })

  it('refresh button calls fetchModels', () => {
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    fireEvent.click(screen.getByText('تحديث'))
    expect(hoisted.dash.fetchModels).toHaveBeenCalledTimes(1)
  })

  it('opens swap modal and executes swap successfully', async () => {
    hoisted.dash.models = [
      { name: 'qwen', architecture: 'qwen', max_seq_len: 1024, max_generation_tokens: 512 },
    ]
    hoisted.dash.currentModel = 'qwen'
    hoisted.dash.swapModel.mockResolvedValue(undefined)
    render(<ToastProvider><ModelsTab /></ToastProvider>)

    fireEvent.click(screen.getByRole('button', { name: 'تبديل النموذج' }))
    expect(screen.getByText('تبديل النموذج', { selector: 'h2' }) || screen.getByText('تبديل النموذج')).toBeInTheDocument()
    const options = screen.getAllByText('qwen')
    fireEvent.click(options[options.length - 1].closest('button')!)
    const confirm = screen.getAllByRole('button', { name: 'تبديل النموذج' }).pop()!
    await waitFor(() => expect(confirm).toBeEnabled())
    fireEvent.click(confirm)
    await waitFor(() => expect(hoisted.dash.swapModel).toHaveBeenCalledWith('qwen'))
  })

  it('shows error toast when swap fails', async () => {
    hoisted.dash.models = [{ name: 'qwen', architecture: 'qwen', max_seq_len: 1024, max_generation_tokens: 512 }]
    hoisted.dash.currentModel = 'qwen'
    hoisted.dash.swapModel.mockRejectedValue(new Error('fail'))
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    fireEvent.click(screen.getByRole('button', { name: 'تبديل النموذج' }))
    const qwenBtn = screen.getAllByText('qwen').pop()!.closest('button')!
    fireEvent.click(qwenBtn)
    const confirm = screen.getAllByRole('button', { name: 'تبديل النموذج' }).pop()!
    fireEvent.click(confirm)
    await waitFor(() => expect(hoisted.dash.swapModel).toHaveBeenCalled())
  })

  it('row swap button selects and opens modal; cancel and X close it', async () => {    hoisted.dash.models = [
      { name: 'qwen', architecture: 'qwen', max_seq_len: 1024, max_generation_tokens: 512 },
      { name: 'other', architecture: 'other', max_seq_len: 512, max_generation_tokens: 256 },
    ]
    hoisted.dash.currentModel = 'qwen'
    render(<ToastProvider><ModelsTab /></ToastProvider>)

    fireEvent.click(screen.getAllByRole('button', { name: 'تبديل' })[1])
    expect(screen.getByText('تبديل النموذج', { selector: 'h2' })).toBeInTheDocument()
    const confirm = screen.getAllByRole('button', { name: 'تبديل النموذج' }).pop()!
    await waitFor(() => expect(confirm).toBeEnabled())

    fireEvent.click(screen.getByRole('button', { name: 'إلغاء' }))
    expect(screen.queryByText('تبديل النموذج', { selector: 'h2' })).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'تبديل النموذج' }))
    const confirm2 = screen.getAllByRole('button', { name: 'تبديل النموذج' }).pop()!
    expect(confirm2).toBeEnabled()
    fireEvent.click(screen.getByLabelText('إغلاق'))
    expect(screen.queryByText('تبديل النموذج', { selector: 'h2' })).not.toBeInTheDocument()
  })

  it('ignores swap when no model is selected', async () => {
    hoisted.dash.models = [{ name: 'qwen', architecture: 'qwen', max_seq_len: 1024, max_generation_tokens: 512 }]
    hoisted.dash.currentModel = 'qwen'
    render(<ToastProvider><ModelsTab /></ToastProvider>)

    fireEvent.click(screen.getByRole('button', { name: 'تبديل النموذج' }))
    const confirm = screen.getAllByRole('button', { name: 'تبديل النموذج' }).pop()!
    fireEvent.click(confirm)
    expect(hoisted.dash.swapModel).not.toHaveBeenCalled()
  })

  it('wraps a single non-array model object into a list', () => {
    hoisted.dash.models = { name: 'solo', architecture: 'solo', max_seq_len: 2048, max_generation_tokens: 512 } as never
    hoisted.dash.currentModel = 'solo'
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getByText('النماذج المتاحة')).toBeInTheDocument()
    expect(screen.getAllByText('solo').length).toBeGreaterThan(0)
  })

  it('renders models with missing token limits', () => {
    hoisted.dash.models = [{ name: 'bare', architecture: 'bare' }] as never
    hoisted.dash.currentModel = 'bare'
    render(<ToastProvider><ModelsTab /></ToastProvider>)
    expect(screen.getAllByText('bare').length).toBeGreaterThan(0)
    expect(screen.getByText(/seq •/)).toBeInTheDocument()
  })
})

describe('LogsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    hoisted.dash.logs = []
    hoisted.dash.logFilter = 'all'
  })

  it('shows empty state for no logs', () => {
    render(<ToastProvider><LogsTab /></ToastProvider>)
    expect(screen.getByText('لا توجد سجلات')).toBeInTheDocument()
  })

  it('renders entries, count header and wires clear button', () => {
    hoisted.dash.logs = ['[10:00] INFO started', '[10:01] ERROR crashed']

    render(<ToastProvider><LogsTab /></ToastProvider>)

    expect(screen.getByText('السجلات (2)')).toBeInTheDocument()
    expect(screen.getByText(/INFO started/)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /مسح/ }))
    expect(hoisted.dash.clearLogs).toHaveBeenCalledTimes(1)
  })

  it('filter select delegates to setLogFilter', () => {
    render(<ToastProvider><LogsTab /></ToastProvider>)

    const select = screen.getAllByRole('combobox')[0] as HTMLSelectElement
    fireEvent.change(select, { target: { value: 'error' } })
    expect(hoisted.dash.setLogFilter).toHaveBeenCalledWith('error')
  })

  it('toggles auto-scroll checkbox', () => {
    hoisted.dash.logs = ['[10:00] INFO started']
    render(<ToastProvider><LogsTab /></ToastProvider>)

    const box = screen.getByRole('checkbox') as HTMLInputElement
    expect(box).toBeChecked()
    fireEvent.click(box)
    expect(box).not.toBeChecked()
    fireEvent.click(box)
    expect(box).toBeChecked()
  })

  it('filters entries by logFilter across severities', () => {
    hoisted.dash.logs = ['[t] ERROR boom', '[t] WARN careful', '[t] INFO hi', '[t] debug trace']
    hoisted.dash.logFilter = 'error'
    const { rerender } = render(<ToastProvider><LogsTab /></ToastProvider>)
    expect(screen.getByText(/ERROR boom/)).toBeInTheDocument()
    expect(screen.queryByText(/INFO hi/)).not.toBeInTheDocument()

    hoisted.dash.logFilter = 'all'
    rerender(<ToastProvider><LogsTab /></ToastProvider>)
    expect(screen.getByText(/WARN careful/)).toBeInTheDocument()
    expect(screen.getByText(/INFO hi/)).toBeInTheDocument()
    expect(screen.getByText(/debug trace/)).toBeInTheDocument()
  })
})

