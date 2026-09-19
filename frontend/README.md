# Vortex Atoms AI — Web Frontend

Arabic-first (RTL) chat interface and management dashboard for the Vortex Atoms AI local server.

## Stack

- React 19 + TypeScript + Vite (rolldown)
- Zustand stores (`stores/`) for chat, dashboard, and settings state
- Tailwind CSS with a forced `rtl` layout (`RTLLayout`)
- WebSocket streaming chat (`services/ws.ts`) plus the REST API client (`services/api.ts`)
- Vitest (unit/integration) and Playwright (E2E, Chromium + Firefox + WebKit)

## Run

```bash
npm install
npm run dev          # http://localhost:5173
```

The app expects the API server at `http://localhost:8080` (see the root `README.md`). Both the API
URL and WebSocket URL can be changed in the dashboard settings.

## Chat generation settings

The chat panel exposes `temperature` and `max_tokens`. The token slider (and the default) is
clamped to `64..512` to match the API ceiling (`MAX_API_MAX_TOKENS = 512`).

## Test & build

```bash
npm run test           # Vitest — 312 tests
npm run test:coverage  # 99.49% stmts / 99.4% branch / 99.42% funcs / 99.88% lines
npm run test:e2e       # Playwright — 48 tests across 3 browsers
npm run lint           # oxlint (0 errors)
npm run build          # tsc -b && vite build
```

The production build (`vite build`) is embedded into `vortex_api.exe` via `rust-embed`
(see `src/embedded_frontend.rs`). In production the API and WebSocket URLs are same-origin
(`/v1`, `ws://<host>/ws`) so the UI works on any server port; in dev they default to
`http://localhost:8080` / `ws://localhost:8080/ws`.