# Source Weaver BSPZIP Windows game-bin wrapper example.
#
# This wrapper is for users who already have a legal local Source game/SDK
# install and a BSPZIP-compatible executable. It runs from the game bin folder
# so local DLLs and vproject-style auto-detection see the same context a mapper
# would use from a manual command prompt.

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $BspzipArgs
)

$ErrorActionPreference = "Stop"

if (-not $env:SOURCEWEAVER_BSPZIP_BIN) {
    throw "Set SOURCEWEAVER_BSPZIP_BIN to the directory containing bspzip.exe or a compatible packer."
}

$exe = if ($env:SOURCEWEAVER_BSPZIP_EXE) { $env:SOURCEWEAVER_BSPZIP_EXE } else { "bspzip.exe" }
$tool = Join-Path $env:SOURCEWEAVER_BSPZIP_BIN $exe

Push-Location $env:SOURCEWEAVER_BSPZIP_BIN
try {
    & $tool @BspzipArgs
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
