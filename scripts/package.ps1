param(
  [ValidateSet("android")]
  [string]$Target = "android",

  [ValidateSet("debug", "release")]
  [string]$Configuration = "debug",

  [switch]$SplitPerAbi,
  [switch]$SkipFrontendSync,
  [switch]$DryRun,
  [switch]$SkipReleaseSigning,
  [string]$KeystorePath = "",
  [string]$KeystoreAlias = "",
  [string]$KeystorePassword = "",
  [string]$KeyPassword = ""
)

$ErrorActionPreference = "Stop"

function Resolve-Cargo {
  $cargoFromProfile = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
  if (Test-Path $cargoFromProfile) {
    return $cargoFromProfile
  }

  $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
  if ($null -ne $cargoCmd) {
    return $cargoCmd.Path
  }

  throw "cargo not found. Install Rust toolchain first."
}

function Ensure-JavaHome {
  if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME "bin\java.exe"))) {
    return
  }

  $adoptiumRoot = "C:\Program Files\Eclipse Adoptium"
  if (Test-Path $adoptiumRoot) {
    $candidate = Get-ChildItem $adoptiumRoot -Directory -ErrorAction SilentlyContinue |
      Sort-Object Name -Descending |
      Select-Object -First 1
    if ($candidate) {
      $env:JAVA_HOME = $candidate.FullName
    }
  }

  if (-not $env:JAVA_HOME) {
    $studioJbr = "C:\Program Files\Android\Android Studio\jbr"
    if (Test-Path (Join-Path $studioJbr "bin\java.exe")) {
      $env:JAVA_HOME = $studioJbr
    }
  }

  if (-not $env:JAVA_HOME -or -not (Test-Path (Join-Path $env:JAVA_HOME "bin\java.exe"))) {
    throw "JAVA_HOME not found. Install JDK 17 and set JAVA_HOME."
  }

  if ($env:PATH -notlike "*$($env:JAVA_HOME)\bin*") {
    $env:PATH = "$($env:JAVA_HOME)\bin;$($env:PATH)"
  }
}

function Resolve-SdkManager([string]$androidHome) {
  $latestPath = Join-Path $androidHome "cmdline-tools\latest\bin\sdkmanager.bat"
  if (Test-Path $latestPath) {
    return $latestPath
  }

  $cmdlineRoot = Join-Path $androidHome "cmdline-tools"
  if (-not (Test-Path $cmdlineRoot)) {
    return $null
  }

  $fallback = Get-ChildItem $cmdlineRoot -Directory -ErrorAction SilentlyContinue |
    ForEach-Object { Join-Path $_.FullName "bin\sdkmanager.bat" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
  return $fallback
}

function Ensure-AndroidEnv {
  if (-not $env:ANDROID_HOME) {
    $env:ANDROID_HOME = Join-Path $env:LOCALAPPDATA "Android\Sdk"
  }
  if (-not $env:ANDROID_SDK_ROOT) {
    $env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
  }

  if (-not (Test-Path $env:ANDROID_HOME)) {
    throw "ANDROID_HOME does not exist: $($env:ANDROID_HOME)"
  }

  $sdkManager = Resolve-SdkManager -androidHome $env:ANDROID_HOME
  if (-not $sdkManager) {
    throw "Android cmdline-tools not found. Install Android SDK cmdline-tools."
  }

  if (-not $env:NDK_HOME -or -not (Test-Path $env:NDK_HOME)) {
    $ndkRoot = Join-Path $env:ANDROID_HOME "ndk"
    $latestNdk = Get-ChildItem $ndkRoot -Directory -ErrorAction SilentlyContinue |
      Sort-Object Name -Descending |
      Select-Object -First 1
    if ($latestNdk) {
      $env:NDK_HOME = $latestNdk.FullName
    }
  }

  if (-not $env:NDK_HOME -or -not (Test-Path $env:NDK_HOME)) {
    throw "Android NDK not found. Install NDK and set NDK_HOME."
  }

  $env:ANDROID_NDK_HOME = $env:NDK_HOME

  $platformTools = Join-Path $env:ANDROID_HOME "platform-tools"
  if ((Test-Path $platformTools) -and $env:PATH -notlike "*$platformTools*") {
    $env:PATH = "$platformTools;$($env:PATH)"
  }
}

function Sync-MobileFrontend([string]$repoRoot) {
  $src = Join-Path $repoRoot "ui"
  $dst = Join-Path $repoRoot "tauri-app\mobile-ui"
  $files = @("index.html", "app.js", "styles.css")
  $assetSrc = Join-Path $src "assets"
  $assetDst = Join-Path $dst "assets"

  New-Item -ItemType Directory -Force -Path $dst | Out-Null
  foreach ($file in $files) {
    $from = Join-Path $src $file
    if (-not (Test-Path $from)) {
      throw "Missing frontend file: $from"
    }
    Copy-Item -Path $from -Destination (Join-Path $dst $file) -Force
  }

  if (-not (Test-Path $assetSrc)) {
    throw "Missing frontend asset directory: $assetSrc"
  }
  if (Test-Path $assetDst) {
    Remove-Item -Path $assetDst -Recurse -Force
  }
  Copy-Item -Path $assetSrc -Destination $assetDst -Recurse -Force
}

function Ensure-AndroidInit([string]$cargo, [string]$tauriRoot) {
  $androidGen = Join-Path $tauriRoot "gen\android"
  if (Test-Path $androidGen) {
    return
  }

  Write-Host "Android project not initialized. Running 'tauri android init --ci'..."
  & $cargo tauri android init --ci
}

function Resolve-BuildToolExe([string]$androidHome, [string]$exeName) {
  $buildToolsRoot = Join-Path $androidHome "build-tools"
  if (-not (Test-Path $buildToolsRoot)) {
    throw "Android build-tools not found under: $buildToolsRoot"
  }

  $candidate = Get-ChildItem $buildToolsRoot -Directory -ErrorAction SilentlyContinue |
    Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName $exeName } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1

  if (-not $candidate) {
    throw "Unable to find $exeName in Android build-tools."
  }
  return $candidate
}

