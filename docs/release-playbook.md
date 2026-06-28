# Release Playbook

## Fast path

1. Update the version in `package.json` if needed.
2. Make sure `src/addons` is pushed first when it changed.
3. Run the local release build:

```powershell
npm run release -- -Version 1.0.0 -Tag v1.0.0
```

4. If the build looks good, create the tag and push it:

```powershell
git tag v1.0.0
git push origin v1.0.0
```

5. GitHub Actions will build the release, verify artifacts, and publish the GitHub Release automatically.

## Manual GitHub Actions release

Use this when the tag already exists and you want to reuse it:

- open the `Release` workflow in GitHub Actions
- choose `Run workflow`
- enter the existing tag, such as `v1.0.0`
- keep `publish` enabled if you want the workflow to create or update the release
- set `draft` or `prerelease` as needed

## What gets produced

- `src-tauri/target/x86_64-pc-windows-msvc/release/galncelet.exe`
- the Tauri bundle under `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`
- `SHA256SUMS.txt`
- a GitHub Release with generated notes
