#define MyAppName "Ninjutso Battery Tray"
#define MyAppPublisher "drstopor"
#define MyAppExeName "ninjutso-battery-tray.exe"
#define MyAppRunName "NinjutsoBatteryTray"

#ifndef MyAppVersion
#define MyAppVersion "0.1.0"
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
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
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
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "autostart"; Description: "Запускать вместе с Windows"; GroupDescription: "Автозапуск:"; Flags: checkedonce

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"

[Registry]
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppRunName}"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#MyAppRunName}"; Flags: deletevalue; Tasks: not autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Запустить {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{cmd}"; Parameters: "/C taskkill /IM ""{#MyAppExeName}"" /F /T >NUL 2>NUL || exit /B 0"; Flags: runhidden; RunOnceId: "StopApp"
