[CmdletBinding()]
param([switch]$CheckLinuxCore)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot

Push-Location $RepoRoot
try {
    & (Join-Path $PSScriptRoot 'verify-docs.ps1')
    & (Join-Path $PSScriptRoot 'update-licenses.ps1') -Check
    Invoke-CheckedNative -FilePath cargo -Arguments @('fmt', '--all', '--check')
    Invoke-CheckedNative -FilePath cargo -Arguments @('clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings')
    Invoke-CheckedNative -FilePath cargo -Arguments @('test', '--workspace', '--locked')
    Invoke-CheckedNative -FilePath cargo -Arguments @('clippy', '--workspace', '--all-targets', '--no-default-features', '--target-dir', 'target/native', '--locked', '--', '-D', 'warnings')
    Invoke-CheckedNative -FilePath cargo -Arguments @('test', '--workspace', '--no-default-features', '--target-dir', 'target/native', '--locked')
    Invoke-CheckedNative -FilePath cargo -Arguments @('build', '--release', '--locked')
    if ($CheckLinuxCore) {
        # The desktop now links real Linux system libraries; cross-check only
        # the dependency-free core here. tools/verify-linux.sh checks the host.
        Invoke-CheckedNative -FilePath cargo -Arguments @('check', '-p', 'ninjutso-battery-core', '--all-targets', '--locked', '--target', 'x86_64-unknown-linux-gnu')
    }
    $expected = Get-ProjectVersion -RepoRoot $RepoRoot
    $actual = (Get-Item -LiteralPath 'target\release\ninjutso-battery-tray.exe').VersionInfo.ProductVersion
    if ($actual -ne $expected) { throw "EXE version '$actual' does not match '$expected'." }
    Write-Host "Verified Ninjutso Battery Tray $expected."
} finally {
    Pop-Location
}
