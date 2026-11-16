[CmdletBinding()]
param([switch]$CheckLinuxCore)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot

Push-Location $RepoRoot
try {
    & (Join-Path $PSScriptRoot 'verify-docs.ps1')
    Invoke-CheckedNative -FilePath cargo -Arguments @('fmt', '--check')
    Invoke-CheckedNative -FilePath cargo -Arguments @('clippy', '--all-targets', '--locked', '--', '-D', 'warnings')
    Invoke-CheckedNative -FilePath cargo -Arguments @('test', '--locked')
    Invoke-CheckedNative -FilePath cargo -Arguments @('build', '--release', '--locked')
    if ($CheckLinuxCore) {
        Invoke-CheckedNative -FilePath cargo -Arguments @('check', '--all-targets', '--locked', '--target', 'x86_64-unknown-linux-gnu')
    }
    $expected = Get-ProjectVersion -RepoRoot $RepoRoot
    $actual = (Get-Item -LiteralPath 'target\release\ninjutso-battery-tray.exe').VersionInfo.ProductVersion
    if ($actual -ne $expected) { throw "EXE version '$actual' does not match '$expected'." }
    Write-Host "Verified Ninjutso Battery Tray $expected."
} finally {
    Pop-Location
}
