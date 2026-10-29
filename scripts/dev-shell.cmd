@echo off
rem ---------------------------------------------------------------------------
rem  Escape hatch for MSVC linker ambiguity (plan risk B1).
rem
rem  Rust normally locates link.exe through its own MSVC detection, so this
rem  wrapper is NOT part of the default build path. Use it only when a cargo
rem  command fails with "linker `link.exe` not found" or LNK1181, which happens
rem  when several Build Tools installs disagree or a POSIX `link` from Git Bash
rem  or MSYS2 wins the PATH lookup.
rem
rem  Usage:
rem    scripts\dev-shell.cmd cargo build --manifest-path src-tauri\Cargo.toml
rem    scripts\dev-shell.cmd bun run tauri build
rem
rem  The selected install is recorded as D-01 in docs\DECISIONS.md. Change it
rem  there and here together, never only here.
rem ---------------------------------------------------------------------------

setlocal

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo [dev-shell] vswhere.exe not found; cannot locate MSVC. See docs\DECISIONS.md D-01.
  exit /b 1
)

rem Newest install that actually ships the x64 C++ toolset.
rem `usebackq` + a quoted command needs the exe path quoted *inside* the
rem backquotes as well, or cmd splits on the space in "Program Files (x86)"
rem and reports vswhere.exe as not recognized while still succeeding via the
rem fallback below. Quoting it here keeps the intended path the only path.
set "VSINSTALL="
for /f "usebackq delims=" %%i in (`^""%VSWHERE%" -products * -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath^"`) do set "VSINSTALL=%%i"

if not defined VSINSTALL (
  echo [dev-shell] no MSVC x64 toolset found. See docs\DECISIONS.md D-01.
  exit /b 1
)

set "VCVARS=%VSINSTALL%\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VCVARS%" (
  echo [dev-shell] vcvars64.bat missing at "%VCVARS%".
  exit /b 1
)

echo [dev-shell] MSVC: %VSINSTALL%
rem vcvars64.bat itself probes for vswhere.exe on a bare PATH and prints
rem "'vswhere.exe' is not recognized" when it is not there. That message comes
rem from Microsoft's script, not this one, and is harmless: vcvars still
rem initializes correctly (verified — VCToolsInstallDir is set and link.exe
rem resolves to the MSVC one). Suppressing stderr here would also hide real
rem failures, so the message is left visible and explained instead.
call "%VCVARS%" >nul
if errorlevel 1 (
  echo [dev-shell] vcvars64.bat failed.
  exit /b 1
)

rem cargo/rustup live per-user and may not be on a bare cmd PATH.
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

if "%~1"=="" (
  echo [dev-shell] MSVC environment ready. Pass a command to run, e.g.:
  echo     scripts\dev-shell.cmd cargo build --manifest-path src-tauri\Cargo.toml
  exit /b 0
)

%*
exit /b %ERRORLEVEL%
