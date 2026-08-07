param(
    [string]$InstallerPath = "",
    [string]$InstallDir = $(Join-Path $env:TEMP "sourceweaver-installer-validation")
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
    $latestInstaller = Get-ChildItem -Path (Join-Path $Root "target\package") -Filter "*-setup.exe" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latestInstaller) {
        throw "No Windows setup installer found under target\package"
    }
    $InstallerPath = $latestInstaller.FullName
}

$InstallerPath = (Resolve-Path $InstallerPath).Path
if ($InstallDir -match '"') {
    throw "InstallDir must not contain quotes because NSIS /D= must be the final unquoted argument"
}

Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue

Write-Output "Installing $InstallerPath into $InstallDir"
$installProcess = Start-Process -FilePath $InstallerPath -ArgumentList @("/S", "/D=$InstallDir") -Wait -PassThru
if ($null -ne $installProcess.ExitCode -and $installProcess.ExitCode -ne 0) {
    throw "Installer exited with code $($installProcess.ExitCode)"
}

$requiredFiles = @(
    "sourceweaver-desktop.exe",
    "sourceweaver.exe",
    "assets\sourceweaver.ico",
    "docs\packaging.md",
    "README.md",
    "LICENSE",
    "Uninstall Source Weaver.exe"
)
foreach ($relativePath in $requiredFiles) {
    $path = Join-Path $InstallDir $relativePath
    if (-not (Test-Path $path)) {
        throw "Installed file is missing: $path"
    }
}

$startMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Source Weaver\Source Weaver.lnk"
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Source Weaver.lnk"
foreach ($shortcut in @($startMenuShortcut, $desktopShortcut)) {
    if (-not (Test-Path $shortcut)) {
        throw "Installer shortcut is missing: $shortcut"
    }
}

& (Join-Path $InstallDir "sourceweaver.exe") --help | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Installed CLI help exited with code $LASTEXITCODE"
}

$uninstaller = Join-Path $InstallDir "Uninstall Source Weaver.exe"
Write-Output "Uninstalling $InstallDir"
$uninstallProcess = Start-Process -FilePath $uninstaller -ArgumentList @("/S", "_?=$InstallDir") -Wait -PassThru
if ($null -ne $uninstallProcess.ExitCode -and $uninstallProcess.ExitCode -ne 0) {
    throw "Uninstaller exited with code $($uninstallProcess.ExitCode)"
}

if (Test-Path $InstallDir) {
    throw "Install directory still exists after uninstall: $InstallDir"
}
foreach ($shortcut in @($startMenuShortcut, $desktopShortcut)) {
    if (Test-Path $shortcut) {
        throw "Shortcut still exists after uninstall: $shortcut"
    }
}

Write-Output "Windows installer install/uninstall validation passed."
