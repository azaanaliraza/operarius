# Contributing to Operarius

Thanks for helping improve Operarius. This guide covers the expected workflow for changes, validation, and pull requests.

## Scope

This repository contains the React frontend, the Tauri native shell, release automation, and bundled sidecars. Keep changes focused and avoid unrelated formatting or refactors unless they support the task at hand.

## Development Setup

1. Install dependencies with `bun install`.
2. Run the web app with `bun run dev`.
3. Run the desktop app with `bun run tauri:dev`.
4. Make sure the Rust toolchain and Tauri prerequisites are installed for your platform.

## Before You Open A PR

Validate the changes locally before requesting review:

- Confirm the app starts in development mode.
- Build the frontend with `bun run build`.
- If you touched Tauri code, run a desktop build from `src-tauri`.
- If you changed release automation, verify the workflow syntax and artifact paths.

## Code Style

- Match the existing code style and file organization.
- Prefer small, readable changes over broad rewrites.
- Keep UI work consistent with the established Operarius branding and typography.
- Do not introduce new dependencies unless they are clearly justified.

## Commit And PR Expectations

- Write commit messages that describe the actual change.
- Reference user-visible behavior when relevant.
- Include testing notes in the pull request description.
- Mention any release or signing impact explicitly.

## Release Changes

If your change affects packaging, signing, or GitHub Actions:

- Verify the workflow still works for macOS and Windows.
- Check that `bun.lock` remains in sync if dependency resolution changes.
- Ensure the release documentation matches the current workflow behavior.

## Reporting Issues

When reporting a bug or regression, include:

- What you expected to happen
- What actually happened
- Your operating system and relevant versions
- Any terminal output, logs, or screenshots that help reproduce the issue

## Secrets And Signing

Unsigned builds are supported. Only add signing or notarization secrets if you need trusted distribution. If you do, follow the release guidance in the README and keep secrets out of version control.

## Questions

If something is unclear, open an issue or describe the problem in the pull request so the next maintainer has enough context to review it quickly.