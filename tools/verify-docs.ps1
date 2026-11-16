[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')

$Version = Get-ProjectVersion -RepoRoot $RepoRoot
$toolchain = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'rust-toolchain.toml')
$rustMatch = [regex]::Match($toolchain, '(?m)^channel\s*=\s*"([^"]+)"')
if (-not $rustMatch.Success) { throw 'Could not read the pinned Rust version.' }
$RustVersion = $rustMatch.Groups[1].Value
$installer = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'installer\ninjutso-battery-tray.iss')
$innoMatch = [regex]::Match($installer, '#if Ver < EncodeVer\((\d+),\s*(\d+),\s*(\d+)\)')
if (-not $innoMatch.Success) { throw 'Could not read the minimum Inno Setup version.' }
$InnoVersion = '{0}.{1}.{2}' -f $innoMatch.Groups[1].Value, $innoMatch.Groups[2].Value, $innoMatch.Groups[3].Value
$sourceVersion = $Version.Replace('-', '--')
$ExpectedBadges = @(
    "https://img.shields.io/badge/source-v$sourceVersion-2563eb",
    'https://img.shields.io/github/v/release/DrStopor/ninjutso-battery-tray?display_name=tag&sort=semver&label=release',
    'https://img.shields.io/badge/Windows-x64-0078D4',
    "https://img.shields.io/badge/Rust-$RustVersion-000000?logo=rust&logoColor=white",
    "https://img.shields.io/badge/Inno%20Setup-$InnoVersion%2B-2ea44f",
    'https://img.shields.io/badge/license-MIT%20%2B%20Commons%20Clause-f59e0b'
)
$Readmes = @('README.md', 'README.ru.md', 'README.be.md', 'README.kk.md', 'README.zh-CN.md')
$linkPatterns = @(
    '!\[[^\]]*\]\(([^)\r\n]+)\)',
    '(?m)(?<![!])\[[^\[\]\r\n]*\]\(([^)\r\n]+)\)',
    '\[!\[[^\]]*\]\([^)\r\n]+\)\]\(([^)\r\n]+)\)'
)

foreach ($readme in $Readmes) {
    $text = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot $readme)
    $badges = [regex]::Matches($text, '!\[([^\]]*)\]\((https://img\.shields\.io/[^)\r\n]+)\)')
    if ($badges.Count -ne $ExpectedBadges.Count) {
        throw "$($readme): expected $($ExpectedBadges.Count) badges, found $($badges.Count)."
    }
    for ($i = 0; $i -lt $ExpectedBadges.Count; $i++) {
        if ($badges[$i].Groups[2].Value -cne $ExpectedBadges[$i]) {
            throw "$($readme): outdated or inconsistent badge: $($badges[$i].Groups[2].Value)"
        }
    }
    foreach ($pair in @(@(0, $Version), @(3, $RustVersion), @(4, $InnoVersion))) {
        if (-not $badges[$pair[0]].Groups[1].Value.Contains($pair[1])) {
            throw "$($readme): outdated badge alternative text."
        }
    }
    foreach ($requirement in @(@('Rust', $RustVersion), @('Inno Setup', $InnoVersion))) {
        $pattern = '(?m)^- .*' + [regex]::Escape($requirement[0]) + '\s+(\d+\.\d+\.\d+)'
        $match = [regex]::Match($text, $pattern)
        if (-not $match.Success -or $match.Groups[1].Value -cne $requirement[1]) {
            throw "$($readme): outdated $($requirement[0]) build requirement."
        }
    }
    if ($text.Contains('docs/architecture.md')) {
        throw "$($readme): public documentation must not link to the local architecture note."
    }
    foreach ($pattern in $linkPatterns) {
        foreach ($match in [regex]::Matches($text, $pattern)) {
            $target = $match.Groups[1].Value
            if ($target -match '^(https?://|mailto:|#)') { continue }
            $relative = [uri]::UnescapeDataString(($target -split '[#?]', 2)[0])
            if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $relative) -PathType Leaf)) {
                throw "$($readme): broken local link: $target"
            }
        }
    }
}
Write-Host "Verified versions, badges and local links in all $($Readmes.Count) README files."
