# Cross-platform install script for pyx (Windows PowerShell)
# Usage:
#   irm https://raw.githubusercontent.com/dkmnx/pyx/main/scripts/install.ps1 | iex
#   or: .\install.ps1 [-Version VERSION] [-FromSource]

param(
    [string]$Version,
    [switch]$FromSource
)

$ErrorActionPreference = 'Stop'

# Configuration
$Repo = "dkmnx/pyx"
$InstallDir = "$env:LOCALAPPDATA\pyx\bin"

# Colors
function Write-Info { Write-Host "[INFO] $args" -ForegroundColor Green }
function Write-Warn { Write-Host "[WARN] $args" -ForegroundColor Yellow }
function Write-Err { Write-Host "[ERROR] $args" -ForegroundColor Red }

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "           pyx Installer (Windows)" -ForegroundColor Cyan
Write-Host "=========================================="
Write-Host ""

# Detect architecture
function Get-Target {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64"   { return "x86_64-pc-windows-msvc" }
        default   { Write-Err "Unsupported architecture: $arch"; exit 1 }
    }
}

function Get-Extension {
    param([string]$Target)
    if ($Target -like "*windows*") { return "zip" }
    return "tar.gz"
}

# Get latest version from GitHub
function Get-LatestVersion {
    $response = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    if (-not $response) {
        Write-Err "Failed to fetch latest version"
        exit 1
    }
    # Remove 'v' prefix if present
    $version = $response.tag_name -replace '^v', ''
    return $version
}

# Check prerequisites
function Test-Prereqs {
    param([bool]$FromSrc)

    if ($FromSrc) {
        try {
            $null = Get-Command cargo -ErrorAction Stop
        }
        catch {
            Write-Err "Missing Rust (cargo). Install from: https://rustup.rs"
            exit 1
        }
    }

    try {
        $null = Get-Command curl -ErrorAction Stop
    }
    catch {
        Write-Err "Missing curl"
        exit 1
    }
}

# Build from source
function Build-FromSource {
    Write-Info "Building pyx from source..."

    if (-not (Test-Path ".git")) {
        Write-Err "Not a git repository. Cannot build from source."
        exit 1
    }

    cargo build --release

    $binary = ".\target\release\pyx.exe"
    if (-not (Test-Path $binary)) {
        Write-Err "Build failed: binary not found"
        exit 1
    }

    return $binary
}

# Download binary with checksum verification
function Get-Binary {
    param([string]$Version, [string]$Target, [string]$Ext)

    Write-Info "Downloading pyx v$Version for $Target..."

    $BaseUrl = "https://github.com/$Repo/releases/download/v$Version"
    $ChecksumUrl = "$BaseUrl/SHA256SUMS.txt"

    $TmpDir = [System.IO.Path]::GetTempPath()
    $TmpSubDir = Join-Path $TmpDir ([System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $TmpSubDir -Force | Out-Null

    try {
        # Download checksums
        Write-Info "Fetching checksums..."
        $ChecksumFile = Join-Path $TmpSubDir "SHA256SUMS.txt"
        try {
            Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumFile -UseBasicParsing
        }
        catch {
            Write-Err "Failed to download checksums from $ChecksumUrl"
            exit 1
        }

        # Download binary
        $Filename = "pyx-$Version-$Target.$Ext"
        $Archive = Join-Path $TmpSubDir $Filename
        Write-Info "Downloading binary..."
        try {
            Invoke-WebRequest -Uri "$BaseUrl/$Filename" -OutFile $Archive -UseBasicParsing
        }
        catch {
            Write-Err "Failed to download binary from $BaseUrl/$Filename"
            exit 1
        }

        # Verify checksum
        Write-Info "Verifying checksum..."
        $Checksums = Get-Content $ChecksumFile

        if ($Ext -eq "zip") {
            Expand-Archive -Path $Archive -DestinationPath $TmpSubDir -Force
            $BinaryPath = Join-Path $TmpSubDir "pyx.exe"
        } else {
            # tar.gz on Windows requires 7zip or wsl
            Write-Err "tar.gz extraction not supported on Windows. Use -FromSource instead."
            exit 1
        }

        if (-not (Test-Path $BinaryPath)) {
            Write-Err "Extracted binary not found"
            exit 1
        }

        # Compute checksum
        $ComputedHash = (Get-FileHash -Path $BinaryPath -Algorithm SHA256).Hash.ToLower()
        $Found = $false

        foreach ($Line in $Checksums) {
            if ($Line -match "([a-f0-9]+)\s+\*?(.+)") {
                $Checksum = $Matches[1].ToLower()
                $File = $Matches[2] -replace '^.+[/\\]', ''

                if ($File -eq $Filename) {
                    $Found = $true
                    if ($Checksum -eq $ComputedHash) {
                        Write-Info "Checksum verified!"
                    } else {
                        Write-Err "Checksum mismatch!"
                        Write-Err "Expected: $Checksum"
                        Write-Err "Got:      $ComputedHash"
                        exit 1
                    }
                    break
                }
            }
        }

        if (-not $Found) {
            Write-Err "Binary not found in checksums file"
            exit 1
        }

        # Copy binary to install location before returning
        # This avoids the temp dir being cleaned up before use
        $InstallDir = $InstallDir
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        Copy-Item $BinaryPath (Join-Path $InstallDir "pyx.exe") -Force
        return (Join-Path $InstallDir "pyx.exe")
    }
    finally {
        # Cleanup handled by OS temp cleanup
    }
}

# Install binary is now handled in Get-Binary

# Add to PATH
function Add-ToPath {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($UserPath -notlike "*$InstallDir*") {
        $NewPath = "$InstallDir;$UserPath"
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        $env:Path = "$InstallDir;$env:Path"
        Write-Info "Added $InstallDir to PATH"
        Write-Info "Restart your terminal for changes to take effect"
    } else {
        Write-Info "$InstallDir already in PATH"
    }
}

# Main
function Main {
    $Target = Get-Target
    $Ext = Get-Extension -Target $Target
    Write-Info "Target: $Target"

    Test-Prereqs -FromSrc $FromSource

    $Src = $null

    if ($FromSource) {
        $Src = Build-FromSource
    } else {
        if (-not $Version) {
            $Version = Get-LatestVersion
        }
        Write-Info "Installing version: $Version"
        $Src = Get-Binary -Version $Version -Target $Target -Ext $Ext
    }

    if ($Src) {
        Add-ToPath

        Write-Host ""
        Write-Info "Installation complete!"
        Write-Info "Run 'pyx --help' to get started."
    }
}

Main
