param(
    [string]$Version = $(if ($env:GITHUB_REF_NAME) { $env:GITHUB_REF_NAME } else { "dev" })
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$PackageName = "sourceweaver-$Version-windows-x86_64"
$PackageRoot = Join-Path $Root "target\package"
$PackageDir = Join-Path $PackageRoot $PackageName
$Archive = Join-Path $PackageRoot "$PackageName.zip"

Remove-Item -Recurse -Force $PackageDir -ErrorAction SilentlyContinue
Remove-Item -Force $Archive -ErrorAction SilentlyContinue
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

No installer is required for this release package. See docs\packaging.md for notes about Windows runtime expectations.
"@ | Set-Content -Encoding UTF8 (Join-Path $PackageDir "RUNNING_ON_WINDOWS.md")

Compress-Archive -Path $PackageDir -DestinationPath $Archive -Force
Write-Output $Archive
