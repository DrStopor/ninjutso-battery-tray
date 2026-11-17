[CmdletBinding()]
param([Parameter(Mandatory)][string]$Destination)
$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
foreach ($line in Get-Content -LiteralPath (Join-Path $RepoRoot 'licenses/copyleft-sources.sha256')) {
    if ($line -notmatch '^([0-9a-f]{64})\s+([a-zA-Z0-9._-]+\.crate)$') { throw 'Invalid source archive manifest.' }
    $expected=$Matches[1]; $filename=$Matches[2]
    $source = Get-ChildItem -LiteralPath (Join-Path $cargoHome 'registry/cache') -Directory |
        ForEach-Object { Join-Path $_.FullName $filename } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $source) { throw "Source archive $filename is missing; run cargo fetch --locked." }
    if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expected) { throw "Source checksum mismatch: $filename" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $Destination $filename)
}
Write-Host 'Collected and verified the original MPL source archives.'
