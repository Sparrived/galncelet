param(
    [string]$Version = "",
    [string]$Tag = "",
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$AllowDirty,
    [switch]$SkipBuild,
    [switch]$Offline,
    [switch]$Publish,
    [switch]$Draft,
    [switch]$Prerelease,
    [switch]$SkipRelease
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$TauriDir = Join-Path $Root "src-tauri"
$ReleaseDir = Join-Path $TauriDir ("target\{0}\release" -f $Target)
$BundleDir = Join-Path $ReleaseDir "bundle"
$ExePath = Join-Path $ReleaseDir "galncelet.exe"
$ChecksumsFileName = if ($Offline) { "SHA256SUMS-offline.txt" } else { "SHA256SUMS.txt" }
$ChecksumsPath = Join-Path $BundleDir $ChecksumsFileName
$OfflineConfigPath = Join-Path $TauriDir "tauri.offline.conf.json"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host ""
    Write-Host "==> $Name"
    & $Action
}

function Assert-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found in PATH."
    }
}

function Get-PackageVersion {
    $packageJson = Get-Content (Join-Path $Root "package.json") -Raw | ConvertFrom-Json
    return [string]$packageJson.version
}

function Set-Utf8NoBomContent {
    param(
        [string]$Path,
        [string[]]$Value
    )

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllLines($Path, $Value, $encoding)
}

function Set-JsonVersion {
    param(
        [string]$Path,
        [string]$NewVersion
    )

    $lines = Get-Content $Path
    $updated = $false
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^\s*"version"\s*:') {
            $indent = [regex]::Match($lines[$i], '^\s*').Value
            $comma = if ($lines[$i].TrimEnd().EndsWith(',')) { ',' } else { '' }
            $lines[$i] = '{0}"version": "{1}"{2}' -f $indent, $NewVersion, $comma
            $updated = $true
            break
        }
    }
    if (-not $updated) {
        throw "Could not find version field in $Path."
    }
    Set-Utf8NoBomContent $Path $lines
}

function Set-CargoPackageVersion {
    param(
        [string]$Path,
        [string]$NewVersion
    )

    $lines = Get-Content $Path
    $inPackage = $false
    $updated = $false

    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^\[package\]\s*$') {
            $inPackage = $true
            continue
        }

        if ($inPackage -and $lines[$i] -match '^\[') {
            break
        }

        if ($inPackage -and $lines[$i] -match '^version\s*=') {
            $lines[$i] = 'version = "{0}"' -f $NewVersion
            $updated = $true
            break
        }
    }

    if (-not $updated) {
        throw "Could not find [package] version in $Path."
    }

    Set-Utf8NoBomContent $Path $lines
}


function Get-RelativePath {
    param(
        [string]$BasePath,
        [string]$Path
    )

    $baseFullPath = [System.IO.Path]::GetFullPath($BasePath)
    $targetFullPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $baseFullPath.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $baseFullPath += [System.IO.Path]::DirectorySeparatorChar
    }

    $baseUri = New-Object System.Uri($baseFullPath)
    $targetUri = New-Object System.Uri($targetFullPath)
    return [System.Uri]::UnescapeDataString(
        $baseUri.MakeRelativeUri($targetUri).ToString()
    ) -replace '/', [System.IO.Path]::DirectorySeparatorChar
}

function Get-ReleaseArtifacts {
    param(
        [string]$ReleaseVersion,
        [bool]$OfflineBuild = $false
    )

    $patterns = @("*.msi", "*.exe", "*.nsis.zip", "*.app.tar.gz", "*.AppImage", "*.deb", "*.rpm", "*.dmg")
    $artifacts = @()
    $versionPattern = if ($OfflineBuild) { "*$ReleaseVersion*offline*" } else { "*$ReleaseVersion*" }

    if (Test-Path $BundleDir) {
        foreach ($pattern in $patterns) {
            $artifacts += Get-ChildItem -Path $BundleDir -Recurse -File -Filter $pattern |
                Where-Object {
                    $_.Name -like $versionPattern -and ($OfflineBuild -or $_.BaseName -notlike "*-offline")
                }
        }
    }

    if (-not $OfflineBuild) {
        $artifacts += Get-Item $ExePath -ErrorAction SilentlyContinue
    }
    return $artifacts | Sort-Object FullName -Unique
}

function Rename-OfflineArtifacts {
    param([string]$ReleaseVersion)

    $offlineArtifacts = @()
    $patterns = @(
        "Galncelet_${ReleaseVersion}_*.msi",
        "Galncelet_${ReleaseVersion}_*.exe"
    )

    foreach ($pattern in $patterns) {
        foreach ($artifact in Get-ChildItem -Path $BundleDir -Recurse -File -Filter $pattern) {
            if ($artifact.BaseName -like "*-offline") {
                $offlineArtifacts += $artifact
                continue
            }

            $newName = "{0}-offline{1}" -f $artifact.BaseName, $artifact.Extension
            $newPath = Join-Path $artifact.DirectoryName $newName
            if (Test-Path $newPath) {
                Remove-Item $newPath -Force
            }
            Move-Item $artifact.FullName $newPath
            $offlineArtifacts += Get-Item $newPath
        }
    }

    return $offlineArtifacts | Sort-Object FullName -Unique
}

