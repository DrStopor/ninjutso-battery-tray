[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [ValidateSet('en', 'ru', 'be', 'kk', 'zh-CN')][string[]]$Languages = @('en', 'ru', 'be', 'kk', 'zh-CN'),
    [string]$CaptureDirectory = 'target\verification\context-menu'
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'build-tools.ps1')
Initialize-ProjectToolchain -RepoRoot $RepoRoot
if (-not $SkipBuild) {
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
public static class TrayMenuSmoke {
    public delegate bool Callback(IntPtr window, IntPtr parameter);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MenuBar {
        public uint Size; public Rect Rect; public IntPtr Menu, MenuWindow; public uint Focus;
    }
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr parameter);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool GetMenuBarInfo(IntPtr window, int objectId, int itemId, ref MenuBar info);
    [DllImport("user32.dll")] public static extern uint GetMenuState(IntPtr menu, uint item, uint flags);
    [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll")] public static extern IntPtr GetSubMenu(IntPtr menu, int item);
    [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process, uint flag);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetMenuStringW(IntPtr menu, uint item, StringBuilder text, int count, uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr window, StringBuilder text, int count);
    public static string Class(IntPtr window) { var s=new StringBuilder(512); GetClassNameW(window,s,s.Capacity); return s.ToString(); }
    public static string Label(IntPtr menu, uint item) { var s=new StringBuilder(512); GetMenuStringW(menu,item,s,s.Capacity,0x400); return s.ToString(); }
    public static IntPtr Menu(IntPtr window) {
        var info=new MenuBar { Size=(uint)Marshal.SizeOf<MenuBar>() };
        if (!GetMenuBarInfo(window,-4,0,ref info)) throw new InvalidOperationException("GetMenuBarInfo failed");
        return info.Menu;
    }
    public static IntPtr[] Windows(int processId) {
        var windows=new List<IntPtr>();
        EnumWindows((window,parameter) => {
            uint owner; GetWindowThreadProcessId(window,out owner);
            if (owner==processId) windows.Add(window);
            return true;
        },IntPtr.Zero);
        return windows.ToArray();
    }
}
'@

function Find-TrayHost([Diagnostics.Process]$Process) {
    for ($retry=0; $retry -lt 100; $retry++) {
        if ($Process.HasExited) { throw "Preview exited: $($Process.ExitCode)" }
        foreach ($window in [TrayMenuSmoke]::Windows($Process.Id)) {
            if ([TrayMenuSmoke]::Class($window) -eq 'NinjutsoBatteryTray.GpuiHost.v1') { return $window }
        }
        Start-Sleep -Milliseconds 50
    }
    throw 'Tray host did not start.'
}
function Get-Popups([int]$OwnerPid) {
    @([TrayMenuSmoke]::Windows($OwnerPid) | Where-Object {
        [TrayMenuSmoke]::Class($_) -eq '#32768' -and [TrayMenuSmoke]::IsWindowVisible($_)
    })
}
function Wait-Popups([int]$OwnerPid, [int]$Count) {
    for ($retry=0; $retry -lt 60; $retry++) {
        $windows = @(Get-Popups $OwnerPid)
        if ($windows.Count -eq $Count) { return $windows }
        Start-Sleep -Milliseconds 50
    }
    throw "Expected $Count menu windows, got $($windows.Count)."
}
function Send-MenuKey([IntPtr]$HostWindow, [int]$Key) {
    [void][TrayMenuSmoke]::PostMessageW($HostWindow, 0x100, [IntPtr]$Key, [IntPtr]1)
    [void][TrayMenuSmoke]::PostMessageW($HostWindow, 0x101, [IntPtr]$Key, [IntPtr]0xc0000001L)
    Start-Sleep -Milliseconds 60
}
function Open-TrayMenu([IntPtr]$HostWindow, [int]$OwnerPid) {
    [void][TrayMenuSmoke]::PostMessageW($HostWindow, 0x8001, [IntPtr]::Zero, [IntPtr]0x1007b)
    $windows = @(Wait-Popups $OwnerPid 1)
    return $windows[0]
}
function Assert-MenuFrame([IntPtr]$Window, [string]$Background, [string]$Name) {
    $path = Join-Path $CaptureRoot "$Name.png"
    for ($retry=0; $retry -lt 10; $retry++) {
        Start-Sleep -Milliseconds 100
        if (-not [TrayMenuSmoke]::IsWindowVisible($Window)) { throw 'Menu dismissed during capture.' }
        $rect = [TrayMenuSmoke+Rect]::new()
        [void][TrayMenuSmoke]::GetWindowRect($Window, [ref]$rect)
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
        try {
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try { $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size) } finally { $graphics.Dispose() }
            $backgroundPixels=0
            $colors=[Collections.Generic.HashSet[int]]::new()
            for ($y=8; $y -lt $bitmap.Height-8; $y+=3) {
                for ($x=8; $x -lt $bitmap.Width-8; $x+=3) {
                    $pixel=$bitmap.GetPixel($x,$y)
                    $hex='{0:x2}{1:x2}{2:x2}' -f $pixel.R,$pixel.G,$pixel.B
                    if ($hex -eq $Background) { $backgroundPixels++ }
                    [void]$colors.Add($pixel.ToArgb())
                }
            }
            if ($backgroundPixels -gt 100 -and $colors.Count -gt 5) {
                $bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)
                return
            }
        } finally { $bitmap.Dispose() }
    }
    throw "Menu '$Name' did not render $Background or its text (background=$backgroundPixels, colors=$($colors.Count))."
}

