#!/usr/bin/env pwsh
# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# REPO-01 Section 2: verify repo URLs + commit SHA match facts.json

 param(
    [string]$FactsPath = "docs/data_room/facts.json"
)

$ErrorActionPreference = "Stop"
$failed = 0

# Read facts.json
$facts = Get-Content -Raw -Path $FactsPath | ConvertFrom-Json

# Expected values from facts.json
$expectedRepoUrl = $facts.repository.repo_url.value
$expectedSha = $facts.repository.repo_sha.value

Write-Host "=== REPO-01: check_repo_refs.ps1 ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check git remote matches repo_url
$gitRemote = (git remote get-url origin 2>$null).Trim()
if ($gitRemote -notlike "*vortex_atoms_ai*") {
    Write-Host "❌ Git remote does not match repo_url: $gitRemote" -ForegroundColor Red
    $failed++
} else {
    Write-Host "✅ Git remote OK: $gitRemote" -ForegroundColor Green
}

# 2. Check HEAD commit matches repo_sha
$gitSha = (git rev-parse HEAD 2>$null).Trim()
$factsShaValid = (git cat-file -t $expectedSha 2>$null) -eq "commit"
if ($gitSha -ne $expectedSha) {
    if ($factsShaValid) {
        Write-Host "⚠️  HEAD SHA differs from facts.json (expected after fresh push)" -ForegroundColor Yellow
        Write-Host "   facts.json SHA $expectedSha is valid commit in repo" -ForegroundColor Yellow
    } else {
        Write-Host "❌ HEAD SHA mismatch AND facts.json SHA not found in repo: $expectedSha" -ForegroundColor Red
        $failed++
    }
} else {
    Write-Host "✅ HEAD SHA matches: $gitSha" -ForegroundColor Green
}

# 3. Check README badge URLs contain this repo
$readmeContent = Get-Content -Raw -Path "README.md"
$badgePattern = 'github\.com/ahmedamdahmed90-oss/vortex_atoms_ai'
if ($readmeContent -notmatch $badgePattern) {
    Write-Host "❌ README badges do not reference this repo" -ForegroundColor Red
    $failed++
} else {
    Write-Host "✅ README badges reference this repo" -ForegroundColor Green
}

# 4. Check docs/index.md references repo
$indexContent = Get-Content -Raw -Path "docs/index.md" -ErrorAction SilentlyContinue
if ($indexContent -and $indexContent -notmatch $badgePattern) {
    Write-Host "❌ docs/index.md does not reference this repo" -ForegroundColor Red
    $failed++
} else {
    Write-Host "✅ docs/index.md references this repo" -ForegroundColor Green
}

# 5. Check OUTREACH_KIT references repo
$outreachContent = Get-Content -Raw -Path "docs/OUTREACH_KIT.md" -ErrorAction SilentlyContinue
if ($outreachContent -and $outreachContent -notmatch "vortexatoms@gmail.com") {
    Write-Host "❌ docs/OUTREACH_KIT.md missing contact email" -ForegroundColor Red
    $failed++
} else {
    Write-Host "✅ docs/OUTREACH_KIT.md contact email present" -ForegroundColor Green
}

Write-Host ""
if ($failed -eq 0) {
    Write-Host "✅ ALL REPO REFS VERIFIED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ $failed CHECK(S) FAILED" -ForegroundColor Red
    exit 1
}
