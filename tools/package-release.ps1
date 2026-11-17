[CmdletBinding()]
param([string]$OutputDirectory = 'dist')

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot

$Version = Get-ProjectVersion -RepoRoot $RepoRoot
$InnoCompiler = Find-InnoCompiler -RepoRoot $RepoRoot
$DistDir = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
    [IO.Path]::GetFullPath($OutputDirectory)
} else {
    [IO.Path]::GetFullPath((Join-Path $RepoRoot $OutputDirectory))
}
$StageBase = [IO.Path]::GetFullPath((Join-Path $RepoRoot 'target\release-packaging'))
$StageDir = Join-Path $StageBase ([Guid]::NewGuid().ToString('N'))
$ExeName = 'ninjutso-battery-tray.exe'
$ReleaseExe = Join-Path $RepoRoot "target\release\$ExeName"
$InstallerScript = Join-Path $RepoRoot 'installer\ninjutso-battery-tray.iss'

New-Item -ItemType Directory -Path $StageDir -Force | Out-Null
Push-Location $RepoRoot
try {
    & (Join-Path $PSScriptRoot 'update-licenses.ps1') -Check
    $MplSourceDir = Join-Path $StageDir 'mpl-sources'
    & (Join-Path $PSScriptRoot 'collect-license-sources.ps1') -Destination $MplSourceDir
    Invoke-CheckedNative -FilePath cargo -Arguments @('build', '--release', '--locked')
    $binaryVersion = (Get-Item -LiteralPath $ReleaseExe).VersionInfo.ProductVersion
    if ($binaryVersion -ne $Version) {
        throw "EXE version '$binaryVersion' does not match Cargo.toml '$Version'."
    }
    Invoke-CheckedNative -FilePath $InnoCompiler -Arguments @(
        "/DMyAppVersion=$Version", "/DMplSourcesDir=$MplSourceDir", "/O$StageDir", $InstallerScript
    )

    $SetupExe = Join-Path $StageDir "NinjutsoBatteryTraySetup-$Version.exe"
    if (-not (Test-Path -LiteralPath $SetupExe -PathType Leaf)) {
        throw 'The installer compiler did not produce the expected file.'
    }
    $PortableDir = Join-Path $StageDir "ninjutso-battery-tray-$Version-portable"
    $PortableZip = Join-Path $StageDir "ninjutso-battery-tray-$Version-portable.zip"
    $Checksums = Join-Path $StageDir 'SHA256SUMS.txt'
    $GitFlicInstallerZip = Join-Path $StageDir "NinjutsoBatteryTraySetup-$Version.zip"
    $GitFlicChecksumsZip = Join-Path $StageDir "SHA256SUMS-$Version.zip"

    New-Item -ItemType Directory -Path $PortableDir | Out-Null
    Copy-Item -LiteralPath $ReleaseExe -Destination (Join-Path $PortableDir $ExeName)
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'LICENSE') -Destination (Join-Path $PortableDir 'LICENSE.txt')
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'THIRD_PARTY_NOTICES.txt') -Destination (Join-Path $PortableDir 'THIRD_PARTY_NOTICES.txt')
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'CHANGELOG.md') -Destination (Join-Path $PortableDir 'CHANGELOG.md')
    $PortableLicenses = Join-Path $PortableDir 'licenses'
    New-Item -ItemType Directory -Path $PortableLicenses | Out-Null
    Copy-Item -LiteralPath $MplSourceDir -Destination (Join-Path $PortableLicenses 'sources') -Recurse
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'licenses/MPL-SOURCES.md') -Destination $PortableLicenses
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'licenses/copyleft-sources.sha256') -Destination $PortableLicenses
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'vendor/xattr/UPSTREAM.md') -Destination (Join-Path $PortableLicenses 'xattr-UPSTREAM.md')
    $PortableFonts = Join-Path $PortableLicenses 'fonts'
    New-Item -ItemType Directory -Path $PortableFonts | Out-Null
    foreach ($name in @('UPSTREAM.md', 'SHA256SUMS')) {
        Copy-Item -LiteralPath (Join-Path $RepoRoot "assets/fonts/$name") -Destination $PortableFonts
    }
    foreach ($family in @('source-sans-3', 'noto-sans-sc')) {
        $directory = Join-Path $PortableFonts $family
        New-Item -ItemType Directory -Path $directory | Out-Null
        Copy-Item -LiteralPath (Join-Path $RepoRoot "assets/fonts/$family/OFL.txt") -Destination $directory
        $copyright = Join-Path $RepoRoot "assets/fonts/$family/NOTICE.txt"
        if (Test-Path -LiteralPath $copyright) { Copy-Item -LiteralPath $copyright -Destination $directory }
    }
    @"
Ninjutso Battery Tray $Version

Portable version.

Run:
  $ExeName

Settings are stored in:
  %APPDATA%\NinjutsoBatteryTray\settings.txt

License:
  MIT License (application); SIL OFL 1.1 (bundled fonts)
"@ | Set-Content -LiteralPath (Join-Path $PortableDir 'README.txt') -Encoding UTF8

    Compress-Archive -Path (Join-Path $PortableDir '*') -DestinationPath $PortableZip
    $hashLines = foreach ($file in @($SetupExe, $PortableZip)) {
        $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
        '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $file)
    }
    $hashLines | Set-Content -LiteralPath $Checksums -Encoding ASCII
    Compress-Archive -LiteralPath $SetupExe -DestinationPath $GitFlicInstallerZip
    Compress-Archive -LiteralPath $Checksums -DestinationPath $GitFlicChecksumsZip

    # Copy only after every build/packaging step succeeds. Previous releases
    # remain available even if a new compiler or packaging command fails.
    New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
    foreach ($file in @($SetupExe, $PortableZip, $Checksums, $GitFlicInstallerZip, $GitFlicChecksumsZip)) {
        $destination = Join-Path $DistDir (Split-Path -Leaf $file)
        Copy-Item -LiteralPath $file -Destination $destination -Force
        Write-Host $destination
    }
} finally {
    Pop-Location
    $resolvedStage = [IO.Path]::GetFullPath($StageDir)
    $allowedPrefix = $StageBase.TrimEnd('\') + '\'
    if (-not $resolvedStage.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a staging path outside $StageBase."
    }
    if (Test-Path -LiteralPath $resolvedStage) {
        Remove-Item -LiteralPath $resolvedStage -Recurse -Force
    }
}
