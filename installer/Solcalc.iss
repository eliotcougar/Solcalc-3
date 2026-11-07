; Compile from build-installer.ps1. Bundled catalogs live inside Solcalc.exe.
#ifndef AppVersion
  #error AppVersion must be supplied by the build script
#endif
#ifndef AppExe
  #define AppExe SourcePath + "..\dist\Solcalc.exe"
#endif
#ifndef OutputPath
  #define OutputPath SourcePath + "..\dist"
#endif

[Setup]
AppId={{E9341401-CA59-4BF4-9CDB-E183C883CA31}
AppName=Solcalc
AppVersion={#AppVersion}
AppPublisher=Solcalc
AppComments=Solution Calculator
DefaultDirName={autopf}\Solcalc
DefaultGroupName=Solcalc
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UsePreviousPrivileges=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputPath}
OutputBaseFilename=Solcalc-{#AppVersion}-x64-setup
UninstallDisplayIcon={app}\Solcalc.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
VersionInfoVersion={#AppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#AppExe}"; DestDir: "{app}"; DestName: "Solcalc.exe"; Flags: ignoreversion
Source: "..\assets\Solcalc-Readme.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; DestName: "Solcalc-Third-Party-Notices.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Solcalc"; Filename: "{app}\Solcalc.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\Solcalc"; Filename: "{app}\Solcalc.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\Solcalc.exe"; Description: "{cm:LaunchProgram,Solcalc}"; Flags: nowait postinstall skipifsilent runasoriginaluser

; No user database, settings, or migration backups are installed or removed.
; Inno's uninstall log removes only files and shortcuts installed by this setup.
