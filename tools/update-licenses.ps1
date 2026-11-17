[CmdletBinding()]
param([switch]$Check)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot
$Output = Join-Path $RepoRoot 'THIRD_PARTY_NOTICES.txt'
$Stamp = Join-Path $RepoRoot 'licenses\inputs.sha256'
& (Join-Path $PSScriptRoot 'verify-fonts.ps1')
$FontLicenses = @('assets/fonts/source-sans-3/OFL.txt', 'assets/fonts/noto-sans-sc/OFL.txt', 'assets/fonts/noto-sans-sc/NOTICE.txt')
$Inputs = @('LICENSE', 'Cargo.toml', 'crates/core/Cargo.toml', 'Cargo.lock', 'licenses/about.toml', 'licenses/notices.hbs', 'licenses/copyleft-sources.sha256', 'licenses/MPL-SOURCES.md', 'tools/update-licenses.ps1', 'tools/verify-fonts.ps1', 'assets/fonts/UPSTREAM.md', 'assets/fonts/SHA256SUMS') + $FontLicenses
$VendorInputs = @(Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'vendor/xattr') -File -Recurse |
    Where-Object { $_.Extension -eq '.rs' -or $_.Name -in @('Cargo.toml','LICENSE-MIT','LICENSE-APACHE','UPSTREAM.md','README.md') } |
    ForEach-Object { [IO.Path]::GetRelativePath($RepoRoot, $_.FullName).Replace('\','/') })
[Array]::Sort($VendorInputs, [StringComparer]::Ordinal)
$Inputs += $VendorInputs
$lockText = (Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'Cargo.lock')).Replace("`r`n", "`n")
$SourceArchives = @([regex]::Matches($lockText, '(?ms)^\[\[package\]\]\nname = "(dwrote|option-ext)"\nversion = "([^"]+)"\nsource = "[^"]+"\nchecksum = "([0-9a-f]{64})"') |
    ForEach-Object { '{0}  {1}-{2}.crate' -f $_.Groups[3].Value, $_.Groups[1].Value, $_.Groups[2].Value })
$PinnedArchives = @(Get-Content -LiteralPath (Join-Path $RepoRoot 'licenses/copyleft-sources.sha256'))
[Array]::Sort($SourceArchives, [StringComparer]::Ordinal)
[Array]::Sort($PinnedArchives, [StringComparer]::Ordinal)
if (($SourceArchives -join "`n") -cne ($PinnedArchives -join "`n")) {
    throw 'MPL source archive versions/hashes do not match Cargo.lock; update licenses/copyleft-sources.sha256.'
}
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
    # Preserve the license wording, normalizing upstream trailing whitespace.
    $noticeText = [IO.File]::ReadAllText($Output)
    foreach ($fontLicense in $FontLicenses) {
        $noticeText += "`n-------------------------------------------------------------------------------`nBundled font license: $fontLicense`n`n"
        $noticeText += [IO.File]::ReadAllText((Join-Path $RepoRoot $fontLicense))
    }
    $noticeText = [regex]::Replace($noticeText, '(?m)[ \t]+(?=\r?$)', '')
    [IO.File]::WriteAllText($Output, $noticeText, [Text.UTF8Encoding]::new($false))
    $Hashes += '{0}  THIRD_PARTY_NOTICES.txt' -f (Get-TextHash $Output)
    # Generated checksum data; no timestamps or machine-specific paths.
    $Hashes | Set-Content -LiteralPath $Stamp -Encoding ascii
} finally { Pop-Location }
