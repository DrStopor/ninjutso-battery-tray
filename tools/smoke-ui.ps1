[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$CaptureDirectory = 'target\verification\gpui',
    [ValidateSet('en', 'ru', 'be', 'kk', 'zh-CN')][string[]]$Languages = @('en', 'ru', 'be', 'kk', 'zh-CN'),
    [ValidateSet('auto', 'dark', 'light', 'ninjaforce')][string]$Theme = 'dark',
    [ValidateSet('charging', 'ready', 'offline', 'missing', 'error', 'reading')][string]$State = 'charging',
    [ValidateRange(0, 100)][int]$Percent = 72,
    [ValidateSet('usb', 'receiver')][string]$Connection = 'usb',
    [ValidateSet('1000', '2000', '4000', '8000', 'unknown')][string]$PollingRate = '1000',
    [string]$Dpi = '1100',
    [switch]$CaptureOnly,
    [int]$AttachToPid = 0,
    [ValidateRange(0, 5000)][int]$WaitForDataMs = 0,
    [switch]$CheckThemePersistence,
    [switch]$CheckDpiLayout
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot
. (Join-Path $PSScriptRoot 'dpi-layout.ps1')
if (-not $SkipBuild -and $AttachToPid -eq 0) {
    Push-Location $RepoRoot
    try { Invoke-CheckedNative -FilePath cargo -Arguments @('build', '--locked') } finally { Pop-Location }
}
$CaptureRoot = [IO.Path]::GetFullPath((Join-Path $RepoRoot $CaptureDirectory))
New-Item -ItemType Directory -Path $CaptureRoot -Force | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class GpuiSmoke {
    public delegate bool Callback(IntPtr window, IntPtr parameter);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr window, StringBuilder text, int count);
    public static string Class(IntPtr window) { var s=new StringBuilder(512); GetClassNameW(window,s,s.Capacity); return s.ToString(); }
    public static string Title(IntPtr window) { var s=new StringBuilder(512); GetWindowTextW(window,s,s.Capacity); return s.ToString(); }
    public static IntPtr[] Windows(int processId) {
        var windows = new List<IntPtr>();
        EnumWindows((window, parameter) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner == processId) windows.Add(window);
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }
}
'@

