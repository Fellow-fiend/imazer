[Setup]
AppId={{A47DE1ED-4E7E-4A1A-9A84-56C80EBD6ACB}
AppName=Imazer
AppVersion=0.1.0
DefaultDirName={autopf}\Imazer
DefaultGroupName=Imazer
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=imazer-setup
Compression=lzma
SolidCompression=yes
UninstallDisplayIcon={app}\imazer.exe
ArchitecturesInstallIn64BitMode=x64
DefaultLanguage=english

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "..\target\release\imazer.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Imazer"; Filename: "{app}\imazer.exe"
Name: "{autodesktop}\Imazer"; Filename: "{app}\imazer.exe"; Tasks: desktopicon

[Registry]
Root: HKCR; Subkey: "*\shell\ResizeImages"; ValueType: string; ValueName: ""; ValueData: "Resize images"; Flags: uninsdeletekey
Root: HKCR; Subkey: "*\shell\ResizeImages\command"; ValueType: string; ValueName: ""; ValueData: """{app}\imazer.exe"" ""%1"""; Flags: uninsdeletekey

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
