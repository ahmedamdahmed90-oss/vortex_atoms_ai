export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  tokensUsed?: number;
  tokensPerSecond?: number;
  isStreaming?: boolean;
  toolCalls?: ToolCall[];
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  result?: string;
}

export interface ChatRequest {
  messages: Array<{ role: string; content: string }>;
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
}

export interface ChatResponse {
  text: string;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export interface GenerateRequest {
  prompt: string;
  max_tokens?: number;
  temperature?: number;
}

export interface GenerateResponse {
  text: string;
  total_tokens: number;
  tokens_per_second: number;
}

export interface EmbeddingRequest {
  texts: string[];
}

export interface EmbeddingResponse {
  embeddings: number[][];
  dimensions: number;
  model: string;
  usage: { prompt_tokens: number };
}

export interface KnowledgeSearchRequest {
  query: string;
  limit?: number;
}

export interface KnowledgeSearchResult {
  id: string;
  score: number;
  text: string;
}

export interface KnowledgeSearchResponse {
  results: KnowledgeSearchResult[];
}

export interface KnowledgeImportRequest {
  text?: string;
  file_path?: string;
}

export interface KnowledgeImportResponse {
  chunks_imported: number;
  status: string;
}

export interface PerfInfo {
  sku: string;
  tier: string;
  tier_model: string;
  tier_max_context: number;
  infer_threads: number;
  async_workers: number;
  prefault_enabled: boolean;
  compiled_features: string[];
  host_features: string[];
  fastpath_avg_ms: number;
}

export interface HealthResponse {
  status: string;
  version: string;
  build_ts: string;
  git_sha: string;
  architecture: string;
  device: string;
  simd: string;
  history_length: number;
  uptime_seconds: number;
  knowledge_chunks: number;
  perf: PerfInfo;
}

export interface BenchEntry {
  sku: string;
  ttft_ms: number | null;
  tok_per_s: number | null;
  peak_rss_mb: number | null;
  fastpath_ms: number;
  cache_hit_ms: number;
  compatible: boolean;
  notes: string;
}

export interface BenchResponse {
  sku_current: string;
  tier: string;
  entries: BenchEntry[];
  generated_at: string;
}

export interface DeviceResponse {
  device: string;
  simd: string;
}

export interface ModelInfo {
  architecture: string;
  max_seq_len: number;
  max_generation_tokens: number;
}

export interface SwapModelRequest {
  repo?: string;
  model?: string;
  tokenizer?: string;
  device?: string;
  cuda_device?: number;
}

export interface SwapModelResponse {
  status: string;
  architecture: string;
  device: string;
}

export interface ToolsListResponse {
  tools: Array<{
    name: string;
    description: string;
  }>;
}

export interface ToolExecuteRequest {
  name: string;
  arguments: Record<string, unknown>;
}

export interface ToolExecuteResponse {
  result: string;
}

export interface ToolCallRequest {
  prompt: string;
  max_tokens?: number;
}

export interface ToolCallResponse {
  text: string;
  tool_calls: Array<{
    name: string;
    arguments: Record<string, unknown>;
    result: string;
  }>;
}

export interface BatchRequest {
  prompts: string[];
  max_tokens?: number;
}

export interface BatchResponse {
  results: string[];
  total_tokens: number;
}

export type WsEvent =
  | { type: 'ready' }
  | { type: 'token'; token_id: number; text: string; position: number }
  | { type: 'batch'; events: Array<{ type: 'token'; token_id: number; text: string; position: number }> }
  | { type: 'done'; total_tokens: number; tokens_per_second: number; full_text: string }
  | { type: 'error'; message: string }
  | { type: 'ping' }
  | { type: 'pong'; id?: number; timestamp?: number };

export interface WsGenerateRequest {
  prompt: string;
  max_tokens?: number;
  temperature?: number;
}

export interface AuthBootstrapResponse {
  auth_enabled: boolean;
  api_token: string | null;
  admin_token: string | null;
}

export interface AdminStatusResponse {
  auth_enabled: boolean;
  allowed_origins: string[];
  knowledge_dirs: string[];
  allow_custom_models: boolean;
  max_prompt_chars: number;
  max_batch_prompts: number;
  max_import_chars: number;
  temperature_range: [number, number];
  rate_limit_per_10s: number;
  read_only: boolean;
  admin_loopback_only: boolean;
  sessions_retention_days: number;
  requests_total: number;
  denied_401: number;
  denied_403: number;
  denied_429: number;
  uptime_seconds: number;
}

export interface AuditLogResponse {
  entries: string[];
}

export interface RotateTokenResponse {
  status: string;
  admin_token: string;
}

export interface AdminMetricsResponse {
  requests_total: number;
  denied_401: number;
  denied_403: number;
  denied_429: number;
  uptime_seconds: number;
}

export interface PerformanceKnobsResponse {
  ws_coalesce_ms: number;
  tier_policy: string;
}

export interface PerformancePatchRequest {
  ws_coalesce_ms?: number;
}

export interface SessionsPurgeResponse {
  status: string;
  removed: number;
}