# agenttrace Windows installer (PowerShell)
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1
# Or:    iwr -useb https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.ps1 | iex
#
# Release pinning (rm-2014): an `iex` pipe cannot pass parameters, so
# environment variables are the ONLY way to pin from the one-liner —
#   AGENTTRACE_VERSION=v0.10.1  install that release tag instead of
#                               latest. A tag that does not exist
#                               fails closed; the installer never
#                               silently falls back to latest.
#   AGENTTRACE_INSTALL_DIR=...  override the install directory.
#   AGENTTRACE_NO_MODIFY_PATH=1 this installer never edits PATH; the
#                               flag only silences the PATH hint.
# An explicit -Version/-InstallDir parameter (a -File invocation) wins
# over the environment, matching install.sh's contract.

param(
    [string]$Version = "",
    [string]$InstallDir = ""
)

# rm-2014: explicit parameters (only usable with -File), then the
# environment (the iex one-liner's only channel), then defaults.
if (-not $Version) { $Version = "${env:AGENTTRACE_VERSION}" }
if (-not $Version) { $Version = "latest" }
if (-not $InstallDir) { $InstallDir = "${env:AGENTTRACE_INSTALL_DIR}" }
if (-not $InstallDir) { $InstallDir = "$env:LOCALAPPDATA\agenttrace" }

$REPO = "luoyuctl/agenttrace"
$BIN = "agenttrace.exe"

# Detect architecture. RuntimeInformation is the primary signal, but in
# interactive Windows PowerShell 5.1 sessions PSReadLine 2.0.0 can shadow
# the type load (upstream #314: iex installs died there), so fall back to
# the PROCESSOR_ARCHITEW6432 / PROCESSOR_ARCHITECTURE environment strings —
# W6432 first, because it carries the REAL machine arch when a 32-bit
# PowerShell runs on 64-bit Windows. Unknown arch fails closed.
$ARCH = $null
try {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture) {
        "X64"   { $ARCH = "amd64" }
        "Arm64" { $ARCH = "arm64" }
        default { }
    }
} catch { }
if (-not $ARCH) {
    $envArch = "${env:PROCESSOR_ARCHITEW6432}"
    if (-not $envArch) { $envArch = "${env:PROCESSOR_ARCHITECTURE}" }
    switch ($envArch) {
        "AMD64" { $ARCH = "amd64" }
        "ARM64" { $ARCH = "arm64" }
        default { }
    }
}
if (-not $ARCH) {
    throw "Unsupported architecture: ${env:PROCESSOR_ARCHITEW6432}/${env:PROCESSOR_ARCHITECTURE} (RuntimeInformation unavailable)"
}

$Pinned = $Version -ne "latest"
Write-Host "🔍 Fetching release ($Version)..." -ForegroundColor Cyan

try {
    if ($Pinned) {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/tags/$Version"
    } else {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        $Version = $release.tag_name
    }
    $assets = $release.assets | Where-Object { $_.name -eq "agenttrace-windows-${ARCH}.exe" }
} catch {
    if ($Pinned) {
        Write-Host "❌ Release '$Version' not found (AGENTTRACE_VERSION/-Version); refusing to fall back to latest." -ForegroundColor Red
        Write-Host "   Unset AGENTTRACE_VERSION to track the newest release."
    } else {
        Write-Host "❌ Could not resolve the latest release from GitHub: $_" -ForegroundColor Red
    }
    exit 1
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

# PATH check — rm-2014: this installer never edits PATH on its own; it
# prints how to add the install directory. AGENTTRACE_NO_MODIFY_PATH=1
# (or -Version-style explicit opt-out parity with install.sh) silences
# the hint for scripted installs.
if (-not ${env:AGENTTRACE_NO_MODIFY_PATH}) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Host ""
        Write-Host "⚠️  $InstallDir is not in your PATH." -ForegroundColor Yellow
        Write-Host "   Run this to add it:"
        Write-Host '     [Environment]::SetEnvironmentVariable("Path", "$env:LOCALAPPDATA\agenttrace;$env:PATH", "User")'
        Write-Host "   Then restart your terminal."
        Write-Host ""
    }
}

Write-Host ""
Write-Host "🎉 agenttrace installed! Try:" -ForegroundColor Cyan
Write-Host "   agenttrace --latest"
Write-Host "   agenttrace            # launch TUI"
