import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { useDashboard } from '../useDashboard'
import { useDashboardStore } from '../../stores/dashboardStore'

const hoisted = vi.hoisted(() => ({
  api: {
    health: vi.fn(),
    models: vi.fn(),
    swapModel: vi.fn(),
    tools: { list: vi.fn(), execute: vi.fn() },
    knowledge: { search: vi.fn(), import: vi.fn() },
  },
}))

vi.mock('../../services/api', () => ({ api: hoisted.api }))

const healthPayload = {
  status: 'ok',
  architecture: '5km',
  device: 'cpu',
  simd: 'avx2',
  history_length: 3,
  uptime_seconds: 120,
  knowledge_chunks: 9,
}

const modelInfoPayload = {
  architecture: 'qwen2.5-0.5b',
  max_seq_len: 32768,
  max_generation_tokens: 8192,
}

describe('useDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useDashboardStore.setState({
      health: null,
      models: null,
      tools: null,
      currentModel: null,
      searchResults: null,
      logs: [],
    })
    hoisted.api.health.mockResolvedValue(healthPayload)
    hoisted.api.models.mockResolvedValue(modelInfoPayload)
    hoisted.api.tools.list.mockResolvedValue({ tools: [{ name: 'calc', description: 'd' }] })
  })

  it('auto-fetches health, models and tools on mount', async () => {
    const { result } = renderHook(() => useDashboard())

    await waitFor(() => expect(result.current.health?.status).toBe('ok'))
    await waitFor(() => expect(result.current.models?.[0].architecture).toBe('qwen2.5-0.5b'))
    await waitFor(() => expect(result.current.tools).toHaveLength(1))
    expect(result.current.currentModel).toBe('qwen2.5-0.5b')
    expect(result.current.logs.some(l => l.includes('Health check'))).toBe(true)
  })

  it('normalizes the single ModelInfo object into a list', async () => {
    const { result } = renderHook(() => useDashboard())
    await waitFor(() => expect(result.current.models).not.toBeNull())

    expect(result.current.models).toEqual([
      {
        name: 'qwen2.5-0.5b',
        architecture: 'qwen2.5-0.5b',
        max_seq_len: 32768,
        max_generation_tokens: 8192,
      },
    ])
  })

  it('logs failures without crashing when health fetch rejects', async () => {
    hoisted.api.health.mockRejectedValue(new Error('boom'))

    renderHook(() => useDashboard())

    await waitFor(() =>
      expect(useDashboardStore.getState().logs.some(l => l.includes('failed'))).toBe(true)
    )
    expect(useDashboardStore.getState().health).toBeNull()
  })

  it('swapModel updates current model and refetches models', async () => {
    const { result } = renderHook(() => useDashboard())
    await waitFor(() => expect(result.current.models).not.toBeNull())

    hoisted.api.swapModel.mockResolvedValue({ status: 'ok' })

    await act(async () => {
      await result.current.swapModel('new-model')
    })

    expect(hoisted.api.swapModel).toHaveBeenCalledWith({ model: 'new-model' })
    expect(useDashboardStore.getState().logs.some(l => l.includes('Model swapped'))).toBe(true)
    expect(useDashboardStore.getState().currentModel).toBe('qwen2.5-0.5b')
    expect(result.current.loading).toBe(false)
  })

  it('searchKnowledge stores results and logs count', async () => {
    hoisted.api.knowledge.search.mockResolvedValue({
      results: [{ id: 'c1', score: 0.9, text: 't' }],
    })

    const { result } = renderHook(() => useDashboard())

    await act(async () => {
      await result.current.searchKnowledge('استعلام')
    })

    expect(useDashboardStore.getState().searchResults).toHaveLength(1)
    expect(
      useDashboardStore.getState().logs.some(l => l.includes('Knowledge search'))
    ).toBe(true)
  })

  it('executeTool returns payload and logs duration', async () => {
    hoisted.api.tools.execute.mockResolvedValue({ result: 'done' })

    const { result } = renderHook(() => useDashboard())

    let out: unknown
    await act(async () => {
      out = await result.current.executeTool('calc', { x: 1 })
    })

    expect(out).toEqual({ result: 'done' })
    expect(
      useDashboardStore.getState().logs.some(l => l.includes('Tool calc executed'))
    ).toBe(true)
  })

  it('refreshAll runs all fetchers and clears loading', async () => {
    const { result } = renderHook(() => useDashboard())

    await act(async () => {
      await result.current.refreshAll()
    })

    expect(hoisted.api.health).toHaveBeenCalled()
    expect(result.current.loading).toBe(false)
  })

  it('importKnowledge logs chunks and refreshes health', async () => {
    hoisted.api.knowledge.import.mockResolvedValue({ chunks_imported: 5 })

    const { result } = renderHook(() => useDashboard())

    await act(async () => {
      await result.current.importKnowledge('نص للاستيراد')
    })

    expect(hoisted.api.knowledge.import).toHaveBeenCalledWith({ text: 'نص للاستيراد' })
    expect(useDashboardStore.getState().logs.some(l => l.includes('Knowledge imported: 5 chunks'))).toBe(true)
    expect(hoisted.api.health).toHaveBeenCalled()
  })

  it('logs failures for import/models/tools/swap/search/execute without crashing', async () => {
    hoisted.api.knowledge.import.mockRejectedValue(new Error('imp'))
    hoisted.api.models.mockRejectedValue(new Error('m'))
    hoisted.api.tools.list.mockRejectedValue(new Error('t'))
    hoisted.api.knowledge.search.mockRejectedValue(new Error('s'))
    hoisted.api.tools.execute.mockRejectedValue(new Error('e'))
    hoisted.api.swapModel.mockRejectedValue(new Error('sw'))

    const { result } = renderHook(() => useDashboard())

    await waitFor(() =>
      expect(useDashboardStore.getState().logs.some(l => l.includes('Models fetch failed'))).toBe(true)
    )
    await waitFor(() =>
      expect(useDashboardStore.getState().logs.some(l => l.includes('Tools fetch failed'))).toBe(true)
    )

    await act(async () => {
      await result.current.importKnowledge('x')
    })
    await act(async () => {
      await result.current.searchKnowledge('q')
    })
    await act(async () => {
      await result.current.executeTool('calc', {}).catch(() => {})
    })
    await act(async () => {
      await result.current.swapModel('x')
    })

    const logs = useDashboardStore.getState().logs.join('\n')
    expect(logs).toContain('Knowledge import failed')
    expect(logs).toContain('Knowledge search failed')
    expect(logs).toContain('Tool calc failed')
    expect(logs).toContain('Model swap failed')
    expect(result.current.loading).toBe(false)
  })

  it('skips current model when architecture is empty', async () => {
    hoisted.api.models.mockResolvedValue({ architecture: '', max_seq_len: 1024, max_generation_tokens: 256 })

    const { result } = renderHook(() => useDashboard())

    await waitFor(() => expect(result.current.models).not.toBeNull())
    expect(result.current.models?.[0].architecture).toBe('')
    expect(useDashboardStore.getState().currentModel).toBeNull()
  })
})
