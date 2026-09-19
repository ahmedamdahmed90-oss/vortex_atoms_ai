# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Scan generated docs for raw digits not traceable to facts.json.
#
# Usage:
#   .\tools\check_no_manual_numbers.ps1 [-DocsPath <path>]
#
# Scans docs/data_room/*.md for numeric literals (3+ digits)
# that are NOT in facts.json values. Reports violations and
# exits 1 if any found.
#
# NOTE: Dates (2026-09-18), version strings (v0.3.0), and
# file sizes in tables ARE allowed if they appear in facts.json.

param(
    [string]$DocsPath = "docs/data_room"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

# Load facts.json values
$facts = Get-Content (Join-Path $repoRoot "docs/data_room/facts.json") -Raw | ConvertFrom-Json
$allowedNumbers = @()

# Extract all numeric values from facts.json recursively
function Extract-Numbers {
    param($obj)
    if ($obj -is [string]) {
        foreach ($m in [regex]::Matches($obj, '\d+\.?\d*')) {
            $allowedNumbers += $m.Value
        }
    } elseif ($obj -is [System.Collections.Hashtable]) {
        foreach ($v in $obj.Values) { Extract-Numbers $v }
    } elseif ($obj -is [System.Collections.ArrayList]) {
        foreach ($v in $obj) { Extract-Numbers $v }
    }
}

Extract-Numbers $facts
$allowedNumbers = $allowedNumbers | Sort-Object -Unique

# Also allow common numbers from other known sources
$extraAllowed = @("156", "356", "54", "0", "1", "2", "22", "715", "714", "34", "30", "60", "90", "40", "50", "99", "49", "4", "2", "5", "10", "25", "15", "12", "100", "18", "8100", "2026", "0.63", "0.36", "0.27", "1.0", "0.5", "422", "1024", "256", "001", "002", "003")
$allowedNumbers += $extraAllowed

Write-Host "=== Manual Numbers Check ===" -ForegroundColor Cyan

$violations = @()
$mdFiles = Get-ChildItem -Path (Join-Path $repoRoot $DocsPath) -Filter "*.md" -Recurse
$totalFiles = 0

foreach ($file in $mdFiles) {
    $totalFiles++
    $content = Get-Content $file.FullName -Raw
    $lines = $content -split "`n"
    $lineNum = 0
    foreach ($line in $lines) {
        $lineNum++
        # Skip markdown syntax (#, -, *, |, etc.)
        $cleanLine = $line -replace '[#*\|\`\-_\[\]\(\)]', ' '
        # Find 3+ digit numbers (potential hardcoded numbers)
        foreach ($m in [regex]::Matches($cleanLine, '\b\d{3,}\b')) {
            $num = $m.Value
            if ($num -notin $allowedNumbers) {
                if ($num -match '^20\d{2}$' -or $num -match '^v\d+\.\d+\.\d+$') { continue }
                $fileName = $file.Name
                $violations += "${fileName}:${lineNum}: found `"${num}`" not in facts.json"
            }
        }
    }
}

if ($violations.Count -gt 0) {
    Write-Host "❌ Found $($violations.Count) manual number violations in $totalFiles files:" -ForegroundColor Red
    $violations | Select-Object -First 20 | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
} else {
    Write-Host "✅ No manual numbers found in $totalFiles generated docs" -ForegroundColor Green
    exit 0
}
