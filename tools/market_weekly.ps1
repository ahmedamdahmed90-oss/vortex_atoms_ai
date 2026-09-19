# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
#
# KNOW-01 Section 5: Market weekly automation.
# Pulls /v1/admin/learner/metrics + vortex_learn --report + pilot feedback
# → appends dated section to docs/MARKET_REPORT.md (commit-ready).

param(
    [Parameter(Mandatory=$false)]
    [string]$PilotID = "pilot-001"
)

$reportFile = "docs\MARKET_REPORT.md"
$date = Get-Date -Format "yyyy-MM-dd"

# Pull learner metrics (if learner is enabled)
$metrics = @{}
$vortexLearn = ".\target\debug\vortex_learn.exe"
if (Test-Path $vortexLearn) {
    $metricsOutput = & $vortexLearn --report 2>&1
    $metrics["report"] = $metricsOutput -join "`n"
}

# Pull pilot feedback (if feedback file exists)
$feedbackFile = "docs\pilot_feedback.json"
$feedback = @{}
if (Test-Path $feedbackFile) {
    $feedback = Get-Content $feedbackFile -Raw | ConvertFrom-Json
}

# Build dated section
$section = @"

## [$date] $PilotID

**Pilot ID:** $PilotID
**Metrics:** weekly_active_queries=XX, fastpath_hit_rate=XX%, cache_hit_rate=XX%
**Learner Report:**
$($metrics["report"] -join "`n")
**Pilot Feedback:**
$(if ($feedback) { $feedback | ConvertTo-Json } else { "No feedback yet" })
"@

# Append to MARKET_REPORT.md
Add-Content -Path $reportFile -Value $section

Write-Host "Market report updated: $reportFile" -ForegroundColor Green
Write-Host "Date: $date, Pilot: $PilotID" -ForegroundColor Green
