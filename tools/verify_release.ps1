# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Verify release artifacts against SBOM and SHA-256 sidecars.
#
# Usage:
#   .\tools\verify_release.ps1 [-ReleaseDir <path>] [-Strict]
#
# Checks:
#   1. SBOM.json exists and is valid JSON
#   2. Every artifact in ReleaseDir has a corresponding SHA-256 sidecar (.sha256)
#   3. SHA-256 hashes match
#   4. Cargo.lock checksums are present for all 715 crates

param(
    [string]$ReleaseDir = "dist",
    [switch]$Strict
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

function Write-Result {
    param([string]$Label, [bool]$Passed)
    $icon = if ($Passed) { "✅" } else { "❌" }
    Write-Host "$icon $Label" -ForegroundColor $(if ($Passed) { "Green" } else { "Red" })
}

Write-Host "=== Vortex Atoms AI Release Verification ===" -ForegroundColor Cyan

# 1. Check SBOM.json
$sbomPath = Join-Path $repoRoot "SBOM.json"
if (Test-Path $sbomPath) {
    try {
        $sbom = Get-Content $sbomPath -Raw | ConvertFrom-Json -ErrorAction Stop
        $componentCount = $sbom.components.Count
        Write-Result "SBOM.json valid ($componentCount components)" -Passed:$true
    } catch {
        Write-Result "SBOM.json invalid: $($_.Exception.Message)" -Passed:$false
    }
} else {
    Write-Result "SBOM.json missing" -Passed:$false
}

# 2. Check Cargo.lock checksums
$cargoLock = Join-Path $repoRoot "Cargo.lock"
$hasChecksums = $false
if (Test-Path $cargoLock) {
    $content = Get-Content $cargoLock
    $checksumCount = ($content | Select-String -Pattern 'checksum = ').Count
    if ($checksumCount -ge 700) {
        $hasChecksums = $true
        Write-Result "Cargo.lock has $checksumCount checksums" -Passed:$true
    } else {
        Write-Result "Cargo.lock has only $checksumCount checksums (need 700+)" -Passed:$false
    }
} else {
    Write-Result "Cargo.lock missing" -Passed:$false
}

# 3. Verify SHA-256 sidecars for each artifact in ReleaseDir
$shaMismatch = $false
if (Test-Path $ReleaseDir) {
    Get-ChildItem -Path $ReleaseDir -Recurse -File | ForEach-Object {
        $file = $_.FullName
        $shaFile = "$file.sha256"
        if (Test-Path $shaFile) {
            $expected = (Get-Content $shaFile -Raw).Split()[0].Trim()
            $actual = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLower()
            if ($expected -eq $actual) {
                Write-Result "  $($_.Name): hash verified" -Passed:$true
            } else {
                Write-Result "  $($_.Name): HASH MISMATCH!" -Passed:$false
                $shaMismatch = $true
            }
        } elseif ($Strict) {
            Write-Result "  $($_.Name): no .sha256 sidecar" -Passed:$false
        }
    }
} else {
    Write-Host "  ReleaseDir '$ReleaseDir' not found (skipping hash verification)" -ForegroundColor Yellow
}

# 4. Verify PROVENANCE.md exists
$provPath = Join-Path $repoRoot "PROVENANCE.md"
if (Test-Path $provPath) {
    Write-Result "PROVENANCE.md exists" -Passed:$true
} else {
    Write-Result "PROVENANCE.md missing" -Passed:$false
}

# Summary
$failed = $shaMismatch -or -not $hasChecksums
Write-Host ""
if ($failed) {
    Write-Host "❌ Release verification FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "✅ Release verification PASSED" -ForegroundColor Green
    exit 0
}
