# agenttrace Windows installer (PowerShell)
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1
# Or:    iwr -useb https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.ps1 | iex

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\agenttrace"
)

$REPO = "luoyuctl/agenttrace"
$BIN = "agenttrace.exe"

# Detect architecture
$ARCH = switch ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture) {
    "X64"   { "amd64" }
    "Arm64" { "arm64" }
    default { throw "Unsupported architecture: $_" }
}

Write-Host "🔍 Fetching latest release..." -ForegroundColor Cyan

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