function Get-HostWindow([int]$OwnerPid) {
    foreach ($handle in [GpuiSmoke]::Windows($OwnerPid)) {
        if ([GpuiSmoke]::Class($handle) -eq 'NinjutsoBatteryTray.GpuiHost.v1') { return $handle }
    }
    return [IntPtr]::Zero
}
function Get-CardWindow([int]$OwnerPid) {
    foreach ($handle in [GpuiSmoke]::Windows($OwnerPid)) {
        if ([GpuiSmoke]::IsWindowVisible($handle) -and [GpuiSmoke]::Title($handle).StartsWith('Ninjutso Ten:')) { return $handle }
    }
    return [IntPtr]::Zero
}
function Wait-Card([Diagnostics.Process]$Process) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt 8000 -and -not $Process.HasExited) {
        $window = Get-CardWindow $Process.Id
        if ($window -ne [IntPtr]::Zero) { return $window }
        $hostWindow = Get-HostWindow $Process.Id
        if ($hostWindow -ne [IntPtr]::Zero) {
            [void][GpuiSmoke]::PostMessageW($hostWindow, 0x8001, [IntPtr]::Zero, [IntPtr]0x10401)
            Start-Sleep -Milliseconds 250
        } else { Start-Sleep -Milliseconds 50 }
    }
    throw "GPUI card did not open (PID $($Process.Id), exited: $($Process.HasExited))."
}
function Save-Card([IntPtr]$Window, [string]$Path) {
    [uint32]$owner = 0
    [void][GpuiSmoke]::GetWindowThreadProcessId($Window, [ref]$owner)
    for ($attempt = 0; $attempt -lt 12; $attempt++) {
        Start-Sleep -Milliseconds 120
        if (-not [GpuiSmoke]::IsWindowVisible($Window)) {
            # Desktop/approval focus can change between detecting an HWND and
            # capturing it. Reopen only this preview, never photograph its background.
            $hostWindow = Get-HostWindow $owner
            if ($hostWindow -eq [IntPtr]::Zero) { throw 'The application exited during capture.' }
            [void][GpuiSmoke]::PostMessageW($hostWindow, 0x8001, [IntPtr]::Zero, [IntPtr]0x10401)
            continue
        }
        $rect = [GpuiSmoke+Rect]::new()
        [void][GpuiSmoke]::GetWindowRect($Window, [ref]$rect)
        $scale = [GpuiSmoke]::GetDpiForWindow($Window) / 96.0
        if ([math]::Abs(($rect.Right-$rect.Left)-304*$scale) -gt 2 -or
            [math]::Abs(($rect.Bottom-$rect.Top)-156*$scale) -gt 2) {
            throw 'The polling-rate badge changed the expected 304 x 156 card size.'
        }
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left, $rect.Bottom-$rect.Top)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
            $colors = [Collections.Generic.HashSet[int]]::new()
            $surfacePixels = 0
            for ($y=8; $y -lt $bitmap.Height-8; $y+=8) {
                for ($x=8; $x -lt $bitmap.Width-8; $x+=8) {
                    $pixel=$bitmap.GetPixel($x,$y)
                    [void]$colors.Add($pixel.ToArgb())
                    if (($pixel.R -eq 25 -and $pixel.G -eq 28 -and $pixel.B -eq 33) -or
                        ($pixel.R -eq 250 -and $pixel.G -eq 251 -and $pixel.B -eq 253) -or
                        ($pixel.R -eq 195 -and $pixel.G -eq 189 -and $pixel.B -eq 219)) { $surfacePixels++ }
                }
            }
            # A visible HWND is not proof of a composited GPUI frame. Reject blank
            # surfaces and captures after an unexpected dismissal.
            if ($surfacePixels -gt 100 -and $colors.Count -ge 6 -and [GpuiSmoke]::IsWindowVisible($Window)) {
                $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
                return
            }
        } finally { $graphics.Dispose(); $bitmap.Dispose() }
    }
    throw 'GPUI did not present a nonblank frame for the screenshot.'
}
function Start-Preview([string[]]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new((Join-Path $RepoRoot 'target\debug\ninjutso-battery-tray.exe'))
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.ArgumentList.Add('--preview')
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    return [Diagnostics.Process]::Start($info)
}
function Stop-Preview([Diagnostics.Process]$Process) {
    if (-not $Process.HasExited) {
        $hostWindow = Get-HostWindow $Process.Id
        if ($hostWindow -ne [IntPtr]::Zero) { [void][GpuiSmoke]::PostMessageW($hostWindow, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) }
        if (-not $Process.WaitForExit(4000)) { $Process.Kill(); throw 'Preview failed to exit through its tray host.' }
    }
    if ($Process.ExitCode -ne 0) { throw "Preview exit code: $($Process.ExitCode)" }
    $Process.Dispose()
}

