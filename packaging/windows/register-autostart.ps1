# Registers olydraw to launch hidden (tray only) at login for the current
# user. Run once from the folder containing olydraw.exe:
#   powershell -ExecutionPolicy Bypass -File register-autostart.ps1
# To undo: Remove-ItemProperty -Path HKCU:\Software\Microsoft\Windows\CurrentVersion\Run -Name olydraw

$exe = Join-Path $PSScriptRoot "olydraw.exe"
if (-not (Test-Path $exe)) {
    Write-Error "olydraw.exe not found next to this script"
    exit 1
}

Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
    -Name "olydraw" -Value "`"$exe`" --hidden"

Write-Host "olydraw will now start hidden at login."
