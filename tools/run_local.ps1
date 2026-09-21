# One-command local run for Vortex Atoms AI.
#
#   powershell -ExecutionPolicy Bypass -File tools\run_local.ps1 [-Port 8080] [-Release] [-Frontend]
#
# Starts the API server on loopback (fail-closed guard refuses non-loopback
# binds without --allow-remote). Optionally also starts the Vite dev server.
# Stop everything with: tools\stop_local.ps1
param(
    [int]$Port = 8080,
    [string]$ListenHost = "127.0.0.1",
    [switch]$Release,
    [switch]$Frontend
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$LogsDir = Join-Path $Root "logs"
New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null

$Binary = Join-Path $Root "target\debug\vortex_api.exe"
if ($Release) {
    $ReleaseBin = Join-Path $Root "target\release\vortex_api.exe"
    if (Test-Path -LiteralPath $ReleaseBin) {
        $Binary = $ReleaseBin
    } else {
        Write-Output "Release binary missing. Build it first:"
        Write-Output "  cargo build --release --features learner --bin vortex_api"
        exit 1
    }
}
if (-not (Test-Path -LiteralPath $Binary)) {
    Write-Output "Binary missing: $Binary"
    Write-Output "Build it first: cargo build --features learner --bin vortex_api"
    exit 1
}

$ApiLog = Join-Path $LogsDir "vortex_api.log"
$ApiErr = Join-Path $LogsDir "vortex_api.err.log"
Write-Output "[run] starting backend: $Binary --host $ListenHost --port $Port"
Start-Process -FilePath $Binary -ArgumentList "--host", $ListenHost, "--port", "$Port" `
    -WorkingDirectory $Root `
    -RedirectStandardOutput $ApiLog -RedirectStandardError $ApiErr | Out-Null

$HealthUrl = "http://${ListenHost}:${Port}/v1/health"
$Ready = $false
for ($i = 0; $i -lt 36; $i++) {
    Start-Sleep -Seconds 5
    try {
        $resp = Invoke-WebRequest -Uri $HealthUrl -UseBasicParsing -TimeoutSec 5
        if ($resp.StatusCode -eq 200) { $Ready = $true; break }
    } catch {
        # 503 busy during generation, connection refused while booting — keep waiting.
    }
}
if (-not $Ready) {
    Write-Output "[run] backend did not become ready in 3 min. See $ApiLog"
    exit 1
}
Write-Output "[run] backend OK: http://${ListenHost}:${Port}/ (UI)  /v1/health"

if ($Frontend) {
    $ViteLog = Join-Path $LogsDir "vite.log"
    $ViteErr = Join-Path $LogsDir "vite.err.log"
    Write-Output "[run] starting Vite dev server..."
    Start-Process -FilePath "cmd.exe" -ArgumentList "/c npm run dev" `
        -WorkingDirectory (Join-Path $Root "frontend") `
        -RedirectStandardOutput $ViteLog -RedirectStandardError $ViteErr | Out-Null
    $ViteReady = $false
    for ($i = 0; $i -lt 24; $i++) {
        Start-Sleep -Seconds 5
        try {
            # NOTE: Vite binds IPv6 localhost; use the hostname, not 127.0.0.1.
            $resp = Invoke-WebRequest -Uri "http://localhost:5173/" -UseBasicParsing -TimeoutSec 5
            if ($resp.StatusCode -eq 200) { $ViteReady = $true; break }
        } catch {
        }
    }
    if (-not $ViteReady) {
        Write-Output "[run] Vite did not become ready in 2 min. See $ViteLog"
        exit 1
    }
    Write-Output "[run] frontend OK: http://localhost:5173/ (API proxied to :$Port)"
}

Write-Output "[run] tokens: loopback-only http://${ListenHost}:${Port}/auth/bootstrap"
Write-Output "[run] logs: $LogsDir  | stop: powershell -File tools\stop_local.ps1"
