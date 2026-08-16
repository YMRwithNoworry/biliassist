#define MyAppVersion GetEnv("VERSION")

[Setup]
AppId={{A63C2632-1F3C-4DFD-9EF2-9A10C70E2EA2}
AppName=BiliAssist
AppVersion={#MyAppVersion}
AppPublisher=YMRwithNoworry
AppPublisherURL=https://github.com/YMRwithNoworry/biliassist
AppSupportURL=https://github.com/YMRwithNoworry/biliassist/issues
DefaultDirName={localappdata}\Programs\BiliAssist
DefaultGroupName=BiliAssist
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputDir=..\..\dist
OutputBaseFilename=biliassist-{#MyAppVersion}-windows-x86_64-setup
SetupIconFile=..\..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\BiliAssist.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
VersionInfoVersion={#MyAppVersion}

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务："; Flags: unchecked

[Files]
Source: "..\..\dist\BiliAssist\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\BiliAssist"; Filename: "{app}\BiliAssist.exe"
Name: "{autodesktop}\BiliAssist"; Filename: "{app}\BiliAssist.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\BiliAssist.exe"; Description: "启动 BiliAssist"; Flags: nowait postinstall skipifsilent
