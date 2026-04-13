# Cross-platform install script for pyx
# Usage:
#   irm https://raw.githubusercontent.com/dkmnx/pyx/main/scripts/install.ps1 | iex
#   or: .\install.ps1 [-Version VERSION] [-FromSource]

param(
    [string]$Version,
    [switch]$FromSource
)

$ErrorActionPreference = 'Stop'

$Repo = "dkmnx/pyx"
$InstallDir = Join-Path $env:LOCALAPPDATA "pyx\bin"

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Err {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Get-OperatingSystem {
    $OSString = if ($PSVersionTable.OS) {
        $PSVersionTable.OS
    } elseif ($env:OS) {
        $env:OS
    } else {
        ''
    }

    if ($OSString -match 'Linux') {
        return 'linux'
    } elseif ($OSString -match 'Darwin') {
        return 'macos'
    } elseif ($OSString -match 'MINGW|NUWC|CYGWIN|Git-Bash') {
        return 'windows'
    } elseif ($env:OS -eq 'Windows_NT' -or $PSVersionTable.Platform -eq 'Win32NT') {
        return 'windows'
    } else {
        return 'unknown'
    }
}

function Get-TargetTriple {
    param([string]$OS)

    $ProcessorArchitecture = if ($OS -eq 'macos') {
        uname -m
    } else {
        $env:PROCESSOR_ARCHITECTURE
    }

    switch ($OS) {
        'linux' {
            switch ($ProcessorArchitecture) {
                'x86_64'  { return 'x86_64-unknown-linux-gnu' }
                'aarch64' { return 'aarch64-unknown-linux-gnu' }
                'arm64'   { return 'aarch64-unknown-linux-gnu' }
                default {
                    Write-Err "Unsupported architecture: $ProcessorArchitecture"
                    exit 1
                }
            }
        }
        'macos' {
            switch ($ProcessorArchitecture) {
                'x86_64'  { return 'x86_64-apple-darwin' }
                'arm64'   { return 'aarch64-apple-darwin' }
                'aarch64' { return 'aarch64-apple-darwin' }
                default {
                    Write-Err "Unsupported architecture: $ProcessorArchitecture"
                    exit 1
                }
            }
        }
        'windows' {
            switch ($ProcessorArchitecture) {
                'AMD64' { return 'x86_64-pc-windows-msvc' }
                default {
                    Write-Err "Unsupported architecture: $ProcessorArchitecture"
                    exit 1
                }
            }
        }
        default {
            Write-Err "Unsupported OS: $OS"
            exit 1
        }
    }
}

function Get-ArchiveExtension {
    param([string]$TargetTriple)

    if ($TargetTriple -like '*windows*') {
        return 'zip'
    }
    return 'tar.gz'
}

function Get-LatestReleaseVersion {
    $Version = $null

    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $Version = gh release list --repo $Repo --limit 1 |
            ForEach-Object { ($_ -split '\t')[1] -replace '^v', '' }
    }

    if (-not $Version) {
        $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        if ($Response) {
            $Version = $Response.tag_name -replace '^v', ''
        }
    }

    if (-not $Version) {
        Write-Err "Failed to fetch latest version"
        exit 1
    }

    return $Version
}

function Test-Prerequisites {
    param([bool]$FromSource)

    if ($FromSource) {
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            Write-Err "Missing Rust (cargo). Install from: https://rustup.rs"
            exit 1
        }
    }

    if (-not (Get-Command curl -ErrorAction SilentlyContinue)) {
        Write-Err "Missing curl"
        exit 1
    }
}

function Build-FromSource {
    Write-Info "Building pyx from source..."

    if (-not (Test-Path '.git')) {
        Write-Err "Not a git repository. Cannot build from source."
        exit 1
    }

    $Env:CARGO_TERM_COLOR = 'never'
    cargo build --release

    $BinaryPath = Join-Path (Join-Path (Get-Location) 'target') 'release\pyx.exe'
    if (-not (Test-Path $BinaryPath)) {
        Write-Err "Build failed: binary not found"
        exit 1
    }

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    Copy-Item -Path $BinaryPath -Destination (Join-Path $InstallDir 'pyx.exe') -Force

    return Join-Path $InstallDir 'pyx.exe'
}

