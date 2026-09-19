# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Check if source reports are fresh (≤30 days).
# Does NOT run cargo (too slow for CI); gate output
# verified by separate CI step.
#
# Usage:
#   .\tools\check_facts_fresh.ps1 [-Strict]

param(
    [switch]$Strict
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$failures = @()
$today = Get-Date

function Check-Age {
    param([string]$Path, [string]$Name)
    $file = Join-Path $repoRoot $Path
    if (Test-Path $file) {
        $age = ($today - (Get-Item $file).LastWriteTime).Days
        if ($age -gt 30) {
            Write-Host "❌ $Name is $age days old (max 30)" -ForegroundColor Red
            $script:failures += "$Name expired: $age days"
        } else {
            Write-Host "✅ $Name is $age days old" -ForegroundColor Green
        }
    } else {
        Write-Host "❌ $Name missing" -ForegroundColor Red
        $script:failures += "$Name missing"
    }
}

Write-Host "=== Facts Freshness Check ===" -ForegroundColor Cyan

Check-Age -Path "docs/PERF_REPORT.md" -Name "PERF_REPORT.md"
Check-Age -Path "docs/LEARNER_REPORT.md" -Name "LEARNER_REPORT.md"
Check-Age -Path "docs/SECURITY.md" -Name "SECURITY.md"
Check-Age -Path "docs/SEC01_LANDING.md" -Name "SEC01_LANDING.md"
Check-Age -Path "README.md" -Name "README.md"
Check-Age -Path "CHANGELOG.md" -Name "CHANGELOG.md"
Check-Age -Path "Cargo.lock" -Name "Cargo.lock"
Check-Age -Path "docs/data_room/facts.json" -Name "facts.json"
Check-Age -Path "tools/check_facts_fresh.ps1" -Name "check_facts_fresh.ps1"
Check-Age -Path "tools/check_no_manual_numbers.ps1" -Name "check_no_manual_numbers.ps1"

# Summary
Write-Host ""
if ($failures.Count -gt 0) {
    Write-Host "❌ Facts freshness check FAILED ($($failures.Count) issues)" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
} else {
    Write-Host "✅ Facts freshness check PASSED" -ForegroundColor Green
    exit 0
}
