[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$CargoToml = Join-Path $RepoRoot 'Cargo.toml'
$DistDir = Join-Path $RepoRoot 'dist'
$ExeName = 'ninjutso-battery-tray.exe'
$ReleaseExe = Join-Path $RepoRoot "target\release\$ExeName"
$InstallerScript = Join-Path $RepoRoot 'installer\ninjutso-battery-tray.iss'

function Get-ProjectVersion {
    $cargo = Get-Content -Raw -LiteralPath $CargoToml

    if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw 'Could not read package version from Cargo.toml.'
    }

    return $Matches[1]
}

function Find-InnoCompiler {
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $candidates = @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
    )

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return $candidate
        }
    }

    throw 'ISCC.exe was not found. Install Inno Setup 6 and run this script again.'
}

function Reset-Directory {
    param([string]$Path)

    $resolvedRoot = [System.IO.Path]::GetFullPath($RepoRoot)
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)

    if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove path outside repository: $resolvedPath"
    }

    if (Test-Path -LiteralPath $resolvedPath) {
        Remove-Item -LiteralPath $resolvedPath -Recurse -Force
    }

    New-Item -ItemType Directory -Path $resolvedPath -Force | Out-Null
}

$Version = Get-ProjectVersion
$InnoCompiler = Find-InnoCompiler
$SetupExe = Join-Path $DistDir "NinjutsoBatteryTraySetup-$Version.exe"
$PortableDir = Join-Path $DistDir "ninjutso-battery-tray-$Version-portable"
$PortableZip = Join-Path $DistDir "ninjutso-battery-tray-$Version-portable.zip"
$Checksums = Join-Path $DistDir 'SHA256SUMS.txt'
$GitFlicInstallerZip = Join-Path $DistDir "NinjutsoBatteryTraySetup-$Version.zip"
$GitFlicChecksumsZip = Join-Path $DistDir "SHA256SUMS-$Version.zip"

Push-Location $RepoRoot
try {
    Reset-Directory -Path $DistDir

    cargo build --release
    & $InnoCompiler "/DMyAppVersion=$Version" $InstallerScript

    Reset-Directory -Path $PortableDir
    Copy-Item -LiteralPath $ReleaseExe -Destination (Join-Path $PortableDir $ExeName) -Force
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'LICENSE') -Destination (Join-Path $PortableDir 'LICENSE.txt') -Force
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'CHANGELOG.md') -Destination (Join-Path $PortableDir 'CHANGELOG.md') -Force

    @"
Ninjutso Battery Tray $Version

Portable version.

Run:
  $ExeName

Settings are stored in:
  %APPDATA%\NinjutsoBatteryTray\settings.txt

License:
  MIT License with Commons Clause License Condition v1.0
"@ | Set-Content -LiteralPath (Join-Path $PortableDir 'README.txt') -Encoding UTF8

    if (Test-Path -LiteralPath $PortableZip) {
        Remove-Item -LiteralPath $PortableZip -Force
    }

    Compress-Archive -Path (Join-Path $PortableDir '*') -DestinationPath $PortableZip -Force

    $releaseFiles = @($SetupExe, $PortableZip)
    $hashLines = foreach ($file in $releaseFiles) {
        $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
        "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $file)
    }
    $hashLines | Set-Content -LiteralPath $Checksums -Encoding ASCII

    Compress-Archive -LiteralPath $SetupExe -DestinationPath $GitFlicInstallerZip -Force
    Compress-Archive -LiteralPath $Checksums -DestinationPath $GitFlicChecksumsZip -Force

    Write-Host "Release artifacts:"
    Write-Host "  $SetupExe"
    Write-Host "  $PortableZip"
    Write-Host "  $Checksums"
    Write-Host "  $GitFlicInstallerZip"
    Write-Host "  $GitFlicChecksumsZip"
} finally {
    Pop-Location
}
