<#
  browserlane installer (Windows).

    irm https://browserlane.com/install.ps1 | iex

  Downloads the latest `bl` release, verifies its checksum, installs it, and adds
  it to your user PATH. Re-run any time to update.

  Env overrides:
    $env:BL_VERSION       install a specific tag (e.g. v0.1.0) instead of latest
    $env:BL_INSTALL_DIR   install location (default: $HOME\.browserlane\bin)
#>
$ErrorActionPreference = 'Stop'
$Repo = 'browserlane/browserlane'

function Fail($m) { Write-Error "browserlane: $m"; exit 1 }
function Say($m)  { Write-Host  "browserlane: $m" }

# Windows on ARM64 runs x64 binaries via emulation, so map both to x64.
switch ($env:PROCESSOR_ARCHITECTURE) {
  'AMD64' { $target = 'x86_64-pc-windows-msvc' }
  'ARM64' { $target = 'x86_64-pc-windows-msvc' }
  default { Fail "unsupported architecture '$($env:PROCESSOR_ARCHITECTURE)' — build from source (see README)" }
}

$version = if ($env:BL_VERSION) { $env:BL_VERSION } else {
  try { (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name }
  catch { '' }
}
if (-not $version) { Fail 'could not determine the latest release (set $env:BL_VERSION to a tag like v0.1.0)' }

$asset = "bl-$version-$target.zip"
$base  = "https://github.com/$Repo/releases/download/$version"
$tmp   = Join-Path $env:TEMP ("bl-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Say "downloading $asset"
  try { Invoke-WebRequest "$base/$asset" -OutFile (Join-Path $tmp $asset) }
  catch { Fail "download failed — is $version released for $target? ($base/$asset)" }

  try {
    Invoke-WebRequest "$base/SHA256SUMS" -OutFile (Join-Path $tmp 'SHA256SUMS')
    $line = Get-Content (Join-Path $tmp 'SHA256SUMS') | Where-Object { $_ -match ([regex]::Escape($asset) + '\s*$') } | Select-Object -First 1
    if ($line) {
      $want = ($line -split '\s+')[0].ToLower()
      $got  = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
      if ($want -ne $got) { Fail "checksum mismatch for $asset" }
      Say 'checksum ok'
    } else { Say "note: $asset not listed in SHA256SUMS — skipping checksum" }
  } catch { Say "note: SHA256SUMS not found — skipping checksum" }

  Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $tmp -Force
  $exe = Get-ChildItem -Path $tmp -Recurse -Filter 'bl.exe' | Select-Object -First 1
  if (-not $exe) { Fail 'bl.exe not found in the archive' }

  $installDir = if ($env:BL_INSTALL_DIR) { $env:BL_INSTALL_DIR } else { Join-Path $HOME '.browserlane\bin' }
  New-Item -ItemType Directory -Force -Path $installDir | Out-Null
  Copy-Item $exe.FullName (Join-Path $installDir 'bl.exe') -Force
  Say "installed bl $version -> $installDir\bl.exe"

  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($userPath -split ';') -notcontains $installDir) {
    [Environment]::SetEnvironmentVariable('Path', "$installDir;$userPath", 'User')
    Say "added $installDir to your user PATH — restart your terminal"
  }
  Say 'done — run: bl --version'
}
finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
