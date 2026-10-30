param([string]$Version = "", [string]$AppArgs = "")
$ErrorActionPreference = "Continue"
$env:PATH = "C:\Program Files\LLVM\bin;" + $env:PATH
$crash = Join-Path $env:TEMP "aircast-crash.log"

Get-Process AircastQGC -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
Remove-Item $crash -ErrorAction SilentlyContinue

Set-Location C:\aircast-head\AircastQGC
[string[]]$props = if ($Version) { @("-p:Version=$Version") } else { @() }
$out = dotnet build .\AircastQGC.csproj -c Release -r win-arm64 --self-contained -v q --nologo @props 2>&1
$errs = $out | Where-Object { $_ -match ': error ' } | ForEach-Object { ($_ -replace '\s*\[C:\\aircast-head[^\]]*\]$', '') } | Sort-Object -Unique
if ($errs) { "=== BUILD ERRORS ==="; $errs; "BUILD_FAILED"; exit 1 }
"BUILD_OK"

$exe = "C:\aircast-head\AircastQGC\bin\Release\net9.0-windows10.0.22621.0\win-arm64\AircastQGC.exe"
$me = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$a = if ($AppArgs) { New-ScheduledTaskAction -Execute $exe -Argument $AppArgs } else { New-ScheduledTaskAction -Execute $exe }
$p = New-ScheduledTaskPrincipal -UserId $me -RunLevel Highest -LogonType Interactive
Register-ScheduledTask -TaskName "RunAircast" -Action $a -Principal $p -Force | Out-Null
Start-ScheduledTask -TaskName "RunAircast"
Start-Sleep 8

$proc = Get-Process AircastQGC -ErrorAction SilentlyContinue
if ($proc) { "RUNNING pid=$($proc.Id) mem=$([math]::Round($proc.WorkingSet64/1MB))MB" }
else {
  "EXITED (crashed)"
  if (Test-Path $crash) { "=== CRASH LOG ==="; Get-Content $crash | Select-Object -First 40 }
  else {
    Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddMinutes(-2)} -ErrorAction SilentlyContinue |
      Where-Object { $_.Message -match 'AircastQGC' } | Select-Object -First 1 | ForEach-Object { $_.Message.Substring(0, [Math]::Min(800, $_.Message.Length)) }
  }
  exit 2
}
