# 对比 embedding 三条链路的吞吐: 直连上游 / 过本地 server / 过远程 server。
# 用来判断代理层引入了多少损耗, 以及并发下代理是否成为瓶颈。
#   powershell -File bench-proxy.ps1 -Token <客户端token> [-Batch 256] [-Parallel 12]

param(
  [Parameter(Mandatory = $true)][string]$Token,
  [string]$Local = "http://192.168.0.6:8080",
  [string]$Upstream = "https://api.moark.com/v1/embeddings",
  [string]$UpstreamKey = "",
  [string]$ProvideId = "eyJjb2RlIjoyMCwibmFtZSI6IlF3ZW4zLUVtYmVkZGluZy0wLjZCIn0",
  [string]$Model = "Qwen3-Embedding-0.6B",
  [int]$Batch = 256,
  [int]$Parallel = 12
)

$ErrorActionPreference = "Stop"
$tmp = Join-Path $env:TEMP ("emb_bench_" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

# 每块约 500 字符, 与实际代码块的平均长度一致
$unit = "fn resolve_provide(and self) -> Result<(AiProvide, usize, usize), AiToolError> { let x = 1; }"
$sample = ($unit * 6).Substring(0, 500)
$inputs = @(1..$Batch | ForEach-Object { "$sample // v$_" })

$proxyBody = @{ id = $ProvideId; input = $inputs } | ConvertTo-Json -Depth 3 -Compress
$proxyFile = Join-Path $tmp "proxy.json"
[System.IO.File]::WriteAllText($proxyFile, $proxyBody)

$directFile = Join-Path $tmp "direct.json"
if ($UpstreamKey -ne "") {
  $directBody = @{ model = $Model; input = $inputs } | ConvertTo-Json -Depth 3 -Compress
  [System.IO.File]::WriteAllText($directFile, $directBody)
}

function Invoke-Single($label, $url, $auth, $file) {
  $raw = curl.exe -s -o NUL -w "%{http_code} %{time_total} %{size_download}" -X POST $url `
    -H "Authorization: Bearer $auth" -H "Content-Type: application/json" `
    --data-binary "@$file" --max-time 600
  if ([string]::IsNullOrWhiteSpace($raw)) {
    "{0,-16} 请求失败(无输出)" -f $label
    return
  }
  $parts = ($raw -join " ").Trim() -split "\s+"
  $code = $parts[0]
  $sec = [double]$parts[1]
  $size = [int64]$parts[2]
  $tp = if ($sec -gt 0) { [math]::Round($Batch / $sec, 1) } else { 0 }
  "{0,-16} http={1}  {2,7:N2}s  {3,7:N1} 块/秒  下行 {4:N1} MB" -f $label, $code, $sec, $tp, ($size / 1MB)
}

function Invoke-Parallel($label, $url, $auth, $file, $par) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $procs = @()
  for ($i = 0; $i -lt $par; $i++) {
    $curlArgs = @("-s", "-o", "NUL", "-w", "%{http_code}", "-X", "POST", $url,
      "-H", "Authorization: Bearer $auth", "-H", "Content-Type: application/json",
      "--data-binary", "@$file", "--max-time", "600")
    $procs += Start-Process -FilePath "curl.exe" -ArgumentList $curlArgs -NoNewWindow -PassThru `
      -RedirectStandardOutput (Join-Path $tmp "p$i.txt")
  }
  $procs | Wait-Process -Timeout 600
  $sw.Stop()
  $codes = Get-ChildItem $tmp -Filter "p*.txt" | ForEach-Object { (Get-Content $_.FullName -Raw).Trim() }
  $ok = ($codes | Where-Object { $_ -eq "200" }).Count
  Remove-Item (Join-Path $tmp "p*.txt") -Force -ErrorAction SilentlyContinue
  $blocks = $par * $Batch
  $tp = [math]::Round($blocks / $sw.Elapsed.TotalSeconds, 1)
  "{0,-16} 并发{1,-3} {2,7:N2}s  {3,7:N1} 块/秒  成功 {4}/{5}" -f $label, $par, $sw.Elapsed.TotalSeconds, $tp, $ok, $par
}

try {
  "=== 单请求 (batch=$Batch) ==="
  if ($UpstreamKey -ne "") { Invoke-Single "直连上游" $Upstream $UpstreamKey $directFile }
  Invoke-Single "本地 server" "$Local/api/aiProxyApi/v1/embeddings" $Token $proxyFile

  ""
  "=== 并发 (batch=$Batch) ==="
  foreach ($par in 4, $Parallel) {
    if ($UpstreamKey -ne "") { Invoke-Parallel "直连上游" $Upstream $UpstreamKey $directFile $par }
    Invoke-Parallel "本地 server" "$Local/api/aiProxyApi/v1/embeddings" $Token $proxyFile $par
  }
}
finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
