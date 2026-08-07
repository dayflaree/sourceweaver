param(
    [string]$Version = $(if ($env:GITHUB_REF_NAME) { $env:GITHUB_REF_NAME } else { "dev" }),
    [string]$MakensisPath = $(if ($env:MAKENSIS) { $env:MAKENSIS } else { "" }),
    [switch]$SkipInstaller,
    [switch]$RequireInstaller
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = "dev"
}
$SafeVersion = $Version -replace '[^A-Za-z0-9._-]', '-'
$PackageName = "sourceweaver-$SafeVersion-windows-x86_64"
$PackageRoot = Join-Path $Root "target\package"
$PackageDir = Join-Path $PackageRoot $PackageName
$Archive = Join-Path $PackageRoot "$PackageName.zip"
$Installer = Join-Path $PackageRoot "$PackageName-setup.exe"

function Find-Makensis {
    param([string]$RequestedPath)

    if (-not [string]::IsNullOrWhiteSpace($RequestedPath)) {
        if (Test-Path $RequestedPath) {
            return (Resolve-Path $RequestedPath).Path
        }
        $requestedCommand = Get-Command $RequestedPath -ErrorAction SilentlyContinue
        if ($requestedCommand) {
            return $requestedCommand.Source
        }
    }

    $pathCommand = Get-Command "makensis" -ErrorAction SilentlyContinue
    if ($pathCommand) {
        return $pathCommand.Source
    }

    $candidateRoots = @(${env:ProgramFiles(x86)}, $env:ProgramFiles) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    foreach ($rootDir in $candidateRoots) {
        $candidate = Join-Path $rootDir "NSIS\makensis.exe"
        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).Path
        }
    }

    return $null
}

Remove-Item -Recurse -Force $PackageDir -ErrorAction SilentlyContinue
Remove-Item -Force $Archive -ErrorAction SilentlyContinue
Remove-Item -Force $Installer -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $PackageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PackageDir "docs") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PackageDir "assets") | Out-Null

cargo build --release -p sourceweaver-cli -p sourceweaver-desktop
Copy-Item (Join-Path $Root "target\release\sourceweaver.exe") (Join-Path $PackageDir "sourceweaver.exe")
Copy-Item (Join-Path $Root "target\release\sourceweaver-desktop.exe") (Join-Path $PackageDir "sourceweaver-desktop.exe")
Copy-Item (Join-Path $Root "LICENSE") (Join-Path $PackageDir "LICENSE")
Copy-Item (Join-Path $Root "README.md") (Join-Path $PackageDir "README.md")
Copy-Item -Recurse (Join-Path $Root "docs\*") (Join-Path $PackageDir "docs")
Copy-Item (Join-Path $Root "packaging\windows\sourceweaver.ico") (Join-Path $PackageDir "assets\sourceweaver.ico")
@"
# Running Source Weaver on Windows

Run from the extracted zip:

```powershell
.\sourceweaver-desktop.exe
.\sourceweaver.exe --help
```

Or install the NSIS setup package from the same release:

```powershell
.\$PackageName-setup.exe
```

For unattended per-user installation, keep `/D=` as the final argument:

```powershell
& .\$PackageName-setup.exe /S "/D=`$env:LOCALAPPDATA\Programs\Source Weaver"
```

See docs\packaging.md for Windows runtime, install, uninstall, and SmartScreen notes.
"@ | Set-Content -Encoding UTF8 (Join-Path $PackageDir "RUNNING_ON_WINDOWS.md")

Compress-Archive -Path $PackageDir -DestinationPath $Archive -Force
Write-Output $Archive

if (-not $SkipInstaller) {
    $Makensis = Find-Makensis -RequestedPath $MakensisPath
    if ($Makensis) {
        $NsiScript = Join-Path $Root "packaging\windows\sourceweaver.nsi"
        & $Makensis "/V3" "/DVERSION=$SafeVersion" "/DPACKAGE_DIR=$PackageDir" "/DOUTPUT_EXE=$Installer" "/DROOT_DIR=$Root" $NsiScript
        if ($LASTEXITCODE -ne 0) {
            throw "makensis failed with exit code $LASTEXITCODE"
        }
        Write-Output $Installer
    } elseif ($RequireInstaller) {
        throw "NSIS makensis was not found, but -RequireInstaller was set. Install NSIS or set -MakensisPath/-MAKENSIS."
    } else {
        Write-Warning "NSIS makensis was not found; skipped Windows installer creation. Install NSIS or pass -MakensisPath to build the setup EXE."
    }
}
