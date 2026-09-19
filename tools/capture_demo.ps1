# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
#
# KNOW-01 Section 1: Demo capture script.
# Runs each demo command and PAUSES for manual capture.
# No auto screen-recording — weak-machine friendly.

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet(1,2,3,4,5,6,7,8,9,"all")]
    [int]$Step = 1
)

$steps = @{
    1 = @{
        Name = "demo-01-health"
        Command = { curl http://127.0.0.1:8080/v1/health }
        Capture = "assets/demo-01-health.png"
    }
    2 = @{
        Name = "demo-02-device"
        Command = { curl http://127.0.0.1:8080/v1/device }
        Capture = "assets/demo-02-device.png"
    }
    3 = @{
        Name = "demo-03-avian-query"
        Command = { curl -X POST http://127.0.0.1:8080/v1/generate -d '{"prompt":"avian genetics summary","max_tokens":50}' }
        Capture = "assets/demo-03-avian-query.png"
    }
    4 = @{
        Name = "demo-04-avian-cache"
        Command = { curl -X POST http://127.0.0.1:8080/v1/generate -d '{"prompt":"avian genetics summary","max_tokens":50}' }
        Capture = "assets/demo-04-avian-cache.png"
    }
    5 = @{
        Name = "demo-05-learner-canary"
        Command = { & ".\target\debug\vortex_learn.exe" --phase canary --run-once }
        Capture = "assets/demo-05-learner-canary.mp4"
    }
    6 = @{
        Name = "demo-06-learner-eval"
        Command = { & ".\target\debug\vortex_learn.exe" --eval }
        Capture = "assets/demo-06-learner-eval.png"
    }
    7 = @{
        Name = "demo-07-tray-icon"
        Command = { Write-Host "Left-click tray icon now..." }
        Capture = "assets/demo-07-tray-icon.png"
    }
    8 = @{
        Name = "demo-08-tier-switch"
        Command = { curl -X POST http://127.0.0.1:8080/v1/admin/device/tier -d '{"tier":"std"}' }
        Capture = "assets/demo-08-tier-switch.png"
    }
    9 = @{
        Name = "demo-09-budget-governor"
        Command = { & ".\target\debug\vortex_learn.exe" --phase canary --run-once }
        Capture = "assets/demo-09-budget-governor.mp4"
    }
}

function Invoke-Step {
    param([int]$StepNumber)
    $step = $steps[$StepNumber]
    Write-Host "=== $($step.Name) ===" -ForegroundColor Cyan
    Write-Host "Command: $($step.Command.ToString())" -ForegroundColor Yellow
    Write-Host "Capture: $($step.Capture)" -ForegroundColor Green
    & $step.Command
    Write-Host "PAUSE: Screenshot/recording captured. Press Enter to continue..." -ForegroundColor Magenta
    Read-Host
}

if ($Step -eq "all") {
    1..9 | ForEach-Object { Invoke-Step $_ }
} else {
    Invoke-Step $Step
}
