; =====================================================================
; SIDM - Smart Internet Download Manager Professional Windows Setup
; Inno Setup Compiler Script (Produces single SIDM_Setup.exe)
; =====================================================================

#define MyAppName "SIDM"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "SFIDM Inc."
#define MyAppURL "https://github.com/kawsarapp/SFIDM"
#define MyAppExeName "sidm-core.exe"
#define MyExtId "gadhilaplipaklonafepmbegnhoafice"

[Setup]
AppId={{E4FFB10A-53CD-2706-928A-98C7F0022026}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\SIDM
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=SIDM_v1.0.0_Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "Automatically launch SIDM when Windows starts"; GroupDescription: "System Integration:"

[Files]
; Core Daemon Binary
Source: "..\sidm-core\target\release\sidm-core.exe"; DestDir: "{app}"; Flags: ignoreversion
; Browser Integration Extension
Source: "..\sidm-extension\*"; DestDir: "{app}\extension"; Flags: ignoreversion recursesubdirs createallsubdirs
; Bundled yt-dlp Video Downloader (if available)
Source: "..\bin\yt-dlp.exe"; DestDir: "{app}\bin"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "server"
Name: "{group}\SIDM Web Dashboard"; Filename: "http://127.0.0.1:45678"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "server"; Tasks: desktopicon

[Registry]
; 1. Google Chrome Extension Auto-Registration
Root: HKCU; Subkey: "Software\Google\Chrome\Extensions\{#MyExtId}"; ValueType: string; ValueName: "path"; ValueData: "{app}\extension"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Google\Chrome\Extensions\{#MyExtId}"; ValueType: string; ValueName: "version"; ValueData: "{#MyAppVersion}"; Flags: uninsdeletekey

; 2. Microsoft Edge Extension Auto-Registration
Root: HKCU; Subkey: "Software\Microsoft\Edge\Extensions\{#MyExtId}"; ValueType: string; ValueName: "path"; ValueData: "{app}\extension"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Microsoft\Edge\Extensions\{#MyExtId}"; ValueType: string; ValueName: "version"; ValueData: "{#MyAppVersion}"; Flags: uninsdeletekey

; 3. Brave Browser Extension Auto-Registration
Root: HKCU; Subkey: "Software\BraveSoftware\Brave-Browser\Extensions\{#MyExtId}"; ValueType: string; ValueName: "path"; ValueData: "{app}\extension"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\BraveSoftware\Brave-Browser\Extensions\{#MyExtId}"; ValueType: string; ValueName: "version"; ValueData: "{#MyAppVersion}"; Flags: uninsdeletekey

; 4. Windows Boot Startup (Run on Windows boot)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SIDM"; ValueData: """{app}\{#MyAppExeName}"" server"; Tasks: autostart; Flags: uninsdeletevalue

[Run]
; Launch SIDM Server Daemon silently in background
Filename: "{app}\{#MyAppExeName}"; Parameters: "server"; Flags: nowait postinstall skipifsilent
; Open Web Dashboard
Filename: "http://127.0.0.1:45678"; Flags: shellexec nowait postinstall skipifsilent
