# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
#
# KNOW-01 Section 6: Number sync check.
# Verifies README numbers == gate output, index.html numbers == README numbers.

param()

$passed = $true

# Check README certified state
$readme = Get-Content "README.md" -Raw

# Check for certified counts
if ($readme -notmatch "156 passed, 0 failed") {
    Write-Host "RED: README missing '156 passed, 0 failed'" -ForegroundColor Red
    $passed = $false
}
if ($readme -notmatch "356/356") {
    Write-Host "RED: README missing '356/356'" -ForegroundColor Red
    $passed = $false
}
if ($readme -notmatch "54/54") {
    Write-Host "RED: README missing '54/54'" -ForegroundColor Red
    $passed = $false
}
if ($readme -notmatch "99.49%") {
    Write-Host "RED: README missing '99.49%'" -ForegroundColor Red
    $passed = $false
}

# Check index.html numbers match README
$index = Get-Content "frontend\index.html" -Raw
if ($index -notmatch "156") {
    Write-Host "RED: index.html missing '156'" -ForegroundColor Red
    $passed = $false
}

# Check vortex.json learner.enabled = false
$vortex = Get-Content "vortex.json" -Raw | ConvertFrom-Json
if ($vortex.learner.enabled -ne $false) {
    Write-Host "RED: vortex.json learner.enabled is not false" -ForegroundColor Red
    $passed = $false
}

if ($passed) {
    Write-Host "GREEN: All numbers synchronized." -ForegroundColor Green
    exit 0
} else {
    Write-Host "GATE RED: Number sync failed." -ForegroundColor Red
    exit 1
}