function Invoke-BinaryDownload {
    param(
        [string]$Version,
        [string]$TargetTriple,
        [string]$Extension
    )

    Write-Info "Downloading pyx v$Version for $TargetTriple..."

    $BaseUrl = "https://github.com/$Repo/releases/download/v$Version"
    $ChecksumUrl = Join-Path $BaseUrl 'SHA256SUMS.txt'

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $TempDir -Force

    try {
        Write-Info "Fetching checksums..."
        $ChecksumFile = Join-Path $TempDir 'SHA256SUMS.txt'

        if (Get-Command gh -ErrorAction SilentlyContinue) {
            gh release download "v$Version" --repo $Repo --pattern 'SHA256SUMS.txt' --dir $TempDir 2>$null | Out-Null
            if (-not (Test-Path $ChecksumFile)) {
                Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumFile -UseBasicParsing
            }
        } else {
            Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumFile -UseBasicParsing
        }

        if (-not (Test-Path $ChecksumFile) -or (Get-Item $ChecksumFile).Length -eq 0) {
            Write-Err "Failed to download checksums"
            exit 1
        }

        $FileName = "pyx-$Version-$TargetTriple.$Extension"
        $ArchivePath = Join-Path $TempDir $FileName

        Write-Info "Downloading binary..."
        if (Get-Command gh -ErrorAction SilentlyContinue) {
            gh release download "v$Version" --repo $Repo --pattern $FileName --dir $TempDir 2>$null | Out-Null
            if (-not (Test-Path $ArchivePath)) {
                Invoke-WebRequest -Uri (Join-Path $BaseUrl $FileName) -OutFile $ArchivePath -UseBasicParsing
            }
        } else {
            Invoke-WebRequest -Uri (Join-Path $BaseUrl $FileName) -OutFile $ArchivePath -UseBasicParsing
        }

        if (-not (Test-Path $ArchivePath)) {
            Write-Err "Failed to download binary"
            exit 1
        }

        Write-Info "Verifying checksum..."
        $ComputedHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()
        $Found = $false

        Get-Content $ChecksumFile | ForEach-Object {
            if ($_ -match '([a-f0-9]+)\s+\*?(.+)') {
                $Checksum = $Matches[1].ToLower()
                $File = $Matches[2] -replace '^.+[/\\]', '' -replace '\*', ''

                if ($File -eq $FileName) {
                    $Found = $true
                    if ($Checksum -eq $ComputedHash) {
                        Write-Info "Checksum verified!"
                    } else {
                        Write-Err "Checksum mismatch!"
                        Write-Err "Expected: $Checksum"
                        Write-Err "Got:      $ComputedHash"
                        exit 1
                    }
                }
            }
        }

        if (-not $Found) {
            Write-Err "Archive not found in checksums file"
            Write-Err "Looking for: $FileName"
            exit 1
        }

        if ($Extension -eq 'zip') {
            Expand-Archive -Path $ArchivePath -DestinationPath $TempDir -Force
            $BinaryPath = Join-Path $TempDir 'pyx.exe'
        } else {
            Write-Err "tar.gz extraction not supported on Windows. Use -FromSource instead."
            exit 1
        }

        if (-not (Test-Path $BinaryPath)) {
            Write-Err "Binary not found after extraction"
            exit 1
        }

        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        Copy-Item -Path $BinaryPath -Destination (Join-Path $InstallDir 'pyx.exe') -Force

        return Join-Path $InstallDir 'pyx.exe'
    }
    finally {
    }
}

function Add-InstallDirToPath {
    $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')

    if ($UserPath -notlike "*$InstallDir*") {
        $NewPath = "$InstallDir;$UserPath"
        [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
        $env:Path = "$InstallDir;$env:Path"
        Write-Info "Added $InstallDir to PATH"
        Write-Info "Restart your terminal for changes to take effect"
    } else {
        Write-Info "$InstallDir already in PATH"
    }
}

function Main {
    Write-Host "==========================================" -ForegroundColor Cyan
    Write-Host "           pyx Installer (Windows)" -ForegroundColor Cyan
    Write-Host "=========================================="
    Write-Host ""

    $OS = Get-OperatingSystem
    Write-Info "Detected: $OS"

    $TargetTriple = Get-TargetTriple -OS $OS
    $Extension = Get-ArchiveExtension -TargetTriple $TargetTriple
    Write-Info "Target: $TargetTriple"

    Test-Prerequisites -FromSource $FromSource

    $InstallPath = $null

    if ($FromSource) {
        $InstallPath = Build-FromSource
    } else {
        if (-not $Version) {
            $Version = Get-LatestReleaseVersion
        }
        Write-Info "Installing version: $Version"
        $InstallPath = Invoke-BinaryDownload -Version $Version -TargetTriple $TargetTriple -Extension $Extension
    }

    if ($InstallPath) {
        Add-InstallDirToPath

        Write-Host ""
        Write-Info "Installation complete!"
        Write-Info "Run 'pyx --help' to get started."
    }
}

Main
