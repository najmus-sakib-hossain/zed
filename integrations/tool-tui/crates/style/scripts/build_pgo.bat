@echo off
REM Profile-Guided Optimization (PGO) build script for dx-style (Windows)
REM This script builds dx-style with maximum performance using PGO

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
set "PGO_DATA_DIR=%PROJECT_ROOT%\target\pgo-data"
set "PROFDATA_FILE=%PGO_DATA_DIR%\merged.profdata"

echo.
echo =============================================================================
echo   Building dx-style with Profile-Guided Optimization (PGO)
echo =============================================================================
echo.

cd /d "%PROJECT_ROOT%"

REM Clean previous PGO data
if exist "%PGO_DATA_DIR%" (
    echo Cleaning previous PGO data...
    rmdir /s /q "%PGO_DATA_DIR%" 2>nul
)
mkdir "%PGO_DATA_DIR%"

REM Step 1: Build with instrumentation
echo.
echo [Step 1] Building with profiling instrumentation...
echo.
set "RUSTFLAGS=-Cprofile-generate=%PGO_DATA_DIR%"
cargo build --release --profile release-pgo
if errorlevel 1 (
    echo ERROR: Instrumented build failed
    exit /b 1
)
echo.
echo [OK] Instrumented build complete
echo.

REM Step 2: Run workloads to collect profile data
echo [Step 2] Collecting profile data from representative workloads...
echo.

REM Create test HTML files for profiling
if not exist "%PROJECT_ROOT%\playgrounds\pgo-test" mkdir "%PROJECT_ROOT%\playgrounds\pgo-test"

REM Small HTML
(
echo ^<!DOCTYPE html^>
echo ^<html^>
echo ^<head^>^<title^>Small Test^</title^>^</head^>
echo ^<body^>
echo     ^<div class="flex items-center justify-between bg-blue-500 p-4 rounded-lg shadow-xl"^>
echo         ^<h1 class="text-2xl font-bold text-white"^>Header^</h1^>
echo         ^<button class="bg-white text-blue-600 px-4 py-2 rounded hover:bg-gray-100"^>Click^</button^>
echo     ^</div^>
echo ^</body^>
echo ^</html^>
) > "%PROJECT_ROOT%\playgrounds\pgo-test\small.html"

REM Medium HTML - Start
(
echo ^<!DOCTYPE html^>
echo ^<html^>
echo ^<head^>^<title^>Medium Test^</title^>^</head^>
echo ^<body^>
echo     ^<div class="container mx-auto px-4 py-8"^>
echo         ^<header class="flex items-center justify-between mb-8"^>
echo             ^<h1 class="text-4xl font-bold text-gray-900"^>Dashboard^</h1^>
echo             ^<nav class="flex gap-4"^>
echo                 ^<a class="text-blue-600 hover:text-blue-800 font-medium"^>Home^</a^>
echo                 ^<a class="text-blue-600 hover:text-blue-800 font-medium"^>About^</a^>
echo                 ^<a class="text-blue-600 hover:text-blue-800 font-medium"^>Contact^</a^>
echo             ^</nav^>
echo         ^</header^>
echo         ^<main class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6"^>
) > "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"

REM Add cards to medium HTML
for /l %%i in (1,1,50) do (
    echo             ^<div class="bg-white p-6 rounded-lg shadow-md hover:shadow-xl transition-shadow"^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"
    echo                 ^<h2 class="text-xl font-semibold text-gray-800 mb-2"^>Card %%i^</h2^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"
    echo                 ^<p class="text-gray-600 mb-4"^>Description for card %%i with some content.^</p^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"
    echo                 ^<button class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600 transition-colors"^>Action^</button^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"
    echo             ^</div^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"
)

REM Medium HTML - End
(
echo         ^</main^>
echo     ^</div^>
echo ^</body^>
echo ^</html^>
) >> "%PROJECT_ROOT%\playgrounds\pgo-test\medium.html"

REM Large HTML - Start
(
echo ^<!DOCTYPE html^>
echo ^<html^>
echo ^<head^>^<title^>Large Test^</title^>^</head^>
echo ^<body class="bg-gray-50"^>
echo     ^<div class="min-h-screen"^>
) > "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"

REM Add elements to large HTML
for /l %%i in (1,1,100) do (
    set /a "mod9=%%i %% 9 + 1"
    set /a "mod8=%%i %% 8 + 1"
    set /a "mod6=%%i %% 6 + 1"
    set /a "mod4=%%i %% 4 + 1"
    set /a "mod5=%%i %% 5"
    set /a "duration=!mod5! * 100 + 100"
    echo         ^<div class="flex items-center justify-between bg-gradient-to-r from-blue-!mod9!00 to-purple-!mod9!00 p-!mod8! rounded-lg shadow-xl hover:shadow-2xl transition-all duration-!duration! m-!mod6!"^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"
    echo             ^<h3 class="text-!mod4!xl font-bold text-white"^>Element %%i^</h3^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"
    echo             ^<span class="text-sm text-gray-!mod9!00"^>Info^</span^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"
    echo         ^</div^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"
)

REM Large HTML - End
(
echo     ^</div^>
echo ^</body^>
echo ^</html^>
) >> "%PROJECT_ROOT%\playgrounds\pgo-test\large.html"

REM Grouping HTML
(
echo ^<!DOCTYPE html^>
echo ^<html^>
echo ^<head^>^<title^>Grouping Test^</title^>^</head^>
echo ^<body^>
) > "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"

