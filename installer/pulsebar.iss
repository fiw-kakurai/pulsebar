; Inno Setup script for Pulsebar.
; Build: ISCC.exe installer\pulsebar.iss  (after `cargo build --release`)

#define MyAppName "Pulsebar"
; Overridable from the command line (CI): ISCC.exe /DMyAppVersion=x.y.z
#ifndef MyAppVersion
#define MyAppVersion "0.2.0"
#endif
#define MyAppExeName "pulsebar.exe"
#define MyAppURL "https://github.com/fiw-kakurai/pulsebar"

[Setup]
; Do not change AppId between versions; it identifies the app for upgrades.
AppId={{9E1B49D4-9C0A-4A38-BE0C-52C4E7B1A7D5}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
; Per-user install: no admin prompt, files go to %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=Output
OutputBaseFilename=pulsebar-setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Tasks]
Name: "autostart"; Description: "Start {#MyAppName} automatically at sign-in"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"

[Registry]
; Same key/value the in-app "Run at startup" toggle uses (src/menu.rs),
; so the menu checkbox stays in sync with what the installer wrote.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
    ValueType: string; ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"""; \
    Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; \
    Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/f /im {#MyAppExeName}"; \
    Flags: runhidden; RunOnceId: "KillPulsebar"

[Code]
// The app is a borderless overlay with no closable window, so ask the OS to
// terminate any running instance before overwriting the exe.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/f /im {#MyAppExeName}', '',
       SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
