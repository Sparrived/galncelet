# Release Playbook

## Fast path through GitHub Actions

1. Make sure `master` is clean and pushed.
2. Create and push a semantic version tag:

```powershell
git tag v1.2.3
git push origin v1.2.3
```

3. GitHub Actions runs `.github/workflows/release.yml` automatically.
4. The workflow builds normal and offline Windows installers, verifies checksums, and publishes the GitHub Release.

## Manual GitHub Actions release

Use this when the tag already exists or you want a draft/prerelease:

- open the `Release` workflow in GitHub Actions
- choose `Run workflow`
- enter the tag, such as `v1.2.3`
- choose `draft` and `prerelease` as needed

## Local release build

Local builds use the same script as Actions:

```powershell
npm run release -- -Version 1.2.3 -Tag v1.2.3
npm run release:offline -- -Version 1.2.3 -Tag v1.2.3
npm run release -- -Version 1.2.3 -Tag v1.2.3 -SkipBuild
```

The final `-SkipBuild` refreshes the normal checksum file after the offline build has generated `*-offline` artifacts.

## What gets produced

- `src-tauri/target/x86_64-pc-windows-msvc/release/galncelet.exe`
- `Galncelet_<version>_x64-setup.exe`
- `Galncelet_<version>_x64_en-US.msi`
- `Galncelet_<version>_x64-setup-offline.exe`
- `Galncelet_<version>_x64_en-US-offline.msi`
- `SHA256SUMS.txt`
- `SHA256SUMS-offline.txt`
- a GitHub Release with generated notes
