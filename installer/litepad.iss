; Inno Setup script for LitePad — builds LitePad-Setup.exe
; Compile with:  ISCC.exe installer\litepad.iss   (run from the repo root)
;
; This installs LitePad and, crucially, creates a Start Menu shortcut — that
; shortcut is what Windows Search / the Start menu index, so LitePad shows up
; when you type "LitePad". It also registers an uninstaller (Apps & features).

#define AppName "LitePad"
#define AppVersion "0.1.0"
#define AppPublisher "Yash Pandey"
#define AppExeName "litepad.exe"
#define AppURL "https://github.com/yashpandey0031/litepad"

[Setup]
; A stable, unique ID so upgrades replace the same install (keep this constant).
AppId={{B7E6D4A2-3F91-4C7E-A5D8-2E9F1C0B4A63}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}/releases

; Per-user install: no admin/UAC prompt, but the Start Menu shortcut is still
; indexed by Windows Search. Users can elevate to install for all users if they want.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExeName}
UninstallDisplayName={#AppName}
LicenseFile=..\LICENSE

OutputDir=..\dist
OutputBaseFilename=LitePad-Setup
SetupIconFile=..\assets\icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; Checked by default, so a desktop shortcut is created (users can opt out).
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Files]
Source: "..\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu entry -> makes LitePad findable in Windows Search.
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
