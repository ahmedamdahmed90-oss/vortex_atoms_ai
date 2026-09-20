# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Generate all 10 data room documents from facts.json.
#
# Usage:
#   .\tools\build_data_room.ps1 [-Tag <tag>] [-OutDir <dir>]
#
# Reads docs/data_room/facts.json → renders all 10 docs + index
# → writes docs/data_room/out/<tag>/

param(
    [string]$Tag = "v0.3.0-dataroom",
    [string]$OutDir = "docs/data_room/out"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

Write-Host "=== Data Room Generator ===" -ForegroundColor Cyan
Write-Host "Tag: $Tag" -ForegroundColor Cyan
Write-Host "OutDir: $OutDir\$Tag" -ForegroundColor Cyan

# Run the Python generator
$pyPath = Join-Path $repoRoot "docs/data_room/_render.py"
$outPath = Join-Path -Path $repoRoot -ChildPath $OutDir
$outPath = Join-Path -Path $outPath -ChildPath $Tag

& "C:\Users\Ahmed\AppData\Local\Programs\Python\Python311\python.exe" $pyPath $outPath $Tag

Write-Host ""
Write-Host "✅ Data room generated at $outPath" -ForegroundColor Green
