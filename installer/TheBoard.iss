; Per-user installer for TheBoard.
; Build the UI and the release exe first (installer\build.ps1). This script
; only copies that exe into %LocalAppData%\TheBoard.
; Sounds, the board, and config stay in %APPDATA%\soundboard and are left there
; when the app is uninstalled.

#define MyAppName "TheBoard"
#define MyAppExeName "soundboard-server.exe"
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

[Setup]
AppId={{7C3E9A14-2B58-4F6D-9E01-A8D4C6B21F70}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=TheBoard
VersionInfoVersion={#MyAppVersion}
DefaultDirName={localappdata}\TheBoard
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=TheBoard-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
AppMutex=Local\TheBoard
CloseApplications=yes
RestartApplications=no

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--open"
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--open"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--open"; Description: "Start {#MyAppName}"; Flags: nowait postinstall skipifsilent
