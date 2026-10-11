# agenttrace Windows installer (PowerShell)
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1
# Or:    iwr -useb https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.ps1 | iex

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\agenttrace"
)

# rm-550: README.md documents AGENTTRACE_VERSION as the environment pin,
# but this installer never read it -- an env-pinned install silently
# fetched latest. Precedence, identical to install.sh: an explicit
# -Version argument > AGENTTRACE_VERSION env > latest. Checking
# $PSBoundParameters (not $Version -eq "latest") keeps an explicit
# `-Version latest` an explicit latest.
if (-not $PSBoundParameters.ContainsKey('Version') -and $env:AGENTTRACE_VERSION) {
    $Version = $env:AGENTTRACE_VERSION
    Write-Host "📌 Version pinned by AGENTTRACE_VERSION=$Version (pass -Version to override)." -ForegroundColor Cyan
}

$REPO = "luoyuctl/agenttrace"
$BIN = "agenttrace.exe"

# Detect architecture
# rm-550 (upstream #314): one resolver with a machine-env fallback.
# Reading [RuntimeInformation]::ProcessArchitecture directly failed in
# two field shapes: PSReadLine 2.0 shadows RuntimeInformation in
# interactive PS 5.1 (the try/catch below), and a 32-bit shell on 64-bit
# Windows reports X86 -- which used to hit the hard `Unsupported
# architecture` throw. PROCESSOR_ARCHITEW6432 is set exactly in that
# 32-bit-on-64-bit case and carries the real machine architecture;
# PROCESSOR_ARCHITECTURE is the last resort. AMD64/ARM64 are the env
# spellings, X64/Arm64 the enum spellings.
function Resolve-AgenttraceArch {
    $reported = $null
    try {
        $reported = [string][System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
    } catch {
        $reported = $null
    }
    if ($reported -eq "X64") { return "amd64" }
    if ($reported -eq "Arm64") { return "arm64" }
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    switch ($arch) {
        "AMD64" { return "amd64" }
        "ARM64" { return "arm64" }
        default { throw "Unsupported architecture: $arch" }
    }
}

$ARCH = Resolve-AgenttraceArch

if ($Version -eq "latest") {
    Write-Host "🔍 Fetching latest release..." -ForegroundColor Cyan
} else {
    Write-Host "🔍 Fetching release $Version..." -ForegroundColor Cyan
}

if ($Version -eq "latest") {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $Version = $release.tag_name
    $assets = $release.assets | Where-Object { $_.name -eq "agenttrace-windows-${ARCH}.exe" }
} else {
    $assets = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/tags/$Version" |
        Select-Object -ExpandProperty assets |
        Where-Object { $_.name -eq "agenttrace-windows-${ARCH}.exe" }
}

if (-not $assets) {
    Write-Host "❌ No binary found for windows/${ARCH}" -ForegroundColor Red
    Write-Host "   Build from source: git clone https://github.com/$REPO.git && cd agenttrace && cargo build --release -p agenttrace"
    exit 1
}

$asset = $assets | Select-Object -First 1
$url = $asset.browser_download_url

Write-Host "⬇️  Downloading agenttrace (windows/${ARCH})..." -ForegroundColor Cyan
Write-Host "   $url"

# Create install directory
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$DEST = Join-Path $InstallDir $BIN

# Download
$tmp = [System.IO.Path]::GetTempFileName()
try {
    Invoke-WebRequest -Uri $url -OutFile $tmp
    $size = (Get-Item $tmp).Length
    Write-Host "   Binary size: $size bytes"

    # rm-005: integrity gate. install.sh and npm/scripts/install.js both
    # verify the .sha256 sidecar before installing; this channel had no
    # check at all. Missing sidecar, malformed digest, or mismatch means
    # the install is refused before anything reaches $DEST. The sidecar is
    # published by the release workflow next to every asset (including the
    # windows ones), so its absence from a fetched URL is itself a red
    # flag, not a condition to tolerate.
    $sidecarUrl = "$url.sha256"
    $sidecarTmp = [System.IO.Path]::GetTempFileName()
    Invoke-WebRequest -Uri $sidecarUrl -OutFile $sidecarTmp
    $expected = ((Get-Content $sidecarTmp -Raw).Trim() -split '\s+')[0]
    if (-not $expected -or $expected -notmatch '^[0-9a-fA-F]{64}$') {
        throw "Checksum sidecar ($sidecarUrl) did not contain a SHA-256 digest"
    }
    $actual = (Get-FileHash -Path $tmp -Algorithm SHA256).Hash
    if ($actual -ne $expected) {
        throw "SHA-256 mismatch: expected $expected, got $actual"
    }
    Write-Host "   SHA-256 verified ($($actual.ToLower()))"

    Move-Item -Force $tmp $DEST

    # Runtime verification: a downloaded artifact must prove it runs on THIS
    # host before the install is declared successful. Release binaries built
    # against a newer toolchain (for example a newer libc) fail here, loudly,
    # instead of installing silently broken (Maestro finding 47fa1154 pinned
    # this exact failure shape on the Linux installer).
    $probe = & $DEST --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Installed agenttrace binary does not run on this host:" -ForegroundColor Red
        $probe | Select-Object -Last 3 | ForEach-Object { Write-Host "   $_" }
        Remove-Item -Force $DEST
        Write-Host "   Usually the release binary was built against a newer system"
        Write-Host "   library than this host provides, or the architecture does not match."
        Write-Host "   Build from source: git clone https://github.com/$REPO.git && cd agenttrace && cargo build --release -p agenttrace"
        exit 1
    }
    Write-Host "   $($probe | Select-Object -First 1)"
    Write-Host "✅ Installed to $DEST" -ForegroundColor Green
} catch {
    Write-Host "❌ Download failed: $_" -ForegroundColor Red
    exit 1
} finally {
    if (Test-Path $tmp) { Remove-Item $tmp -Force }
    if (Test-Path $sidecarTmp) { Remove-Item $sidecarTmp -Force }
}

# PATH check
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    Write-Host ""
    Write-Host "⚠️  $InstallDir is not in your PATH." -ForegroundColor Yellow
    Write-Host "   Run this to add it:"
    Write-Host '     [Environment]::SetEnvironmentVariable("Path", "$env:LOCALAPPDATA\agenttrace;$env:PATH", "User")'
    Write-Host "   Then restart your terminal."
    Write-Host ""
}

Write-Host ""
Write-Host "🎉 agenttrace installed! Try:" -ForegroundColor Cyan
Write-Host "   agenttrace --latest"
Write-Host "   agenttrace            # launch TUI"
