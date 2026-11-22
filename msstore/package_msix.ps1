<#
.SYNOPSIS
    Packages R.I.D.E (by Byte-Labs) into an official Microsoft Store MSIX Package.

.DESCRIPTION
    Automates the entire Windows SDK MSIX build pipeline:
      1. Validates or builds the release binary (ride-desktop.exe).
      2. Stages binary, assets, and AppxManifest.xml.
      3. Compiles the Package Resource Index (resources.pri) via makepri.exe.
      4. Packs Byte-Labs.R.I.D.E_2.0.0.0_x64.msix via makeappx.exe.
      5. Creates Byte-Labs.R.I.D.E_2.0.0.0_x64.msixupload for Microsoft Partner Center.
      6. Optionally signs the package for local side-load testing with -SignForLocalTesting.

.PARAMETER BuildRelease
    Compiles cargo build --release -p ride-desktop before packaging.

.PARAMETER SignForLocalTesting
    Generates a trusted local developer certificate matching Partner Center Publisher CN
    and signs the MSIX package for instant double-click installation on this machine.

.EXAMPLE
    .\package_msix.ps1
    .\package_msix.ps1 -BuildRelease -SignForLocalTesting
#>

[CmdletBinding()]
param(
    [switch]$BuildRelease = $false,
    [switch]$SignForLocalTesting = $false,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

if (-not $OutputDir) {
    $OutputDir = "$PSScriptRoot\dist"
}

$ProjectRoot = Resolve-Path "$PSScriptRoot\.."
$StagingDir  = "$PSScriptRoot\staging"
$AssetsDir   = "$PSScriptRoot\Assets"
$Manifest    = "$PSScriptRoot\AppxManifest.xml"
$PriConfig   = "$PSScriptRoot\priconfig.xml"

$PackageIdentity = "Byte-Labs.R.I.D.E"
$PublisherCN     = "CN=6CDD7E10-2EAB-4D1D-B6CA-5D8F278050DF"
$PackageVersion  = "2.0.0.0"
$PackageArch     = "x64"
$PackageFileName = "${PackageIdentity}_${PackageVersion}_${PackageArch}.msix"
$UploadFileName  = "${PackageIdentity}_${PackageVersion}_${PackageArch}.msixupload"

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "  R.I.D.E — Microsoft Store MSIX Packaging Pipeline" -ForegroundColor Cyan
Write-Host "  Publisher: Byte-Labs | ID: $PackageIdentity | Ver: $PackageVersion" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan

# 1. Locate Windows SDK Tools
Write-Host "`n[1/6] Detecting Windows SDK Build Tools..." -ForegroundColor Yellow
$SdkBasePaths = @(
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64",
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.28000.0\x64",
    "C:\Program Files (x86)\Windows Kits\10\bin\*\x64"
)

$MakeAppxPath = $null
$MakePriPath  = $null
$SignToolPath = $null

foreach ($base in $SdkBasePaths) {
    $resolved = Resolve-Path $base -ErrorAction SilentlyContinue | Select-Object -Last 1 -ExpandProperty Path
    if ($resolved) {
        $candidateMakeAppx = Join-Path $resolved "makeappx.exe"
        $candidateMakePri  = Join-Path $resolved "makepri.exe"
        $candidateSignTool = Join-Path $resolved "signtool.exe"
        if ((Test-Path $candidateMakeAppx) -and (Test-Path $candidateMakePri)) {
            $MakeAppxPath = $candidateMakeAppx
            $MakePriPath  = $candidateMakePri
            $SignToolPath = $candidateSignTool
            break
        }
    }
}

if (-not $MakeAppxPath) {
    throw "makeappx.exe not found in Windows Kits directory. Please ensure Windows 10/11 SDK is installed."
}
Write-Host "  Found makeappx.exe : $MakeAppxPath" -ForegroundColor Green
Write-Host "  Found makepri.exe  : $MakePriPath" -ForegroundColor Green

# 2. Build or Verify Release Binary
Write-Host "`n[2/6] Verifying Release Executable..." -ForegroundColor Yellow
$TargetExe = "$ProjectRoot\target\release\ride-desktop.exe"

if ($BuildRelease -or (-not (Test-Path $TargetExe))) {
    Write-Host "  Building release binary: cargo build --release -p ride-desktop ..." -ForegroundColor Cyan
    Push-Location $ProjectRoot
    try {
        & cargo build --release -p ride-desktop
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo release build failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $TargetExe)) {
    throw "Executable not found at $TargetExe. Run with -BuildRelease to compile it."
}
$exeSizeMb = [math]::Round((Get-Item $TargetExe).Length / 1MB, 2)
Write-Host "  Found binary: $TargetExe ($exeSizeMb MB)" -ForegroundColor Green

# 3. Prepare Clean Staging Area
Write-Host "`n[3/6] Staging Application Files..." -ForegroundColor Yellow
if (Test-Path $StagingDir) {
    Remove-Item -Recurse -Force $StagingDir
}
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
New-Item -ItemType Directory -Force -Path "$StagingDir\Assets" | Out-Null
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Copy-Item $TargetExe -Destination "$StagingDir\ride-desktop.exe" -Force
Copy-Item $Manifest  -Destination "$StagingDir\AppxManifest.xml" -Force
Copy-Item "$AssetsDir\*" -Destination "$StagingDir\Assets\" -Recurse -Force
Write-Host "  Staged executable, manifest, and visual assets." -ForegroundColor Green

# 4. Generate PRI Resource Index
Write-Host "`n[4/6] Compiling Package Resource Index (resources.pri)..." -ForegroundColor Yellow
$PriOutput = "$StagingDir\resources.pri"
& $MakePriPath new /pr $StagingDir /cf $PriConfig /of $PriOutput /o | Out-Null
if (-not (Test-Path $PriOutput)) {
    Write-Warning "makepri.exe did not generate resources.pri; packaging will proceed with manifest assets."
} else {
    Write-Host "  Compiled resources.pri successfully." -ForegroundColor Green
}

# 5. Pack MSIX with makeappx.exe
Write-Host "`n[5/6] Creating MSIX Package..." -ForegroundColor Yellow
$MsixPath = Join-Path $OutputDir $PackageFileName
& $MakeAppxPath pack /d $StagingDir /p $MsixPath /o /v

if (-not (Test-Path $MsixPath)) {
    throw "makeappx.exe packaging failed."
}
$pkgSizeMb = [math]::Round((Get-Item $MsixPath).Length / 1MB, 2)
Write-Host "  Successfully created: $MsixPath ($pkgSizeMb MB)" -ForegroundColor Green

# 6. Create Partner Center .msixupload bundle
Write-Host "`n[6/6] Creating Microsoft Partner Center Upload Bundle..." -ForegroundColor Yellow
$UploadPath = Join-Path $OutputDir $UploadFileName
$TempZipPath = [System.IO.Path]::ChangeExtension($UploadPath, ".zip")

if (Test-Path $TempZipPath) {
    Remove-Item -Force $TempZipPath
}
if (Test-Path $UploadPath) {
    Remove-Item -Force $UploadPath
}

Compress-Archive -Path $MsixPath -DestinationPath $TempZipPath -CompressionLevel Optimal
Rename-Item -Path $TempZipPath -NewName (Split-Path $UploadPath -Leaf) -Force
Write-Host "  Created Partner Center upload bundle: $UploadPath" -ForegroundColor Green

# Optional: Sign for Local Sideload Testing
if ($SignForLocalTesting) {
    Write-Host "`n[OPTIONAL] Signing Package for Local Sideloading..." -ForegroundColor Yellow
    $CertSubject = $PublisherCN
    $Cert = Get-ChildItem -Path Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $CertSubject } | Select-Object -First 1

    if (-not $Cert) {
        Write-Host "  Creating self-signed developer certificate for $CertSubject..." -ForegroundColor Cyan
        $Cert = New-SelfSignedCertificate `
            -Type Custom `
            -Subject $CertSubject `
            -KeyUsage DigitalSignature `
            -FriendlyName "R.I.D.E Byte-Labs Dev Certificate" `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
    }

    if ($SignToolPath -and (Test-Path $SignToolPath)) {
        & $SignToolPath sign /fd SHA256 /a /sha1 $Cert.Thumbprint $MsixPath
        Write-Host "  Signed $MsixPath with certificate thumbprint $($Cert.Thumbprint)" -ForegroundColor Green
    } else {
        Set-AuthenticodeSignature -FilePath $MsixPath -Certificate $Cert -HashAlgorithm SHA256 | Out-Null
        Write-Host "  Signed $MsixPath with PowerShell Authenticode." -ForegroundColor Green
    }
}

Write-Host "`n======================================================================" -ForegroundColor Green
Write-Host "  PACKAGE GENERATION COMPLETE!" -ForegroundColor Green
Write-Host "  Store Package : $MsixPath" -ForegroundColor Green
Write-Host "  Store Upload  : $UploadPath" -ForegroundColor Green
Write-Host "======================================================================" -ForegroundColor Green
