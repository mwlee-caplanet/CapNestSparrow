@echo off
REM ── SparrowLib JNI Sample: build & run ──────────────────────────────
REM Prerequisites: CMake build complete (SparrowLib.dll in target\debug\)
REM Usage: run.bat [input.json] [output_dir]

setlocal
cd /d "%~dp0.."

echo === Building Java classes ===
if not exist out mkdir out
javac -encoding UTF-8 -d out java/com/sparrow/*.java java-sample/SampleRunner.java
if %errorlevel% neq 0 (
    echo ERROR: Java compilation failed. Is JDK installed?
    exit /b 1
)

echo.
echo === Running Sample ===
set INPUT=%1
if "%INPUT%"=="" set INPUT=data/input/albano.json
set OUTDIR=%2
if "%OUTDIR%"=="" set OUTDIR=output

java -Djava.library.path=target\debug -cp out com.sparrow.sample.SampleRunner "%INPUT%" "%OUTDIR%"

endlocal
