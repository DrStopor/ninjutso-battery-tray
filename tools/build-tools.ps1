# Shared build helpers; dot-source from a build or verification script.

function Initialize-ProjectToolchain {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $toolchain = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'rust-toolchain.toml')
    if ($toolchain -notmatch '(?m)^channel\s*=\s*"([^"]+)"') {
        throw 'Could not read the pinned Rust toolchain.'
    }
    $localRustup = Join-Path $RepoRoot 'target\tooling\rustup'
    $localBin = Join-Path $localRustup "toolchains\$($Matches[1])-x86_64-pc-windows-msvc\bin"
    # Normal development uses rustup. Restricted environments can keep the
    # compiler inside the repository instead of changing the global toolchain.
    if (Test-Path -LiteralPath (Join-Path $localBin 'rustc.exe') -PathType Leaf) {
        $env:RUSTUP_HOME = $localRustup
        $env:CARGO_HOME = Join-Path $RepoRoot 'target\tooling\cargo'
        $env:PATH = "$localBin;$env:PATH"
    }
    if ($env:GPUI_FXC_PATH) {
        if (-not (Test-Path -LiteralPath $env:GPUI_FXC_PATH -PathType Leaf)) {
            throw "GPUI_FXC_PATH does not exist: $env:GPUI_FXC_PATH"
        }
    } else {
        $fxc = Get-Command fxc.exe -ErrorAction SilentlyContinue
        if ($fxc) { $env:GPUI_FXC_PATH = $fxc.Source }
        else {
            $sdkBin = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Windows Kits\10\bin'
            if (Test-Path -LiteralPath $sdkBin -PathType Container) {
                $versions = Get-ChildItem -LiteralPath $sdkBin -Directory |
                    Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
                    Sort-Object { [version]$_.Name } -Descending
                foreach ($version in $versions) {
                    $candidate = Join-Path $version.FullName 'x64\fxc.exe'
                    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                        $env:GPUI_FXC_PATH = $candidate
                        break
                    }
                }
            }
        }
    }
}

function Invoke-CheckedNative {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @()
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE."
    }
}

function Get-ProjectVersion {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $cargo = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'Cargo.toml')
    if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw 'Could not read package version from Cargo.toml.'
    }
    return $Matches[1]
}

function Find-InnoCompiler {
    param([Parameter(Mandatory)][string]$RepoRoot)
    if ($env:INNO_SETUP_COMPILER) {
        if (-not (Test-Path -LiteralPath $env:INNO_SETUP_COMPILER -PathType Leaf)) {
            throw "INNO_SETUP_COMPILER does not exist: $env:INNO_SETUP_COMPILER"
        }
        return $env:INNO_SETUP_COMPILER
    }
    $programFilesX86 = [Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
    $candidates = @(
        (Join-Path $RepoRoot 'target\tooling\inno-6.7.4\{app}\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
        (Join-Path $programFilesX86 'Inno Setup 7\ISCC.exe')
    )
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) { $candidates += $command.Source }
    $candidates += @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path $programFilesX86 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    throw 'Install Inno Setup 6.7.4 or newer, or set INNO_SETUP_COMPILER.'
}
