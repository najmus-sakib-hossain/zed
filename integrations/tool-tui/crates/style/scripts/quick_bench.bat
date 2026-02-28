@echo off
setlocal enabledelayedexpansion

echo Building in release mode...
cargo build --release 2>&1 | tail -5

echo.
echo Running quick benchmark...
echo ========================================

REM Create test HTML
set TEMP_HTML=%TEMP%\dx_perf_test.html
echo ^<!DOCTYPE html^> > "%TEMP_HTML%"
echo ^<html^> >> "%TEMP_HTML%"
echo ^<head^>^<title^>Performance Test^</title^>^</head^> >> "%TEMP_HTML%"
echo ^<body^> >> "%TEMP_HTML%"
echo   ^<div class="container mx-auto px-4 py-8"^> >> "%TEMP_HTML%"
echo     ^<h1 class="text-3xl font-bold text-gray-900 mb-6"^>Performance Test Page^</h1^> >> "%TEMP_HTML%"

REM Add elements (simplified for batch)
for /L %%i in (1,1,500) do (
    set /a mod10=%%i %% 10
    set /a mod8=%%i %% 8 + 1
    set /a mod6=%%i %% 6 + 1
    set /a mod5=%%i %% 5 * 100 + 100
    echo     ^<div class="flex items-center bg-blue-!mod10!00 p-!mod8! text-!mod6!xl"^>Element %%i^</div^> >> "%TEMP_HTML%"
)

echo   ^</div^> >> "%TEMP_HTML%"
echo ^</body^> >> "%TEMP_HTML%"
echo ^</html^> >> "%TEMP_HTML%"

echo Test HTML created
echo.

REM Run the benchmark using cargo test with output
echo Creating Rust test program...

set TEST_FILE=%TEMP%\perf_test.rs
(
echo use std::time::Instant;
echo use std::fs;
echo.
echo fn main^(^) {
echo     let html = fs::read_to_string^(std::env::args^(^).nth^(1^).expect^("Need HTML file path"^)^).unwrap^(^);
echo     let html_bytes = html.as_bytes^(^);
echo.    
echo     println!^("HTML size: {} bytes", html_bytes.len^(^)^);
echo     println!^("Running 50 iterations...\n"^);
echo.    
echo     // Warm up
echo     for _ in 0..5 {
echo         let _ = style::parser::extract_classes_fast^(html_bytes, 128^);
echo     }
echo.    
echo     // Benchmark
echo     let mut total = std::time::Duration::ZERO;
echo     for _ in 0..50 {
echo         let start = Instant::now^(^);
echo         let result = style::parser::extract_classes_fast^(html_bytes, 128^);
echo         total += start.elapsed^(^);
echo         std::hint::black_box^(result^);
echo     }
echo.    
echo     let avg = total / 50;
echo     println!^("Average time per iteration: {:?}", avg^);
echo     println!^("Classes per second: {:.0}", 50.0 / total.as_secs_f64^(^)^);
echo.    
echo     let result = style::parser::extract_classes_fast^(html_bytes, 128^);
echo     println!^("\nExtracted {} unique classes", result.classes.len^(^)^);
echo }
) > "%TEST_FILE%"

echo Compiling test program...
rustc --edition 2021 -L target\release\deps -L target\release --extern style=target\release\style.rlib "%TEST_FILE%" -o "%TEMP%\perf_test.exe" -C opt-level=3 2>&1

if exist "%TEMP%\perf_test.exe" (
    echo.
    echo Running performance test...
    echo ========================================
    "%TEMP%\perf_test.exe" "%TEMP_HTML%"
    echo ========================================
) else (
    echo Failed to compile test program. Running criterion benchmarks instead...
    cargo bench --bench style_benchmark -- html_parsing/small --quick
)

REM Cleanup
del "%TEMP_HTML%" 2>nul
del "%TEST_FILE%" 2>nul
del "%TEMP%\perf_test.exe" 2>nul
del "%TEMP%\perf_test.pdb" 2>nul

echo.
echo Done! For full benchmarks, run: cargo bench
pause
