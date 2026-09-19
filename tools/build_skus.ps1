# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Build opt-in CPU SKU executables without touching the default SSE2 baseline.
#
# Produces:
#   dist/skus/vortex_api-sse41.exe (SSE4.1/SSSE3)
#   dist/skus/vortex_api-avx1.exe  (AVX/SSE4.2/SSE4.1/SSSE3)
#
# The default `cargo build --release` command and cache stay untouched because
# each SKU compiles under its own `--target-dir` and RUSTFLAGS is restored.
param(
    [string]$Features = "tray",
    [string]$OutDir = "",
    [switch]$CleanTarget
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path $repoRoot "dist\skus"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$cargo = Get-Command cargo -ErrorAction Stop
$oldRustflags = $env:RUSTFLAGS
$skus = @(
    @{ Name = "sse41"; Flags = "-C target-feature=+sse4.1,+ssse3"; Exe = "vortex_api-sse41.exe" },
    @{ Name = "avx1"; Flags = "-C target-feature=+avx,+sse4.2,+sse4.1,+ssse3"; Exe = "vortex_api-avx1.exe" }
)

try {
    foreach ($sku in $skus) {
        $targetDir = Join-Path $repoRoot ("target\skus\" + $sku.Name)
        $outExe = Join-Path $OutDir $sku.Exe
        Write-Host "[sku] building $($sku.Name) with RUSTFLAGS='$($sku.Flags)'"
        $env:RUSTFLAGS = $sku.Flags
        & $cargo.Source build --locked --release --bin vortex_api --features $Features --target-dir $targetDir
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed for SKU $($sku.Name) (exit $LASTEXITCODE)" }

        $srcExe = Join-Path $targetDir "release\vortex_api.exe"
        if (-not (Test-Path $srcExe)) { throw "expected SKU binary missing: $srcExe" }
        Copy-Item -Force $srcExe $outExe
        $size = (Get-Item $outExe).Length
        Write-Host "[sku] wrote $outExe ($size bytes)"

        $probe = & $outExe --simd-probe
        $code = $LASTEXITCODE
        Write-Host "[sku] probe: $probe (exit $code)"
        if ($code -ne 0) {
            Write-Warning "SKU $($sku.Name) is incompatible with this host CPU; artifact kept for compatible machines."
        }

        if ($CleanTarget -and (Test-Path $targetDir)) {
            Remove-Item -Recurse -Force $targetDir
            Write-Host "[sku] cleaned $targetDir"
        }
    }
    Write-Host "[sku] default SSE2 baseline untouched (separate target dirs; RUSTFLAGS restored)"
} finally {
    if ($null -eq $oldRustflags) {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTFLAGS = $oldRustflags
    }
}
