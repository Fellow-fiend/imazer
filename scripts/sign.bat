@echo off
setlocal

if "%~1"=="" (
  echo Usage: sign.bat ^<path-to-exe^> [path-to-pfx]
  exit /b 1
)

set "APP_EXE=%~1"
set "CERT_PATH=%~2"

if "%CERT_PATH%"=="" set "CERT_PATH=certificate.pfx"
if "%SIGN_PWD%"=="" (
  echo Please set SIGN_PWD environment variable before running.
  exit /b 1
)

signtool sign /f "%CERT_PATH%" /p "%SIGN_PWD%" /tr http://timestamp.digicert.com /td sha256 /fd sha256 "%APP_EXE%"
