[CmdletBinding()]
param(
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "Windows installers must be built on Windows."
}

$projectDir = Split-Path -Parent $PSScriptRoot
Set-Location $projectDir

if (-not (Get-Command java.exe -ErrorAction SilentlyContinue)) {
    throw "JDK 21 is required. Install it and make sure JAVA_HOME and PATH are configured."
}

if (-not (Get-Command candle.exe -ErrorAction SilentlyContinue) -or
    -not (Get-Command light.exe -ErrorAction SilentlyContinue)) {
    throw "WiX Toolset 3.x is required to build MSI/EXE installers. Add its bin directory to PATH."
}

$gradleTasks = @(
    ":composeApp:clean"
)

if (-not $SkipTests) {
    $gradleTasks += ":composeApp:desktopTest"
}

$gradleTasks += @(
    ":composeApp:packageMsi"
    ":composeApp:packageExe"
)

& .\gradlew.bat @gradleTasks --stacktrace
if ($LASTEXITCODE -ne 0) {
    throw "Gradle build failed with exit code $LASTEXITCODE."
}

$binariesDir = Join-Path $projectDir "composeApp\build\compose\binaries\main"
$artifacts = Get-ChildItem $binariesDir -Recurse -File |
    Where-Object { $_.Extension -in ".msi", ".exe" }

if (-not $artifacts) {
    throw "The build completed, but no MSI/EXE files were found in $binariesDir."
}

Write-Host "Windows installers:"
$artifacts | ForEach-Object { Write-Host "  $($_.FullName)" }
