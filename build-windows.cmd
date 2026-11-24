@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0packaging\scripts\build-windows.ps1"
if errorlevel 1 (
 echo Build failed. See the error above.
 pause
 exit /b 1
)
pause
