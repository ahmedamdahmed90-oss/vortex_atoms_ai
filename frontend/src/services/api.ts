import type {
  ChatRequest,
  ChatResponse,
  GenerateRequest,
  GenerateResponse,
  EmbeddingRequest,
  EmbeddingResponse,
  KnowledgeSearchRequest,
  KnowledgeSearchResponse,
  KnowledgeImportRequest,
  KnowledgeImportResponse,
  HealthResponse,
  DeviceResponse,
  ModelInfo,
  SwapModelRequest,
  SwapModelResponse,
  ToolsListResponse,
  ToolExecuteRequest,
  ToolExecuteResponse,
  ToolCallRequest,
  ToolCallResponse,
  BatchRequest,
  BatchResponse,
  AuthBootstrapResponse,
  AdminStatusResponse,
  AuditLogResponse,
  RotateTokenResponse,
  AdminMetricsResponse,
  PerformanceKnobsResponse,
  PerformancePatchRequest,
  SessionsPurgeResponse,
} from '../types';

const API_BASE =
  import.meta.env.VITE_API_URL ||
  (import.meta.env.PROD ? '/v1' : 'http://localhost:8080/v1');
const DEFAULT_TIMEOUT = 30000;
const MAX_RETRIES = 3;
const RETRY_DELAY_BASE = 1000;

// ---------------------------------------------------------------------------
// Auth tokens (server bearer roles: user/admin).
//
// The bundled UI is same-origin with the server, so on first use it pulls the
// tokens from the loopback-only `/auth/bootstrap` endpoint and keeps them in
// memory (never localStorage: a stored bearer is stealable by any XSS).
// Remote browsers (non-loopback) get 403 from bootstrap and stay
// unauthenticated until the operator pastes a token (see setTokens).
// ---------------------------------------------------------------------------

let apiToken: string | null = null;
let adminToken: string | null = null;
let bootstrapDone = false;
let bootstrapFlight: Promise<void> | null = null;

/** Synchronous accessor for non-fetch consumers (e.g. WebSocket URL). */
export function getApiTokenSync(): string | null {
  return apiToken;
}

/** Manual override (remote deployments): paste tokens from the server host. */
export function setTokens(api: string | null, admin: string | null = null): void {
  apiToken = api;
  adminToken = admin;
  bootstrapDone = api !== null;
}

/** Test/rotation helper: drop cached tokens so the next call re-bootstraps. */
export function resetAuth(): void {
  apiToken = null;
  adminToken = null;
  bootstrapDone = false;
  bootstrapFlight = null;
}

function bootstrapUrl(): string {
  const base = API_BASE.endsWith('/v1') ? API_BASE.slice(0, -3) : API_BASE;
  return `${base}/auth/bootstrap`;
}

async function ensureTokens(): Promise<void> {
  if (bootstrapDone) return;
  if (!bootstrapFlight) {
    bootstrapFlight = (async () => {
      try {
        const ctrl = new AbortController();
        const timer = setTimeout(() => ctrl.abort(), 5000);
        let res: Response;
        try {
          res = await fetch(bootstrapUrl(), { signal: ctrl.signal });
        } finally {
          clearTimeout(timer);
        }
        if (!res.ok) return;
        const data = (await res.json().catch(() => null)) as AuthBootstrapResponse | null;
        if (data && data.auth_enabled) {
          apiToken = data.api_token;
          adminToken = data.admin_token;
        }
      } catch {
        // Unreachable server / remote 403: stay unauthenticated; calls 401.
      } finally {
        bootstrapDone = true;
      }
    })();
  }
  await bootstrapFlight;
}

/** Admin-only endpoints always use the admin token when available. */
function isAdminPath(endpoint: string): boolean {
  return (
    endpoint.startsWith('/models/swap') ||
    endpoint.startsWith('/knowledge/import') ||
    endpoint.startsWith('/admin/')
  );
}

class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public data?: unknown,
    public retryable = false
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