function Test-WindowsGuiSubsystem {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        throw "Release executable was not found: $Path"
    }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = New-Object System.IO.BinaryReader($stream)
        $stream.Seek(0x3c, [System.IO.SeekOrigin]::Begin) | Out-Null
        $peHeaderOffset = $reader.ReadInt32()
        $stream.Seek($peHeaderOffset + 0x5c, [System.IO.SeekOrigin]::Begin) | Out-Null
        $subsystem = $reader.ReadUInt16()

        if ($subsystem -ne 2) {
            throw "Expected Windows GUI subsystem (2), found subsystem $subsystem. The app may open a console window."
        }
    }
    finally {
        $stream.Dispose()
    }
}

function New-ReleaseChecksumFile {
    param(
        [string]$ReleaseVersion,
        [bool]$OfflineBuild = $false
    )

    $artifacts = @(Get-ReleaseArtifacts -ReleaseVersion $ReleaseVersion -OfflineBuild $OfflineBuild)
    if ($artifacts.Count -eq 0) {
        throw "No release artifacts were found under $BundleDir."
    }

    $checksums = foreach ($artifact in $artifacts) {
        $hash = Get-FileHash -Algorithm SHA256 -Path $artifact.FullName
        $relativePath = Get-RelativePath $Root $artifact.FullName
        "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), ($relativePath -replace '\\', '/')
    }

    $checksums | Set-Content $ChecksumsPath -Encoding UTF8
    return $artifacts
}

function Publish-GitHubRelease {
    param(
        [string]$ReleaseTag,
        [string]$ReleaseVersion,
        [bool]$OfflineBuild,
        [bool]$IsDraft,
        [bool]$IsPrerelease
    )

    Assert-Command "gh"

    $artifactPaths = @(Get-ReleaseArtifacts -ReleaseVersion $ReleaseVersion -OfflineBuild $OfflineBuild | ForEach-Object { $_.FullName })
    $artifactPaths += $ChecksumsPath

    $releaseExists = $false
    cmd /c "gh release view $ReleaseTag >NUL 2>NUL"
    if ($LASTEXITCODE -eq 0) {
        $releaseExists = $true
    }

    if ($releaseExists) {
        gh release upload $ReleaseTag @artifactPaths --clobber
        return
    }

    $args = @("release", "create", $ReleaseTag)
    $args += $artifactPaths
    $args += @("--title", $ReleaseTag, "--generate-notes")
    if ($IsDraft) { $args += "--draft" }
    if ($IsPrerelease) { $args += "--prerelease" }
    gh @args
}

Push-Location $Root
try {
    Invoke-Step "Checking prerequisites" {
        Assert-Command "npm"
        Assert-Command "cargo"
        Assert-Command "git"
        if ($Publish -and -not $SkipRelease) {
            Assert-Command "gh"
        }
    }

    Invoke-Step "Checking working tree" {
        if (-not $AllowDirty) {
            $dirty = git status --porcelain
            if ($dirty) {
                throw "Working tree is not clean. Commit/stash changes or pass -AllowDirty."
            }
        }
    }

    if ($Version) {
        Invoke-Step "Syncing version $Version" {
            Set-JsonVersion (Join-Path $Root "package.json") $Version
            Set-JsonVersion (Join-Path $TauriDir "tauri.conf.json") $Version
            Set-CargoPackageVersion (Join-Path $TauriDir "Cargo.toml") $Version
        }
    }

    $releaseVersion = if ($Version) { $Version } else { Get-PackageVersion }
    $releaseTag = if ($Tag) { $Tag } else { "v$releaseVersion" }

    if (-not $SkipBuild) {
        Invoke-Step "Installing locked dependencies" {
            npm ci
        }

        Invoke-Step "Building Tauri release" {
            if ($Offline) {
                npm run tauri -- build --target $Target --config $OfflineConfigPath
                Rename-OfflineArtifacts $releaseVersion | Out-Null
            } else {
                npm run tauri -- build --target $Target
            }
        }
    }

    Invoke-Step "Verifying release executable subsystem" {
        Test-WindowsGuiSubsystem $ExePath
    }

    Invoke-Step "Writing release checksums" {
        New-ReleaseChecksumFile -ReleaseVersion $releaseVersion -OfflineBuild ([bool]$Offline) | Out-Null
    }

    if ($Publish -and -not $SkipRelease) {
        Invoke-Step "Publishing GitHub Release $releaseTag" {
            Publish-GitHubRelease -ReleaseTag $releaseTag -ReleaseVersion $releaseVersion -OfflineBuild ([bool]$Offline) -IsDraft ([bool]$Draft) -IsPrerelease ([bool]$Prerelease)
        }
    }

    Write-Host ""
    Write-Host "Release build completed for $releaseTag."
}
finally {
    Pop-Location
}
