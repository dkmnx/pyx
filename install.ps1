#Requires -Version 3.0
$ErrorActionPreference = 'Stop'

$Repo = 'dkmnx/pyx'
$Binary = 'pyx.exe'
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { "$env:LOCALAPPDATA\Programs\pyx" }

$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x86_64' } else { 'i386' }

$Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases"
$Version = ($Releases[0].tag_name -replace '^v', '')

$Filename = "pyx_${Version}_windows_${Arch}.zip"
$Asset = $Releases[0].assets | Where-Object { $_.name -eq $Filename } | Select-Object -First 1

if (-not $Asset) { throw "Asset not found: $Filename" }

$TmpDir = Join-Path $env:TEMP "pyx-install-$(Get-Random)"
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
    $ZipPath = Join-Path $TmpDir $Filename
    Write-Host "Downloading pyx $Version..."
    Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $ZipPath

    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir | Out-Null
    }

    Move-Item -Path (Join-Path $TmpDir $Binary) -Destination (Join-Path $InstallDir $Binary) -Force

    $CurrentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($CurrentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable('Path', "$CurrentPath;$InstallDir", 'User')
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "Added $InstallDir to user PATH"
    }

    Write-Host "Installed pyx to $InstallDir"
} finally {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
