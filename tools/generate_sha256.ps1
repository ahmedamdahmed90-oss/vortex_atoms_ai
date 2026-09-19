# Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# Generate SHA-256 sidecars for all release artifacts.
#
# Usage:
#   .\tools\generate_sha256.ps1 [-ReleaseDir <path>]
#
# For every .exe, .dll, .zip in ReleaseDir, creates a .sha256 sidecar
# containing the SHA-256 hash of the artifact.

param(
    [string]$ReleaseDir = "dist"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not (Test-Path $ReleaseDir)) {
    Write-Host "ReleaseDir '$ReleaseDir' not found." -ForegroundColor Red
    exit 1
}

$count = 0
Get-ChildItem -Path $ReleaseDir -Recurse -File | ForEach-Object {
    # Skip .sha256 files themselves
    if ($_.Extension -eq '.sha256') { return }
    # Skip sidecars (already generated)
    if ($_.Name.EndsWith('.sha256')) { return }

    $file = $_.FullName
    $hash = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLower()
    $shaFile = "$file.sha256"
    Set-Content -Path $shaFile -Value "$hash  $(Split-Path $file -Leaf)" -Encoding ASCII
    Write-Host "  Generated: $($_.Name).sha256" -ForegroundColor Green
    $count++
}

Write-Host ""
Write-Host "✅ Generated $count SHA-256 sidecar files in $ReleaseDir" -ForegroundColor Cyan
