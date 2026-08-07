param(
    [string]$Version = $(if ($env:GITHUB_REF_NAME) { $env:GITHUB_REF_NAME } else { "dev" }),
    [string]$MakensisPath = $(if ($env:MAKENSIS) { $env:MAKENSIS } else { "" }),
    [string]$SignToolPath = $(if ($env:SOURCEWEAVER_WINDOWS_SIGNTOOL) { $env:SOURCEWEAVER_WINDOWS_SIGNTOOL } elseif ($env:SIGNTOOL) { $env:SIGNTOOL } else { "" }),
    [string]$SigningCertificatePfxPath = $(if ($env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_PATH) { $env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_PATH } else { "" }),
    [string]$SigningCertificatePfxBase64 = $(if ($env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64) { $env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64 } else { "" }),
    [string]$SigningCertificatePassword = $(if ($env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD) { $env:SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD } else { "" }),
    [string]$TimestampUrl = $(if ($env:SOURCEWEAVER_WINDOWS_TIMESTAMP_URL) { $env:SOURCEWEAVER_WINDOWS_TIMESTAMP_URL } else { "http://timestamp.digicert.com" }),
    [switch]$SkipInstaller,
    [switch]$RequireInstaller,
    [switch]$RequireSigning
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
$TemporarySigningCertificatePath = $null

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

function Find-SignTool {
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

    $pathCommand = Get-Command "signtool" -ErrorAction SilentlyContinue
    if ($pathCommand) {
        return $pathCommand.Source
    }

    $windowsKitRoots = @(
        (Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"),
        (Join-Path $env:ProgramFiles "Windows Kits\10\bin")
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path $_) }

    foreach ($kitRoot in $windowsKitRoots) {
        $x64Candidates = Get-ChildItem -Path $kitRoot -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
            Sort-Object FullName -Descending
        if ($x64Candidates) {
            return $x64Candidates[0].FullName
        }

        $anyCandidates = Get-ChildItem -Path $kitRoot -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending
        if ($anyCandidates) {
            return $anyCandidates[0].FullName
        }
    }

    return $null
}

function Get-SigningCertificatePath {
    if (-not [string]::IsNullOrWhiteSpace($SigningCertificatePfxPath)) {
        if (-not (Test-Path $SigningCertificatePfxPath)) {
            throw "Configured signing certificate PFX does not exist: $SigningCertificatePfxPath"
        }
        return (Resolve-Path $SigningCertificatePfxPath).Path
    }

    if (-not [string]::IsNullOrWhiteSpace($SigningCertificatePfxBase64)) {
        $tempPath = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName() + ".pfx")
        [System.IO.File]::WriteAllBytes($tempPath, [Convert]::FromBase64String($SigningCertificatePfxBase64))
        $script:TemporarySigningCertificatePath = $tempPath
        return $tempPath
    }

    return $null
}

function Invoke-AuthenticodeSigning {
    param([string[]]$Files)

    $filesToSign = @($Files | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path $_) })
    if ($filesToSign.Count -eq 0) {
        return
    }

    $signingConfigured = (-not [string]::IsNullOrWhiteSpace($SigningCertificatePfxPath)) -or (-not [string]::IsNullOrWhiteSpace($SigningCertificatePfxBase64))
    if (-not $signingConfigured) {
        if ($RequireSigning) {
            throw "Windows signing certificate is not configured. Set SOURCEWEAVER_WINDOWS_SIGNING_PFX_PATH or SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64."
        }
        return
    }

    $signtool = Find-SignTool -RequestedPath $SignToolPath
    if (-not $signtool) {
        throw "signtool.exe was not found. Install the Windows SDK or set SOURCEWEAVER_WINDOWS_SIGNTOOL/SIGNTOOL."
    }

    $certificatePath = Get-SigningCertificatePath
    if (-not $certificatePath) {
        throw "Windows signing certificate path could not be resolved."
    }

    foreach ($file in $filesToSign) {
        $signArguments = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $TimestampUrl, "/f", $certificatePath)
        if (-not [string]::IsNullOrWhiteSpace($SigningCertificatePassword)) {
            $signArguments += @("/p", $SigningCertificatePassword)
        }
        $signArguments += @($file)

        Write-Output "Signing $file"
        & $signtool @signArguments
        if ($LASTEXITCODE -ne 0) {
            throw "signtool failed with exit code $LASTEXITCODE for $file"
        }

        $signature = Get-AuthenticodeSignature $file
        if (-not $signature.SignerCertificate) {
            throw "No Authenticode signer certificate was found after signing $file"
        }
    }
}

try {
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
    $RunningNotes = @(
        '# Running Source Weaver on Windows',
        '',
        'Run from the extracted zip:',
        '',
        '```powershell',
        '.\sourceweaver-desktop.exe',
        '.\sourceweaver.exe --help',
        '```',
        '',
        'Or install the NSIS setup package from the same release:',
        '',
        '```powershell',
        ".\$PackageName-setup.exe",
        '```',
        '',
        'For unattended per-user installation, keep `/D=` as the final argument:',
        '',
        '```powershell',
        "& .\$PackageName-setup.exe /S `"/D=`$env:LOCALAPPDATA\Programs\Source Weaver`"",
        '```',
        '',
        'See docs\packaging.md for Windows runtime, install, uninstall, signing, and SmartScreen notes.'
    ) -join [Environment]::NewLine
    $RunningNotes | Set-Content -Encoding UTF8 (Join-Path $PackageDir "RUNNING_ON_WINDOWS.md")

    Invoke-AuthenticodeSigning -Files @(
        (Join-Path $PackageDir "sourceweaver.exe"),
        (Join-Path $PackageDir "sourceweaver-desktop.exe")
    )

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
            Invoke-AuthenticodeSigning -Files @($Installer)
            Write-Output $Installer
        } elseif ($RequireInstaller) {
            throw "NSIS makensis was not found, but -RequireInstaller was set. Install NSIS or set -MakensisPath/-MAKENSIS."
        } else {
            Write-Warning "NSIS makensis was not found; skipped Windows installer creation. Install NSIS or pass -MakensisPath to build the setup EXE."
        }
    }
} finally {
    if ($TemporarySigningCertificatePath -and (Test-Path $TemporarySigningCertificatePath)) {
        Remove-Item -Force $TemporarySigningCertificatePath -ErrorAction SilentlyContinue
    }
}
