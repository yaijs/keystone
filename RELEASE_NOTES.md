# Keystone v0.1.0

Keystone is a local native host for browser extensions that want stronger handling of provider API keys than plain extension storage.

## Highlights

- Native Messaging bridge for local extension-to-host communication
- Flavor-scoped trust domains for `dev`, `beta`, and `prod`
- OS keyring-backed provider secret storage
- Ephemeral localhost session tunnel for authenticated upstream provider requests
- Built-in local admin UI at `/admin` with JSON status and secret-management endpoints
- First release-facing CLI commands for `status`, `detect`, `install`, and `serve`

## Setup And Admin

- Linux Chromium-family browser detection and manifest installation from the main `keystone` CLI
- Flavor-specific native-host manifests and wrapper handling for browser integration
- Standalone local admin/server mode via `keystone serve`
- Dev harness commands for smoke testing, manifest generation, and runtime inspection

## Platform Scope

- Supported now: Linux with Chrome, Chromium, Brave, Opera, and Vivaldi
- Planned later: polished macOS and Windows install flows, approval UX, and packaging/signing improvements

## Packaging

- GitHub Actions release workflow included for Linux, macOS, and Windows build artifacts
- Planned release assets are downloadable archives plus SHA-256 checksum files
- Browser integration remains an explicit local install step rather than a silent system-wide installer