if ($AttachToPid -ne 0) {
    $process = [Diagnostics.Process]::GetProcessById($AttachToPid)
    # Keep a native process handle before it exits; GetProcessById alone may
    # otherwise lose the exit code when the PID disappears during cleanup.
    $null = $process.Handle
    $attachedPath = [IO.Path]::GetFullPath($process.MainModule.FileName)
    $allowedPrefix = [IO.Path]::GetFullPath((Join-Path $RepoRoot 'target')).TrimEnd('\') + '\'
    if (-not $attachedPath.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($attachedPath) -ne 'ninjutso-battery-tray.exe') {
        throw 'AttachToPid accepts only a Ninjutso Battery Tray build from this repository target directory.'
    }
    try {
        $card = Wait-Card $process
        if ($WaitForDataMs -gt 0) { Start-Sleep -Milliseconds $WaitForDataMs }
        Save-Card $card (Join-Path $CaptureRoot 'attached.png')
        if ($CheckDpiLayout) {
            Assert-DpiScreenshot (Join-Path $CaptureRoot 'attached.png') ([GpuiSmoke]::Title($card).Contains('DPI')) ([int]([regex]::Match([GpuiSmoke]::Title($card),'(\d+)%').Groups[1].Value))
        }
        Write-Host "Captured running application: $([GpuiSmoke]::Title($card))"
    }
    finally { Stop-Preview $process }
    return
}

foreach ($language in $Languages) {
    # Simulated charging data: reproducible UI previews, never persisted.
    $preview = Start-Preview @("--preview-language=$language", "--preview-theme=$Theme", "--preview-percent=$Percent", "--preview-state=$State", "--preview-connection=$Connection", "--preview-polling-rate=$PollingRate", "--preview-dpi=$Dpi")
    try {
        $card = Wait-Card $preview
        Save-Card $card (Join-Path $CaptureRoot "flyout.$language.png")
        if ($CheckDpiLayout) {
            Assert-DpiScreenshot (Join-Path $CaptureRoot "flyout.$language.png") ([GpuiSmoke]::Title($card).Contains('DPI')) $Percent
        }
        Write-Host "Captured $language ($Theme), title: $([GpuiSmoke]::Title($card))"
    } finally { Stop-Preview $preview }
}
if ($CaptureOnly) { return }

$preview = Start-Preview @('--preview-language=en', "--preview-theme=$Theme", '--preview-cycle')
$StateRoot = Join-Path $RepoRoot 'target\verification\gpui-states'
New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
try {
    $card = Wait-Card $preview
    $hostWindow = Get-HostWindow $preview.Id
    $titles = [Collections.Generic.HashSet[string]]::new()
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt 9700) {
        $current = Get-CardWindow $preview.Id
        if ($current -ne $card) { throw 'Live state update replaced or dismissed the GPUI window.' }
        if ($titles.Add([GpuiSmoke]::Title($card))) {
            Save-Card $card (Join-Path $StateRoot "state.$($titles.Count).png")
        }
        Start-Sleep -Milliseconds 100
    }
    foreach ($expected in @('72%', '73%', '74%', 'Disconnected', 'Mouse not found', 'Could not read', 'Reading battery', '1000 Hz', '4000 Hz', '1100 DPI', '1600 DPI')) {
        if (-not ($titles | Where-Object { $_.Contains($expected) })) { throw "Live state not rendered: $expected" }
    }
    foreach ($hz in @('1000 Hz', '4000 Hz')) {
        if (-not ($titles | Where-Object { $_.Contains('72%') -and $_.Contains($hz) })) {
            throw 'Changing only the polling rate did not update the existing card.'
        }
    }
    foreach ($value in @('1100 DPI', '1600 DPI')) {
        if (-not ($titles | Where-Object { $_.Contains('72%') -and $_.Contains('4000 Hz') -and $_.Contains($value) })) {
            throw 'Changing only DPI did not update the existing card.'
        }
    }
    [void][GpuiSmoke]::PostMessageW($card, 0x100, [IntPtr]0x1b, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    if ([GpuiSmoke]::IsWindowVisible($card)) { throw 'Escape did not dismiss the card.' }
    $card = Wait-Card $preview
    [void][GpuiSmoke]::PostMessageW($hostWindow, 0x8001, [IntPtr]::Zero, [IntPtr]0x10401)
    Start-Sleep -Milliseconds 200
    if ([GpuiSmoke]::IsWindowVisible($card)) { throw 'A repeated tray selection did not close the card.' }
    $card = Wait-Card $preview
    $scale = [GpuiSmoke]::GetDpiForWindow($card) / 96.0
    $point = ([int](28*$scale) -shl 16) -bor [int](275*$scale)
    [void][GpuiSmoke]::PostMessageW($card, 0x201, [IntPtr]1, [IntPtr]$point)
    [void][GpuiSmoke]::PostMessageW($card, 0x202, [IntPtr]::Zero, [IntPtr]$point)
    Start-Sleep -Milliseconds 250
    if ([GpuiSmoke]::IsWindowVisible($card)) { throw 'The close button did not dismiss the card.' }
    $card = Wait-Card $preview
    [void][GpuiSmoke]::PostMessageW($card, 0x100, [IntPtr]9, [IntPtr]::Zero)
    [void][GpuiSmoke]::PostMessageW($card, 0x100, [IntPtr]13, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    if ([GpuiSmoke]::IsWindowVisible($card)) { throw 'Tab and Enter did not activate the close button.' }
    $card = Wait-Card $preview
    [void][GpuiSmoke]::PostMessageW($card, 6, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 100
    if ([GpuiSmoke]::IsWindowVisible($card)) { throw 'Deactivation did not dismiss the card.' }
    [void][GpuiSmoke]::PostMessageW($hostWindow, 0x8001, [IntPtr]::Zero, [IntPtr]0x10400)
    Start-Sleep -Milliseconds 100
    if ((Get-CardWindow $preview.Id) -ne [IntPtr]::Zero) { throw 'A tray click after focus loss reopened the card.' }
    $preview.Refresh()
    $cpuStart = $preview.TotalProcessorTime.TotalMilliseconds
    Start-Sleep -Milliseconds 1500
    $preview.Refresh()
    Write-Host "Hidden idle (1.5s): $([int]($preview.TotalProcessorTime.TotalMilliseconds-$cpuStart)) ms CPU; $([math]::Round($preview.WorkingSet64/1MB,1)) MiB working set."
    Write-Host 'GPUI smoke passed: live states keep one window; Escape, tray toggle, close button and light-dismiss work.'
} finally { Stop-Preview $preview }

if ($CheckThemePersistence) {
    # Exercise production persistence in a private APPDATA, never the owner's settings.
    $privateAppData = Join-Path $RepoRoot ('target\verification\theme-settings\' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $privateAppData -Force | Out-Null
    $settingsFile = Join-Path $privateAppData 'NinjutsoBatteryTray\settings.txt'
    $themeCaptureRoot = Join-Path $RepoRoot 'target\verification\theme-switch'
    New-Item -ItemType Directory -Path $themeCaptureRoot -Force | Out-Null
    $systemPreference = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name AppsUseLightTheme -ErrorAction SilentlyContinue
    $autoColor = if ($null -ne $systemPreference -and $systemPreference.AppsUseLightTheme -eq 0) { '191c21' } else { 'fafbfd' }

    function Start-ThemeTestApp {
        $info = [Diagnostics.ProcessStartInfo]::new((Join-Path $RepoRoot 'target\debug\ninjutso-battery-tray.exe'))
        $info.UseShellExecute = $false
        $info.CreateNoWindow = $true
        $info.WorkingDirectory = $RepoRoot
        $info.Environment['APPDATA'] = $privateAppData
        return [Diagnostics.Process]::Start($info)
    }
    function Assert-ThemeFrame([IntPtr]$Window, [string]$Expected, [string]$Name) {
        $path = Join-Path $themeCaptureRoot "theme.$Name.png"
        for ($retry=0; $retry -lt 12; $retry++) {
            Save-Card $Window $path
            $bitmap = [Drawing.Bitmap]::new($path)
            try { $pixel = $bitmap.GetPixel(8,40); $actual = '{0:x2}{1:x2}{2:x2}' -f $pixel.R,$pixel.G,$pixel.B }
            finally { $bitmap.Dispose() }
            if ($actual -eq $Expected) { return }
        }
        throw "Theme '$Name' rendered $actual instead of $Expected."
    }
    function Set-TestTheme([IntPtr]$HostWindow, [string]$Code, [int]$Command) {
        [void][GpuiSmoke]::PostMessageW($HostWindow, 0x111, [IntPtr]$Command, [IntPtr]::Zero)
        for ($retry=0; $retry -lt 30; $retry++) {
            Start-Sleep -Milliseconds 50
            if ((Test-Path -LiteralPath $settingsFile) -and
                (Get-Content -Raw -LiteralPath $settingsFile) -match "(?m)^theme=$Code\r?$" ) { return }
        }
        throw "Theme preference was not saved: $Code"
    }

    $themeApp = Start-ThemeTestApp
    try {
        $card = Wait-Card $themeApp
        $hostWindow = Get-HostWindow $themeApp.Id
        [void][GpuiSmoke]::PostMessageW($hostWindow, 0x111, [IntPtr]1002, [IntPtr]::Zero)
        foreach ($choice in @(@('dark',1202,'191c21'), @('light',1201,'fafbfd'), @('auto',1200,$autoColor), @('ninjaforce',1203,'c3bddb'))) {
            Set-TestTheme $hostWindow $choice[0] $choice[1]
            Assert-ThemeFrame $card $choice[2] $choice[0]
            if ((Get-CardWindow $themeApp.Id) -ne $card) { throw 'Theme selection replaced the existing card.' }
            if ((Get-Content -Raw -LiteralPath $settingsFile) -notmatch '(?m)^large_icon_percent=false\r?$') {
                throw 'Theme selection overwrote another preference.'
            }
        }
        Set-TestTheme $hostWindow 'ninjaforce' 1203
        Assert-ThemeFrame $card 'c3bddb' 'ninjaforce-before-restart'
    } finally { Stop-Preview $themeApp }

    $themeApp = Start-ThemeTestApp
    try {
        $card = Wait-Card $themeApp
        Assert-ThemeFrame $card 'c3bddb' 'ninjaforce-after-restart'
        Write-Host 'Theme smoke passed: Auto/Light/Dark/NinjaForce redraw one window, preserve other settings, and NinjaForce persists after restart; owner APPDATA unchanged.'
    } finally { Stop-Preview $themeApp }
}
