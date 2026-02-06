param(
  [switch]$Release,
  [int]$Port = 33030
)

$ErrorActionPreference = "Stop"

function Resolve-Cargo {
  $cargoFromProfile = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
  if (Test-Path $cargoFromProfile) {
    return $cargoFromProfile
  }

  $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
  if ($null -ne $cargoCmd) {
    return $cargoCmd.Path
  }

  throw "cargo not found. Install Rust toolchain first."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$debugServerExePath = Join-Path $repoRoot "target\debug\server.exe"
$releaseServerExePath = Join-Path $repoRoot "target\release\server.exe"
$cargo = Resolve-Cargo

# Stop stale server processes from this repository.
$stale = Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -eq "server.exe" -and (
      ($_.ExecutablePath -and ($_.ExecutablePath -ieq $debugServerExePath -or $_.ExecutablePath -ieq $releaseServerExePath)) -or
      ($_.CommandLine -and ($_.CommandLine -like "*target\debug\server.exe*" -or $_.CommandLine -like "*target\release\server.exe*"))
    )
  }

$killedPids = @()
foreach ($p in $stale) {
  Write-Host "Stopping old process PID=$($p.ProcessId) ($($p.Name))"
  Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
  $killedPids += $p.ProcessId
}

# If target port is still occupied, fail with a clear message.
$listeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
if ($listeners) {
  $ownerPids = $listeners | Select-Object -ExpandProperty OwningProcess -Unique
  foreach ($pid in $ownerPids) {
    if ($killedPids -contains $pid) {
      continue
    }

    $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
    $procName = if ($proc) { $proc.ProcessName } else { "unknown" }
    throw "Port $Port is already in use by PID=$pid ($procName). Stop it and run the script again."
  }
}

$args = @("run", "-p", "server")
if ($Release) {
  $args += "--release"
}

Write-Host "Starting server: $cargo $($args -join ' ')"
Push-Location $repoRoot
try {
  & $cargo @args
} finally {
  Pop-Location
}
