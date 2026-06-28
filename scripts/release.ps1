param(
    [string]$Version = "",
    [string]$Tag = "",
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$AllowDirty,
    [switch]$SkipBuild,
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
$ChecksumsPath = Join-Path $BundleDir "SHA256SUMS.txt"

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

function Set-JsonVersion {
    param(
        [string]$Path,
        [string]$NewVersion
    )

    $json = Get-Content $Path -Raw | ConvertFrom-Json
    $json.version = $NewVersion
    $json | ConvertTo-Json -Depth 100 | Set-Content $Path -Encoding UTF8
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

    $lines | Set-Content $Path -Encoding UTF8
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
    $patterns = @("*.msi", "*.exe", "*.nsis.zip", "*.app.tar.gz", "*.AppImage", "*.deb", "*.rpm", "*.dmg")
    $artifacts = @()

    if (Test-Path $BundleDir) {
        foreach ($pattern in $patterns) {
            $artifacts += Get-ChildItem -Path $BundleDir -Recurse -File -Filter $pattern
        }
    }

    $artifacts += Get-Item $ExePath -ErrorAction SilentlyContinue
    return $artifacts | Sort-Object FullName -Unique
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
    $artifacts = @(Get-ReleaseArtifacts)
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
        [bool]$IsDraft,
        [bool]$IsPrerelease
    )

    Assert-Command "gh"

    $artifactPaths = @(Get-ReleaseArtifacts | ForEach-Object { $_.FullName })
    $artifactPaths += $ChecksumsPath

    $releaseExists = $false
    $null = & gh release view $ReleaseTag 2>$null
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
            npm run tauri -- build --target $Target
        }
    }

    Invoke-Step "Verifying release executable subsystem" {
        Test-WindowsGuiSubsystem $ExePath
    }

    Invoke-Step "Writing release checksums" {
        New-ReleaseChecksumFile | Out-Null
    }

    if ($Publish -and -not $SkipRelease) {
        Invoke-Step "Publishing GitHub Release $releaseTag" {
            Publish-GitHubRelease -ReleaseTag $releaseTag -IsDraft ([bool]$Draft) -IsPrerelease ([bool]$Prerelease)
        }
    }

    Write-Host ""
    Write-Host "Release build completed for $releaseTag."
}
finally {
    Pop-Location
}