function Resolve-ReleaseSigningProfile {
  $profile = [ordered]@{
    KeystorePath = $KeystorePath
    KeystoreAlias = $KeystoreAlias
    KeystorePassword = $KeystorePassword
    KeyPassword = $KeyPassword
    Source = "cli"
  }

  if (-not $profile.KeystorePath) { $profile.KeystorePath = $env:DDZ_ANDROID_KEYSTORE_PATH }
  if (-not $profile.KeystoreAlias) { $profile.KeystoreAlias = $env:DDZ_ANDROID_KEY_ALIAS }
  if (-not $profile.KeystorePassword) { $profile.KeystorePassword = $env:DDZ_ANDROID_KEYSTORE_PASSWORD }
  if (-not $profile.KeyPassword) { $profile.KeyPassword = $env:DDZ_ANDROID_KEY_PASSWORD }

  if ($profile.KeystorePath -and $profile.KeystoreAlias -and $profile.KeystorePassword) {
    if (-not $profile.KeyPassword) {
      $profile.KeyPassword = $profile.KeystorePassword
    }
    $profile.Source = "custom-keystore"
    return $profile
  }

  $debugKeystore = Join-Path $env:USERPROFILE ".android\debug.keystore"
  if (-not (Test-Path $debugKeystore)) {
    throw "No signing config found. Provide keystore args/env, or create $debugKeystore."
  }
  $profile.KeystorePath = $debugKeystore
  $profile.KeystoreAlias = "androiddebugkey"
  $profile.KeystorePassword = "android"
  $profile.KeyPassword = "android"
  $profile.Source = "android-debug-keystore"
  return $profile
}

