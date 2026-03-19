@echo off
setlocal

cargo build --release
if errorlevel 1 exit /b 1

echo Build complete: target\release\imazer.exe
