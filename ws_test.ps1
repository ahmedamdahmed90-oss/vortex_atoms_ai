param([string]$Token = "")
# Fetch a bearer token automatically (loopback bootstrap); pass -Token explicitly for remote hosts.
if (-not $Token) {
    try {
        $boot = Invoke-RestMethod -Uri "http://localhost:8080/auth/bootstrap" -TimeoutSec 5
        $Token = $boot.api_token
        Write-Host "bootstrapped user token"
    } catch { Write-Host "no token bootstrapped ($_) — connecting unauthenticated" }
}
$ws = [System.Net.WebSockets.ClientWebSocket]::new()
$target = "ws://localhost:8080/ws"
if ($Token) { $target += "?token=$Token" }
$uri = [System.Uri]$target
$ws.ConnectAsync($uri, [Threading.CancellationToken]::None).GetAwaiter().GetResult()
Write-Host "WS CONNECTED: $($ws.State)"

$bytes = [Text.Encoding]::UTF8.GetBytes('{"prompt":"Say hi in 3 words","max_tokens":10}')
$seg = [ArraySegment[byte]]::new($bytes)
$ws.SendAsync($seg, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).GetAwaiter().GetResult()
Write-Host "SENT generate request"

$buf = [byte[]]::new(65536)
for ($i = 0; $i -lt 50; $i++) {
    $seg2 = [ArraySegment[byte]]::new($buf)
    $result = $ws.ReceiveAsync($seg2, [Threading.CancellationToken]::None).GetAwaiter().GetResult()
    if ($result.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { Write-Host "WS CLOSED"; break }
    $text = [Text.Encoding]::UTF8.GetString($buf, 0, $result.Count)
    Write-Host "RECEIVED: $text"
    if ($text -match '"done"') { Write-Host "GENERATION COMPLETE"; break }
}
try { $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "done", [Threading.CancellationToken]::None).GetAwaiter().GetResult() } catch {}
