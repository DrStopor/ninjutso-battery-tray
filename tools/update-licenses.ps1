[CmdletBinding()]
param([switch]$Check)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot
$Output = Join-Path $RepoRoot 'THIRD_PARTY_NOTICES.txt'
$Stamp = Join-Path $RepoRoot 'licenses\inputs.sha256'
$Inputs = @('Cargo.toml', 'crates/core/Cargo.toml', 'Cargo.lock', 'licenses/about.toml', 'licenses/notices.hbs', 'tools/update-licenses.ps1')
function Get-TextHash([string]$Path) {
    # Git checkouts may use LF or CRLF; that must not invalidate the same text.
    $text = (Get-Content -Raw -LiteralPath $Path).Replace("`r`n", "`n")
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))).ToLowerInvariant()
}
$Hashes = foreach ($inputFile in $Inputs) {
    '{0}  {1}' -f (Get-TextHash (Join-Path $RepoRoot $inputFile)), $inputFile
}
if ($Check) {
    if (-not (Test-Path -LiteralPath $Output) -or -not (Test-Path -LiteralPath $Stamp)) {
        throw 'Third-party notices are missing. Run tools/update-licenses.ps1.'
    }
    $Hashes += '{0}  THIRD_PARTY_NOTICES.txt' -f (Get-TextHash $Output)
    if ((Get-Content -LiteralPath $Stamp) -join "`n" -cne ($Hashes -join "`n")) {
        throw 'Third-party notices are stale. Regenerate with tools/update-licenses.ps1.'
    }
    Write-Host 'Verified third-party notices against Cargo.lock and generation inputs.'
    return
}
$Tool = Join-Path $RepoRoot 'target\tooling\cargo-about\bin\cargo-about.exe'
if (-not (Test-Path -LiteralPath $Tool)) {
    $command = Get-Command cargo-about -ErrorAction SilentlyContinue
    if (-not $command) { throw 'Install cargo-about 0.9.3 with --features cli, or use --root target/tooling/cargo-about.' }
    $Tool = $command.Source
}
$toolVersion = & $Tool --version
if ($LASTEXITCODE -ne 0 -or $toolVersion -notmatch '\b0\.9\.3$') { throw 'cargo-about 0.9.3 is required for reproducible notices.' }
Push-Location $RepoRoot
try {
    Invoke-CheckedNative -FilePath $Tool -Arguments @('generate', '--locked', '--fail', '--config', 'licenses/about.toml',
        '--output-file', $Output, 'licenses/notices.hbs')
    $Hashes += '{0}  THIRD_PARTY_NOTICES.txt' -f (Get-TextHash $Output)
    # Generated checksum data; no timestamps or machine-specific paths.
    $Hashes | Set-Content -LiteralPath $Stamp -Encoding ascii
} finally { Pop-Location }