interface FetchOptions extends RequestInit {
  timeout?: number;
  retries?: number;
  retryableStatuses?: number[];
  onProgress?: (progress: number) => void;
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function fetchWithTimeout<T>(
  url: string,
  options: FetchOptions = {}
): Promise<T> {
  const {
    timeout = DEFAULT_TIMEOUT,
    retries = MAX_RETRIES,
    // 503 ("engine busy, back off") is deliberately NOT retried: hammering a
    // saturated engine is exactly what the server's fail-fast design avoids.
    retryableStatuses = [408, 429, 500, 502, 504],
    ...fetchOptions
  } = options;

  await ensureTokens();

  const method = (fetchOptions.method ?? 'GET').toUpperCase();
  // Retries are only safe for idempotent reads. Retrying POSTs re-executes
  // side effects (duplicate knowledge chunks, repeated generations) and
  // multiplies load during saturation — so POST/PUT/DELETE go out once.
  const effectiveRetries = method === 'GET' ? retries : 0;

  const token = isAdminPath(url) ? adminToken ?? apiToken : apiToken;
  const authHeaders: Record<string, string> = { ...fetchOptions.headers } as Record<string, string>;
  if (token && !authHeaders['Authorization']) {
    authHeaders['Authorization'] = `Bearer ${token}`;
  }

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  let lastError: Error;

  for (let attempt = 0; attempt <= effectiveRetries; attempt++) {
    try {
      const response = await fetch(`${API_BASE}${url}`, {
        ...fetchOptions,
        signal: controller.signal,
        headers: {
          'Content-Type': 'application/json',
          ...authHeaders,
        },
      });

      clearTimeout(timeoutId);

      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        const retryable = retryableStatuses.includes(response.status);
        throw new ApiError(
          (data as { error?: string }).error || `HTTP ${response.status}`,
          response.status,
          data,
          retryable
        );
      }

      return data as T;
    } catch (error) {
      clearTimeout(timeoutId);
      lastError = error as Error;

      if (error instanceof ApiError) {
        if (!error.retryable || attempt === effectiveRetries) throw error;
      } else if (error instanceof DOMException && error.name === 'AbortError') {
        throw new ApiError('Request timeout', 408, undefined, true);
      } else if (attempt === effectiveRetries) {
        throw new ApiError(
          error instanceof Error ? error.message : 'Network error',
          0,
          undefined,
          true
        );
      }

      const delay = RETRY_DELAY_BASE * Math.pow(2, attempt) + Math.random() * 1000;
      await sleep(delay);
    }
  }

  throw lastError!;
}

function createApiMethod<TArgs, TResponse>(
  endpoint: string,
  method: 'GET' | 'POST' = 'GET',
  defaultOptions: Partial<FetchOptions> = {}
) {
  return async (data?: TArgs): Promise<TResponse> => {
    const options: FetchOptions = {
      method,
      body: data ? JSON.stringify(data) : undefined,
      ...defaultOptions,
    };
    return fetchWithTimeout<TResponse>(endpoint, options);
  };
}

export const api = {
  health: () => fetchWithTimeout<HealthResponse>('/health', { timeout: 10000 }),

  device: () => fetchWithTimeout<DeviceResponse>('/device', { timeout: 10000 }),

  models: () => fetchWithTimeout<ModelInfo>('/models', { timeout: 10000 }),

  swapModel: createApiMethod<SwapModelRequest, SwapModelResponse>('/models/swap', 'POST', { timeout: 60000 }),

  generate: createApiMethod<GenerateRequest, GenerateResponse>('/generate', 'POST', { timeout: 120000 }),

  chat: createApiMethod<ChatRequest, ChatResponse>('/chat', 'POST', { timeout: 120000 }),

  batch: createApiMethod<BatchRequest, BatchResponse>('/batch', 'POST', { timeout: 180000 }),

  embeddings: createApiMethod<EmbeddingRequest, EmbeddingResponse>('/embeddings', 'POST', { timeout: 60000 }),

  knowledge: {
    search: createApiMethod<KnowledgeSearchRequest, KnowledgeSearchResponse>('/knowledge/search', 'POST', { timeout: 30000 }),
    import: createApiMethod<KnowledgeImportRequest, KnowledgeImportResponse>('/knowledge/import', 'POST', { timeout: 60000 }),
  },

  tools: {
    list: () => fetchWithTimeout<ToolsListResponse>('/tools', { timeout: 10000 }),
    execute: createApiMethod<ToolExecuteRequest, ToolExecuteResponse>('/tools/execute', 'POST', { timeout: 60000 }),
    call: createApiMethod<ToolCallRequest, ToolCallResponse>('/tools/call', 'POST', { timeout: 120000 }),
  },

  admin: {
    status: () => fetchWithTimeout<AdminStatusResponse>('/admin/status', { timeout: 10000 }),
    audit: (lines = 50) =>
      fetchWithTimeout<AuditLogResponse>(`/admin/audit?lines=${lines}`, { timeout: 10000 }),
    rotate: () =>
      fetchWithTimeout<RotateTokenResponse>('/admin/rotate', { method: 'POST', timeout: 10000 }),
    reload: () =>
      fetchWithTimeout<{ status: string }>('/admin/reload', { method: 'POST', timeout: 10000 }),
    metrics: () => fetchWithTimeout<AdminMetricsResponse>('/admin/metrics', { timeout: 10000 }),
    performance: {
      get: () =>
        fetchWithTimeout<PerformanceKnobsResponse>('/admin/performance', { timeout: 10000 }),
      update: (patch: PerformancePatchRequest) =>
        fetchWithTimeout<PerformanceKnobsResponse>('/admin/performance', {
          method: 'POST',
          timeout: 10000,
          body: JSON.stringify(patch),
        }),
    },
    sessionsPurge: (days?: number) =>
      fetchWithTimeout<SessionsPurgeResponse>('/admin/sessions/purge', {
        method: 'POST',
        timeout: 30000,
        body: JSON.stringify({ days: days ?? null }),
      }),
  },

  bench: () => fetchWithTimeout<import('../types').BenchResponse>('/bench', { timeout: 10000 }),
};

export { ApiError, fetchWithTimeout };