# Operarius

Operarius is a Tauri desktop app with a React and TypeScript frontend, backed by a Rust command layer and native packaging for macOS and Windows. It is designed to run locally, ship as a desktop installer, and bundle the runtime assets it needs without requiring a separate server.

## What It Does

Operarius provides a desktop shell for local and remote model workflows, onboarding, dashboards, chat surfaces, Telegram setup, and native integration with bundled sidecars and binaries. The current app branding uses the Panther icon set throughout the web and native layers.

## Tech Stack

- Frontend: React 19, Vite 7, TypeScript 5.8
- Desktop shell: Tauri v2
- Styling and utilities: Tailwind CSS v4, Lucide icons, Zustand
- Package manager: Bun for local development and release CI
- Native layer: Rust via `src-tauri`

## Repository Layout

- `src/` - React application source
- `src/assets/` - app artwork and branding assets
- `src-tauri/` - Tauri configuration, Rust commands, icons, and sidecars
- `.github/workflows/` - release automation
- `docs/` - release and signing notes

## Prerequisites

Install these before working on the app:

- Bun
- Rust toolchain
- A supported Tauri development environment for your platform
- VS Code with the Tauri and rust-analyzer extensions if you want the best editor support

## Local Development

Install dependencies:

```bash
bun install
```

Run the web frontend only:

```bash
bun run dev
```

Run the desktop app:

```bash
bun run tauri:dev
```

The Tauri build hooks use `bun run dev` before development and `bun run build` before packaging, as defined in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json).

## Building

Build the frontend only:

```bash
bun run build
```

Build a desktop release locally from the Tauri project:

```bash
cd src-tauri
bun tauri build
```

Release builds target macOS and Windows and bundle the configured native icons, resources, and sidecars.

## Release Process

The automated release workflow lives in [.github/workflows/release-desktop.yml](.github/workflows/release-desktop.yml).

It runs on tagged pushes and manual dispatches, then publishes GitHub Release assets for:

- macOS Intel (`x86_64-apple-darwin`)
- macOS Apple Silicon (`aarch64-apple-darwin`)
- Windows x64 (`x86_64-pc-windows-msvc`)

Each release now includes:

- End-user installer assets (`.dmg`, `.msi`, and `.exe` when available)
- A `SHA256SUMS.txt` file for integrity verification
- A release note file with install and verification guidance

To cut a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

After the workflow finishes, download the generated installers from the GitHub Release page.

Recommended installer choice:

- macOS: use the `.dmg` matching your architecture
- Windows: use `.msi` for most users

Checksum verification examples:

```bash
shasum -a 256 -c SHA256SUMS.txt
```

```powershell
Get-FileHash .\Operarius-setup.msi -Algorithm SHA256
```

## Signing And Secrets

Unsigned builds work without any secrets. That is the default release path.

If you want signed and notarized macOS builds, configure the optional GitHub secrets below:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Without those secrets, the workflow still produces release artifacts, but macOS binaries will not be signed or notarized.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution workflow, commit conventions, validation steps, and pull request expectations.

## Troubleshooting

- If the dev server port is already in use, stop the conflicting process and restart `bun run tauri:dev`.
- If native icons look stale, rebuild from the Panther assets in `src/assets/` and verify the generated Tauri icon set in `src-tauri/icons/`.
- If a release job fails on dependency installation, make sure `bun.lock` is up to date and committed.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

Refer to the repository files for licensing details.
