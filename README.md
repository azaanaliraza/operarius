# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Release Builds (macOS + Windows)

- Workflow file: `.github/workflows/release-desktop.yml`
- Triggers:
	- Push a tag like `v0.1.0`
	- Manual run via GitHub Actions (`workflow_dispatch`)
- Output: GitHub Release artifacts for:
	- macOS Intel (`x86_64-apple-darwin`)
	- macOS Apple Silicon (`aarch64-apple-darwin`)
	- Windows (`x86_64-pc-windows-msvc`)

### How to cut a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

After the workflow completes, download installers from the GitHub Release page.

### GitHub Secrets for trusted distribution

Set these repository secrets before releasing signed builds:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Without Apple secrets, macOS artifacts still build but will not be signed/notarized.
