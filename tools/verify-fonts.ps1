[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$listed = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($line in Get-Content -LiteralPath (Join-Path $root 'assets/fonts/SHA256SUMS')) {
    if ($line -notmatch '^([a-f0-9]{64})  (assets/fonts/(?:source-sans-3|noto-sans-sc)/[A-Za-z0-9.-]+)$') {
        throw "Invalid font manifest entry: $line"
    }
    $expected = $Matches[1]
    $path = $Matches[2]
    if (-not $listed.Add($path)) { throw "Duplicate font manifest entry: $path" }
    $actual = (Get-FileHash -LiteralPath (Join-Path $root $path) -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne $expected) { throw "Bundled font/license checksum mismatch: $path" }
}
if ($listed.Count -ne 8) { throw 'Expected five font files, two OFL licenses and Noto copyright notice.' }
foreach ($file in Get-ChildItem -LiteralPath (Join-Path $root 'assets/fonts') -Recurse -File) {
    if ($file.Extension -notin @('.otf', '.ttf') -and $file.Name -notin @('OFL.txt', 'NOTICE.txt')) { continue }
    $relative = [IO.Path]::GetRelativePath($root, $file.FullName).Replace('\', '/')
    if (-not $listed.Contains($relative)) { throw "Unpinned font/license asset: $relative" }
}
Write-Host 'Verified five bundled font files and all original font notices.'
