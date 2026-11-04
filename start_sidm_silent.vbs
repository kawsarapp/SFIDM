' ===================================================
' SIDM - Silent Background Launcher (Zero CMD Window)
' ===================================================
Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
ScriptDir = fso.GetParentFolderName(WScript.ScriptFullName)

WshShell.CurrentDirectory = ScriptDir
WshShell.Run "cmd.exe /c start_sidm.bat", 0, False