$systemPreference=Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name AppsUseLightTheme -ErrorAction SilentlyContinue
$autoColor=if ($null -ne $systemPreference -and $systemPreference.AppsUseLightTheme -eq 0) { '191c21' } else { 'fafbfd' }
foreach ($language in $Languages) {
    $info=[Diagnostics.ProcessStartInfo]::new((Join-Path $RepoRoot 'target\debug\ninjutso-battery-tray.exe'))
    $info.UseShellExecute=$false
    $info.CreateNoWindow=$true
    $info.WorkingDirectory=$RepoRoot
    foreach ($arg in @('--preview',"--preview-language=$language",'--preview-theme=light','--preview-state=ready')) { $info.ArgumentList.Add($arg) }
    # Preview settings are never saved. Mock battery data avoids touching HID.
    $preview=[Diagnostics.Process]::Start($info)
    try {
        $hostWindow=Find-TrayHost $preview
        Start-Sleep -Milliseconds 500
        foreach ($choice in @(@('light',1201,'fafbfd'),@('dark',1202,'191c21'),@('auto',1200,$autoColor),@('ninjaforce',1203,'c3bddb'))) {
            [void][TrayMenuSmoke]::PostMessageW($hostWindow,0x111,[IntPtr]$choice[1],[IntPtr]::Zero)
            Start-Sleep -Milliseconds 120
            $menuWindow=Open-TrayMenu $hostWindow $preview.Id
            Assert-MenuFrame $menuWindow $choice[2] "$language.$($choice[0]).root"
            $rootMenu=[TrayMenuSmoke]::Menu($menuWindow)
            if ([TrayMenuSmoke]::GetMenuItemCount($rootMenu) -ne 6) { throw 'Menu structure changed.' }
            for ($index=0; $index -lt 6; $index++) {
                if ($index -ne 4 -and [string]::IsNullOrWhiteSpace([TrayMenuSmoke]::Label($rootMenu,$index))) {
                    throw 'A native/MSAA menu label is missing.'
                }
            }
            $themeMenu=[TrayMenuSmoke]::GetSubMenu($rootMenu,3)
            if ([TrayMenuSmoke]::GetMenuItemCount($themeMenu) -ne 4) { throw 'Theme choices are missing.' }
            if ([TrayMenuSmoke]::Label($themeMenu,3) -ne 'NinjaForce') { throw 'NinjaForce label is missing.' }
            if (([TrayMenuSmoke]::GetMenuState($themeMenu,$choice[1],0) -band 8) -eq 0) { throw 'Selected theme has no radio mark.' }
            # Native TrackPopupMenu does not interpret Home/End consistently.
            Send-MenuKey $hostWindow 0x28 # Down selects the first item
            Send-MenuKey $hostWindow 0x28
            Send-MenuKey $hostWindow 0x28
            Send-MenuKey $hostWindow 0x28
            Send-MenuKey $hostWindow 0x27 # Right opens Theme
            $windows=@(Wait-Popups $preview.Id 2)
            $submenu=@($windows | Where-Object { $_ -ne $menuWindow })[0]
            if ([TrayMenuSmoke]::GetMenuItemCount([TrayMenuSmoke]::Menu($submenu)) -ne 4) { throw 'Keyboard opened the wrong submenu.' }
            Assert-MenuFrame $submenu $choice[2] "$language.$($choice[0]).themes"
            Send-MenuKey $hostWindow 0x1b # Escape closes only the submenu
            $null=Wait-Popups $preview.Id 1
            Send-MenuKey $hostWindow 0x26 # Up selects Language
            Send-MenuKey $hostWindow 0x27
            $windows=@(Wait-Popups $preview.Id 2)
            $submenu=@($windows | Where-Object { $_ -ne $menuWindow })[0]
            Assert-MenuFrame $submenu $choice[2] "$language.$($choice[0]).languages"
            Send-MenuKey $hostWindow 0x1b
            Send-MenuKey $hostWindow 0x1b
            $null=Wait-Popups $preview.Id 0
        }
        # Choose NinjaForce through the real submenu, not a synthetic WM_COMMAND.
        $menuWindow=Open-TrayMenu $hostWindow $preview.Id
        1..4 | ForEach-Object { Send-MenuKey $hostWindow 0x28 }
        Send-MenuKey $hostWindow 0x27
        $null=Wait-Popups $preview.Id 2
        Send-MenuKey $hostWindow 0x28 # Auto -> Light
        Send-MenuKey $hostWindow 0x28 # Light -> Dark
        Send-MenuKey $hostWindow 0x28 # Dark -> NinjaForce
        Send-MenuKey $hostWindow 13
        $null=Wait-Popups $preview.Id 0
        $menuWindow=Open-TrayMenu $hostWindow $preview.Id
        Assert-MenuFrame $menuWindow 'c3bddb' "$language.ninjaforce-selected"
        $themeMenu=[TrayMenuSmoke]::GetSubMenu([TrayMenuSmoke]::Menu($menuWindow),3)
        if (([TrayMenuSmoke]::GetMenuState($themeMenu,1203,0) -band 8) -eq 0) { throw 'Keyboard selection did not apply NinjaForce.' }
        Send-MenuKey $hostWindow 0x1b
        $null=Wait-Popups $preview.Id 0
        $resourcesBefore=[TrayMenuSmoke]::GetGuiResources($preview.Handle,0)
        1..12 | ForEach-Object {
            $null=Open-TrayMenu $hostWindow $preview.Id
            Send-MenuKey $hostWindow 0x1b
            $null=Wait-Popups $preview.Id 0
        }
        $resourcesAfter=[TrayMenuSmoke]::GetGuiResources($preview.Handle,0)
        if ($resourcesAfter -gt $resourcesBefore+2) { throw "Menu leaked GDI handles: $resourcesBefore -> $resourcesAfter" }
        # Also verify graceful shutdown from inside the native modal menu loop.
        $null=Open-TrayMenu $hostWindow $preview.Id
        Write-Host "Menu passed ($language): Auto/Light/Dark/NinjaForce, both submenus, labels/radio state, keyboard selection, GDI handles $resourcesBefore -> $resourcesAfter."
    } finally {
        if (-not $preview.HasExited) {
            $hostWindow=Find-TrayHost $preview
            [void][TrayMenuSmoke]::PostMessageW($hostWindow,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
            if (-not $preview.WaitForExit(5000)) { $preview.Kill(); throw 'Preview failed to shut down with an open menu.' }
        }
        if ($preview.ExitCode -ne 0) { throw "Menu preview exit code $($preview.ExitCode)." }
        $preview.Dispose()
    }
}
