#define MyAppName "Ninjutso Battery Tray"
#define MyAppPublisher "drstopor"
#define MyAppExeName "ninjutso-battery-tray.exe"
#define MyAppRunName "NinjutsoBatteryTray"

#if Ver < EncodeVer(6, 7, 4)
#error Inno Setup 6.7.4 or newer is required.
#endif

#ifndef MyAppVersion
#define MyAppVersion GetStringFileInfo("..\target\release\" + MyAppExeName, "ProductVersion")
#if MyAppVersion == ""
#error Build the release executable before compiling the installer.
#endif
#endif

[Setup]
AppId={{B82A9ACD-B942-41F4-9A26-59B18F052686}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\NinjutsoBatteryTray
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=NinjutsoBatteryTraySetup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LanguageDetectionMethod=uilanguage
ShowLanguageDialog=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\{#MyAppExeName}
LicenseFile=..\LICENSE
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} installer
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "belarusian"; MessagesFile: "compiler:Default.isl,languages\Belarusian.isl"
Name: "kazakh"; MessagesFile: "compiler:Default.isl,languages\Kazakh.islu"
Name: "chinesesimplified"; MessagesFile: "compiler:Default.isl,languages\ChineseSimplified.isl"

[LangOptions]
kazakh.LanguageName=Қазақша

[CustomMessages]
kazakh.AutoStartProgramGroupDescription=Автоматты іске қосу:
kazakh.AutoStartProgram=%1 қолданбасын Windows жүйесімен бірге іске қосу
kazakh.LaunchProgram=%1 қолданбасын іске қосу

[Tasks]
Name: "autostart"; Description: "{cm:AutoStartProgram,{#MyAppName}}"; GroupDescription: "{cm:AutoStartProgramGroupDescription}"; Flags: checkedonce

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\THIRD_PARTY_NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"

[Registry]
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppRunName}"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#MyAppRunName}"; Flags: deletevalue; Tasks: not autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{cmd}"; Parameters: "/C taskkill /IM ""{#MyAppExeName}"" /F /T >NUL 2>NUL || exit /B 0"; Flags: runhidden; RunOnceId: "StopApp"
