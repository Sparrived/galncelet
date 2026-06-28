# Release

This project ships Windows releases through `scripts/release.ps1` and `.github/workflows/release.yml`.

## Local build

```powershell
npm run release
```

The script runs `npm ci`, builds the Tauri release bundle, verifies that `src-tauri/target/release/galncelet.exe` uses the Windows GUI subsystem, and writes `SHA256SUMS.txt` next to the bundle artifacts.

Release builds do not open a console window because `src-tauri/src/main.rs` sets:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

The script verifies this at the PE header level and fails if the executable is linked as a console app.

## Local GitHub Release publish

Authenticate GitHub CLI first:

```powershell
gh auth login
```

Build and publish a release:

```powershell
npm run release:publish -- -Tag v0.1.0
```

Useful options:

```powershell
npm run release:publish -- -Version 0.1.1 -Tag v0.1.1 -Draft
npm run release:publish -- -Tag v0.2.0-beta.1 -Prerelease
npm run release -- -AllowDirty
```

`-Version` updates `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` before building.

## GitHub Actions release

Push a version tag to build and publish from GitHub Actions:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

The workflow can also be started manually from the GitHub Actions UI with an existing tag.
