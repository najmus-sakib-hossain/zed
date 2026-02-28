@echo off
echo Stopping any running Forge processes...
taskkill /F /IM forge-cli.exe >nul 2>&1
if %ERRORLEVEL% == 0 (
    echo Forge process stopped.
    timeout /t 2 /nobreak >nul
) else (
    echo No running Forge process found.
)

echo.
echo Building Forge binary...
cargo build --release

if %ERRORLEVEL% == 0 (
    echo.
    echo ✅ Build successful!
    echo Binary location: target\release\forge-cli.exe
) else (
    echo.
    echo ❌ Build failed!
    exit /b 1
)