function Sign-And-VerifyApk([string]$unsignedApk, [string]$androidHome) {
  $apksigner = Resolve-BuildToolExe -androidHome $androidHome -exeName "apksigner.bat"
  $profile = Resolve-ReleaseSigningProfile

  if (-not (Test-Path $unsignedApk)) {
    throw "Unsigned APK not found: $unsignedApk"
  }
  if (-not (Test-Path $profile.KeystorePath)) {
    throw "Keystore not found: $($profile.KeystorePath)"
  }

  $signedApk = if ($unsignedApk -like "*-unsigned.apk") {
    $unsignedApk -replace "-unsigned\.apk$", "-signed.apk"
  } else {
    "$unsignedApk.signed.apk"
  }

  Write-Host "Signing release APK ($($profile.Source)): $unsignedApk"
  $null = & $apksigner sign `
    --ks $profile.KeystorePath `
    --ks-key-alias $profile.KeystoreAlias `
    --ks-pass "pass:$($profile.KeystorePassword)" `
    --key-pass "pass:$($profile.KeyPassword)" `
    --out $signedApk `
    $unsignedApk

  $null = & $apksigner verify --verbose $signedApk
  if ($LASTEXITCODE -ne 0) {
    throw "APK signature verification failed: $signedApk"
  }
  Write-Host "Verified signed APK: $signedApk"
  return $signedApk
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tauriRoot = Join-Path $repoRoot "tauri-app\src-tauri"
$cargo = Resolve-Cargo

if ($Target -ne "android") {
  throw "Only 'android' target is supported right now."
}

if (-not $SkipFrontendSync) {
  Write-Host "Syncing frontend assets to tauri-app/mobile-ui ..."
  Sync-MobileFrontend -repoRoot $repoRoot
}

Ensure-JavaHome
Ensure-AndroidEnv

if ($DryRun) {
  Write-Host "DryRun complete:"
  Write-Host "  JAVA_HOME=$($env:JAVA_HOME)"
  Write-Host "  ANDROID_HOME=$($env:ANDROID_HOME)"
  Write-Host "  NDK_HOME=$($env:NDK_HOME)"
  if ($Configuration -eq "release" -and -not $SkipReleaseSigning) {
    $profile = Resolve-ReleaseSigningProfile
    Write-Host "  SIGN_RELEASE=true"
    Write-Host "  SIGN_SOURCE=$($profile.Source)"
    Write-Host "  KEYSTORE=$($profile.KeystorePath)"
    Write-Host "  KEY_ALIAS=$($profile.KeystoreAlias)"
  } else {
    Write-Host "  SIGN_RELEASE=false"
  }
  exit 0
}

Push-Location $tauriRoot
try {
  Ensure-AndroidInit -cargo $cargo -tauriRoot $tauriRoot

  $args = @("tauri", "android", "build", "--apk", "--ci")
  if ($Configuration -eq "debug") {
    $args += "--debug"
  }
  if ($SplitPerAbi) {
    $args += "--split-per-abi"
  }

  Write-Host "Building Android APK: $cargo $($args -join ' ')"
  & $cargo @args
} finally {
  Pop-Location
}

$apkRoot = Join-Path $tauriRoot "gen\android\app\build\outputs\apk"
$apks = Get-ChildItem -Path $apkRoot -Recurse -Filter *.apk -ErrorAction SilentlyContinue |
  Sort-Object LastWriteTime -Descending

if (-not $apks -or $apks.Count -eq 0) {
  throw "No APK generated. Check build logs."
}

if ($Configuration -eq "release" -and -not $SkipReleaseSigning) {
  $unsignedReleaseApks = $apks |
    Where-Object { $_.Name -like "*-release-unsigned.apk" -and $_.FullName -notlike "*baselineProfiles*" }

  if (-not $unsignedReleaseApks -or $unsignedReleaseApks.Count -eq 0) {
    throw "Release unsigned APK not found under: $apkRoot"
  }

  $signedPaths = @()
  foreach ($unsigned in $unsignedReleaseApks) {
    $signedPaths += Sign-And-VerifyApk -unsignedApk $unsigned.FullName -androidHome $env:ANDROID_HOME
  }

  $signedApks = $signedPaths | ForEach-Object { Get-Item $_ } | Sort-Object LastWriteTime -Descending
  Write-Host ""
  Write-Host "Signed APK files:"
  foreach ($apk in $signedApks) {
    Write-Host "  $($apk.FullName)"
  }
}

Write-Host ""
Write-Host "APK files:"
foreach ($apk in $apks) {
  Write-Host "  $($apk.FullName)"
}
