# jbundle installer for Windows
# Usage: irm https://raw.githubusercontent.com/avelino/jbundle/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = "avelino/jbundle"
$Artifact = "jbundle-windows-x86_64"
$InstallDir = if ($env:JBUNDLE_INSTALL_DIR) { $env:JBUNDLE_INSTALL_DIR } else { "$env:USERPROFILE\.jbundle\bin" }

function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
        return $release.tag_name
    } catch {
        return $null
    }
}

$Version = if ($env:JBUNDLE_VERSION) { $env:JBUNDLE_VERSION } else { $null }
if (-not $Version) {
    Write-Host "Fetching latest version..."
    $Version = Get-LatestVersion
    if (-not $Version) {
        Write-Host "Could not determine latest version, using pre-release..."
        $Version = "latest"
    }
}

if ($Version -eq "latest") {
    $ZipName = "$Artifact.zip"
} else {
    $ZipName = "$Artifact-$Version.zip"
}

$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$ZipName"

Write-Host "Downloading jbundle $Version for Windows..."
Write-Host "  $DownloadUrl"

$TmpDir = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "jbundle-install-$(Get-Random)")
$ZipPath = Join-Path $TmpDir $ZipName

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item (Join-Path $TmpDir "jbundle.exe") (Join-Path $InstallDir "jbundle.exe") -Force

    # Add to PATH if not already there
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$UserPath", "User")
        Write-Host ""
        Write-Host "Added $InstallDir to your PATH."
        Write-Host "Restart your terminal for PATH changes to take effect."
    }

    Write-Host ""
    Write-Host "jbundle installed to $InstallDir\jbundle.exe"
    Write-Host ""
    Write-Host "Run 'jbundle --help' to get started."
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
