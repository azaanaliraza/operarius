# Release Signing Checklist

You can release without any secrets. Secrets are only needed for signed and notarized distribution.

## 0. No-secrets release (works now)

- Push a version tag and the workflow will build release artifacts for macOS and Windows.
- This path does not require Apple or Tauri signing secrets.
- Output installers may show OS security prompts because they are unsigned.

## 1. Apple signing assets

- Create a Developer ID Application certificate in Apple Developer.
- Export certificate as `.p12` from Keychain.
- Base64 encode the `.p12` and store it in GitHub secret `APPLE_CERTIFICATE`.
- Store the `.p12` export password in `APPLE_CERTIFICATE_PASSWORD`.
- Set `APPLE_SIGNING_IDENTITY` to the exact identity name, for example:
  - `Developer ID Application: Your Name (TEAMID)`

## 2. Apple notarization

- Use Apple account credentials with app-specific password.
- Set:
  - `APPLE_ID`
  - `APPLE_PASSWORD`
  - `APPLE_TEAM_ID`

## 3. Tauri updater signing (optional but recommended)

- Generate updater keypair.
- Store private key in `TAURI_SIGNING_PRIVATE_KEY`.
- Store private key password in `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## 4. Trigger release

- Tag-based release:
  - `git tag v0.1.0`
  - `git push origin v0.1.0`
- Or run GitHub Actions workflow `Release Desktop` manually.

## 5. Add GitHub secrets (optional, for signed distribution)

1. Open repository Settings.
2. Go to Secrets and variables > Actions.
3. Click New repository secret.
4. Add each secret name/value from this checklist.
5. Repeat until all required secrets are present.

Optional secret names:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## 6. Verify outputs

- GitHub Release contains macOS Intel, macOS Apple Silicon, and Windows artifacts.
- If you enabled signing/notarization, verify on macOS:
  - `spctl -a -vv /Applications/Operarius.app`

## 7. Note on Windows resources

- The release workflow uses `src-tauri/tauri.windows.conf.json` for Windows builds.
- This avoids bundling mac-only `.dylib` files and `llama-server` binary during Windows packaging.