for /l %%i in (1,1,30) do (
    set /a "mod4=%%i %% 4 + 1"
    echo     ^<div class="card(bg-white p-4 rounded shadow hover:shadow-lg transition-all) m-!mod4!"^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
    echo         ^<div class="header(flex items-center justify-between mb-2)"^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
    echo             ^<h3 class="title(text-lg font-bold text-gray-800)"^>Card %%i^</h3^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
    echo         ^</div^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
    echo         ^<p class="content(text-gray-600 text-sm)"^>Content here^</p^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
    echo     ^</div^> >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"
)

(
echo ^</body^>
echo ^</html^>
) >> "%PROJECT_ROOT%\playgrounds\pgo-test\grouping.html"

REM Create PGO config
(
echo [paths]
echo html_dir = "playgrounds/pgo-test"
echo index_file = "playgrounds/pgo-test/medium.html"
echo css_file = "playgrounds/pgo-test/output.css"
echo.
echo [watch]
echo debounce_ms = 10
) > "%PROJECT_ROOT%\.dx\config.pgo.toml"

REM Run workloads
echo   Running workload: small HTML...
set "DX_CONFIG=%PROJECT_ROOT%\.dx\config.pgo.toml"
timeout /t 2 /nobreak >nul 2>&1
start /wait /b "" "%PROJECT_ROOT%\target\release-pgo\style.exe" 2>nul
timeout /t 1 /nobreak >nul

echo   Running workload: medium HTML...
powershell -Command "(Get-Content '%PROJECT_ROOT%\.dx\config.pgo.toml') -replace 'small\.html', 'medium.html' | Set-Content '%PROJECT_ROOT%\.dx\config.pgo.toml'" 2>nul
timeout /t 2 /nobreak >nul 2>&1
start /wait /b "" "%PROJECT_ROOT%\target\release-pgo\style.exe" 2>nul
timeout /t 1 /nobreak >nul

echo   Running workload: large HTML...
powershell -Command "(Get-Content '%PROJECT_ROOT%\.dx\config.pgo.toml') -replace 'medium\.html', 'large.html' | Set-Content '%PROJECT_ROOT%\.dx\config.pgo.toml'" 2>nul
timeout /t 2 /nobreak >nul 2>&1
start /wait /b "" "%PROJECT_ROOT%\target\release-pgo\style.exe" 2>nul
timeout /t 1 /nobreak >nul

echo   Running workload: grouping HTML...
powershell -Command "(Get-Content '%PROJECT_ROOT%\.dx\config.pgo.toml') -replace 'large\.html', 'grouping.html' | Set-Content '%PROJECT_ROOT%\.dx\config.pgo.toml'" 2>nul
timeout /t 2 /nobreak >nul 2>&1
start /wait /b "" "%PROJECT_ROOT%\target\release-pgo\style.exe" 2>nul
timeout /t 1 /nobreak >nul

echo.
echo [OK] Profile data collected
echo.

REM Step 3: Merge profile data
echo [Step 3] Merging profile data...
echo.

REM Find llvm-profdata
set "LLVM_PROFDATA="
where llvm-profdata >nul 2>&1
if !errorlevel! equ 0 (
    set "LLVM_PROFDATA=llvm-profdata"
) else (
    REM Try rustup llvm-tools
    for /f "delims=" %%i in ('rustc --print sysroot 2^>nul') do set "RUST_SYSROOT=%%i"
    if exist "!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe" (
        set "LLVM_PROFDATA=!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe"
    ) else if exist "!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-gnu\bin\llvm-profdata.exe" (
        set "LLVM_PROFDATA=!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-gnu\bin\llvm-profdata.exe"
    )
)

if "!LLVM_PROFDATA!"=="" (
    echo Warning: llvm-profdata not found. Installing llvm-tools...
    rustup component add llvm-tools-preview
    for /f "delims=" %%i in ('rustc --print sysroot 2^>nul') do set "RUST_SYSROOT=%%i"
    if exist "!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe" (
        set "LLVM_PROFDATA=!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe"
    ) else if exist "!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-gnu\bin\llvm-profdata.exe" (
        set "LLVM_PROFDATA=!RUST_SYSROOT!\lib\rustlib\x86_64-pc-windows-gnu\bin\llvm-profdata.exe"
    )
)

if not "!LLVM_PROFDATA!"=="" (
    "!LLVM_PROFDATA!" merge -o "%PROFDATA_FILE%" "%PGO_DATA_DIR%\default_*.profraw"
    echo [OK] Profile data merged
) else (
    echo Warning: Could not find llvm-profdata. Continuing without profile merge...
)
echo.

REM Step 4: Build with PGO
echo [Step 4] Building optimized binary with PGO...
echo.

if exist "%PROFDATA_FILE%" (
    set "RUSTFLAGS=-Cprofile-use=%PROFDATA_FILE% -Cllvm-args=-pgo-warn-missing-function"
    cargo build --release
    if errorlevel 1 (
        echo ERROR: PGO build failed
        exit /b 1
    )
    echo.
    echo [OK] PGO-optimized build complete
) else (
    echo Profile data not available, building without PGO...
    cargo build --release
    if errorlevel 1 (
        echo ERROR: Release build failed
        exit /b 1
    )
)
echo.

REM Cleanup
echo Cleaning up...
if exist "%PROJECT_ROOT%\playgrounds\pgo-test" rmdir /s /q "%PROJECT_ROOT%\playgrounds\pgo-test"
if exist "%PROJECT_ROOT%\.dx\config.pgo.toml" del /q "%PROJECT_ROOT%\.dx\config.pgo.toml"

echo.
echo =============================================================================
echo   Build Complete!
echo =============================================================================
echo.
echo Optimized binary: %PROJECT_ROOT%\target\release\style.exe
echo.
echo Performance improvements from PGO typically range from 10-20%%
echo Run benchmarks to verify: cargo bench
echo.
echo To install: cargo install --path . --locked
echo.

endlocal
