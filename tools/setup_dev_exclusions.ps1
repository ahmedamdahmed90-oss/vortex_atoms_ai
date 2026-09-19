# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Configure Windows Defender exclusions for the Vortex Atoms AI runtime.
#
# Idempotent: safe to re-run. Requires Administrator.
# Adds path exclusion for %LOCALAPPDATA%\vortex_atoms_ai and process
# exclusion for vortex_api.exe (and the SKU variants).
param(
    [switch]$VerifyOnly
)

$ErrorActionPreference = "Stop"

function Test-Admin {
    $principal = New-Object Security.Principal.WindowsPrincipal(
        [Security.Principal.WindowsIdentity]::GetCurrent())
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Error "This script must run as Administrator. Re-launch PowerShell elevated."
    exit 1
}

$cacheRoot = Join-Path $env:LOCALAPPDATA "vortex_atoms_ai"
$exeNames = @("vortex_api.exe", "vortex_api-sse41.exe", "vortex_api-avx1.exe")

Write-Host "[defender] verifying/excluding path: $cacheRoot"
$existing = Get-MpPreference
if ($existing.ExclusionPath -contains $cacheRoot) {
    Write-Host "[defender] path exclusion already present"
} else {
    if (-not $VerifyOnly) {
        Add-MpPreference -ExclusionPath $cacheRoot -ErrorAction Stop
        Write-Host "[defender] added path exclusion"
    } else {
        Write-Host "[defender] would add path exclusion (verify-only)"
    }
}

foreach ($exe in $exeNames) {
    if ($existing.ExclusionProcess -contains $exe) {
        Write-Host "[defender] process exclusion already present: $exe"
    } else {
        if (-not $VerifyOnly) {
            Add-MpPreference -ExclusionProcess $exe -ErrorAction Stop
            Write-Host "[defender] added process exclusion: $exe"
        } else {
            Write-Host "[defender] would add process exclusion: $exe (verify-only)"
        }
    }
}

# Verification
$verify = Get-MpPreference
$ok = $true
if ($verify.ExclusionPath -contains $cacheRoot) {
    Write-Host "[verify] path exclusion confirmed: $cacheRoot"
} else {
    Write-Warning "[verify] path exclusion MISSING: $cacheRoot"
    $ok = $false
}
foreach ($exe in $exeNames) {
    if ($verify.ExclusionProcess -contains $exe) {
        Write-Host "[verify] process exclusion confirmed: $exe"
    } else {
        Write-Warning "[verify] process exclusion MISSING: $exe"
        $ok = $false
    }
}

if ($ok) {
    Write-Host "[defender] all exclusions active"
    exit 0
} else {
    Write-Error "[defender] one or more exclusions missing"
    exit 1
}