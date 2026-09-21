# Stop a local stack started with tools\run_local.ps1.
$ErrorActionPreference = "SilentlyContinue"
Stop-Process -Name vortex_api -Force
# Vite runs as node; only stop node processes started under frontend/.
Get-CimInstance Win32_Process -Filter "Name='node.exe'" | ForEach-Object {
    $cmd = $_.CommandLine
    if ($cmd -like "*frontend*") {
        Stop-Process -Id $_.ProcessId -Force
    }
}
Write-Output "[stop] done (vortex_api + frontend node processes)."
