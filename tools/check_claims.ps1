# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
#
# KNOW-01 Section 3: Claim guardrail test.
# Greps outreach docs for banned phrases. Gate red if found.

$banned = @(
    "أسرع من",
    "faster than ChatGPT",
    "unlimited",
    "بدون حدود سرعة",
    "الأسرع",
    "fastest",
    "most powerful",
    "the best"
)

$files = @(
    "docs\OUTREACH_KIT.md",
    "docs\DEMO_RUNBOOK.md",
    "docs\LEARNER.md",
    "README.md",
    "frontend\index.html"
)

# For OUTREACH_KIT.md, skip the guardrail section (starts with "## 6. Positioning Guardrail Test")
$skipGuardrail = @{}
$guardrailFiles = @("docs\OUTREACH_KIT.md")

$found = $false
foreach ($file in $files) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        # Split on guardrail section if applicable
        if ($guardrailFiles -contains $file) {
            $content = $content -split '## 6\. Positioning Guardrail Test' | Select-Object -First 1
        }
        foreach ($phrase in $banned) {
            if ($content -match [regex]::Escape($phrase)) {
                Write-Host "RED: '$phrase' found in $file" -ForegroundColor Red
                $found = $true
            }
        }
    }
}

if (-not $found) {
    Write-Host "GREEN: No banned phrases found in outreach docs." -ForegroundColor Green
    exit 0
} else {
    Write-Host "GATE RED: Banned phrases found. Fix before shipping." -ForegroundColor Red
    exit 1
}
