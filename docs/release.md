# Release

This project uses one release script plus one GitHub Actions workflow for repeatable local and CI releases.

## Before you release

- make sure `master` is clean and up to date
- update versions in `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock` when cutting a new semantic version
- confirm the release tag is `v<version>`
- verify GitHub Actions can use `GITHUB_TOKEN` with `contents: write`

## GitHub Actions release

The workflow at `.github/workflows/release.yml` is the canonical release path.

It runs on:

- pushing a tag like `v1.2.3`
- manual `workflow_dispatch` with a tag, draft flag, and prerelease flag

For each release, the workflow:

1. resolves `tag` and `version` from the event
2. installs Node and Rust on `windows-latest`
3. runs `scripts/release.ps1` for the normal installer build
4. runs `scripts/release.ps1 -Offline` for the offline WebView2 installer build
5. recomputes normal checksums after the offline build
6. verifies expected artifacts and checksum files
7. creates or updates the GitHub Release with generated notes

## Local release build

Normal installer:

```powershell
npm run release -- -Version 1.2.3 -Tag v1.2.3
```

Offline installer with embedded WebView2 runtime installer:

```powershell
npm run release:offline -- -Version 1.2.3 -Tag v1.2.3
```

Refresh normal checksums after building offline artifacts:

```powershell
npm run release -- -Version 1.2.3 -Tag v1.2.3 -SkipBuild
```

Publish from a local machine with `gh` configured:

```powershell
npm run release:publish -- -Version 1.2.3 -Tag v1.2.3
```

## Artifact layout

The release produces:

- `src-tauri/target/x86_64-pc-windows-msvc/release/galncelet.exe`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/Galncelet_<version>_x64_en-US.msi`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Galncelet_<version>_x64-setup.exe`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/Galncelet_<version>_x64_en-US-offline.msi`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Galncelet_<version>_x64-setup-offline.exe`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/SHA256SUMS.txt`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/SHA256SUMS-offline.txt`

`SHA256SUMS.txt` covers the normal installers and standalone exe. `SHA256SUMS-offline.txt` covers the offline installers.
