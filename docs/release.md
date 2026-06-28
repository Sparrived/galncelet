# Release

This project now uses one reusable flow for local builds and GitHub Actions releases.

## Local release build

```powershell
npm run release
```

This performs the full build-and-verify flow:

- checks prerequisites
- validates the working tree unless `-AllowDirty` is passed
- optionally syncs the version into `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`
- installs locked dependencies with `npm ci`
- builds the Windows Tauri release bundle
- verifies the release binary is linked as a Windows GUI app
- writes `SHA256SUMS.txt` next to the bundle artifacts

To build with a specific version without publishing:

```powershell
npm run release -- -Version 0.1.0 -Tag v0.1.0
```

To publish an existing build from a machine with `gh` configured:

```powershell
npm run release:publish -- -Tag v0.1.0
```

## GitHub Actions release

The workflow at `.github/workflows/release.yml` now supports both tag pushes and manual dispatch:

- pushing a tag like `v0.1.0` builds the artifacts and publishes the release
- running the workflow manually can reuse an existing tag and optionally publish, draft, or mark the release as prerelease

## Artifact layout

The release build always produces:

- `src-tauri/target/release/galncelet.exe`
- the Tauri bundle directory under `src-tauri/target/release/bundle/`
- `SHA256SUMS.txt` for checksum verification

The release script is the single entry point for local verification and for the GitHub Actions build step, so future releases can reuse the same logic without duplicating steps.
