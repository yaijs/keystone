# Keystone v0.1.0

Keystone is a browser-extension-first local credential proxy for stronger handling of provider API keys than plain extension storage.

## Highlights

- Native Messaging bridge for local extension-to-host communication
- Flavor-scoped trust domains for `dev`, `beta`, and `prod`
- OS keyring-backed provider secret storage
- Ephemeral localhost session tunnel for authenticated upstream provider requests
- Built-in local admin UI at `/admin` with JSON status and secret-management endpoints
- First release-facing CLI commands for `status`, `detect`, `install`, and `serve`

Important limits:

- Keystone improves secret handling; it does not eliminate trust risk
- secrets are still present in Keystone process memory during active use
- localhost transport is not itself a trust boundary

## Setup And Admin

- Linux Chromium-family browser detection and manifest installation from the main `keystone` CLI
- Flavor-specific native-host manifests and wrapper handling for browser integration
- Standalone local admin/server mode via `keystone serve`
- Dev harness commands for smoke testing, manifest generation, and runtime inspection

## Platform Scope

- Supported now: Linux with Chrome, Chromium, Brave, Opera, and Vivaldi; helper-based macOS install flow; helper-based Windows flow pending live smoke test
- Planned later: tighter threat-model docs, approval UX, uninstall/recovery polish, and packaging/signing improvements

## Packaging

- GitHub Actions release workflow included for Linux, macOS, and Windows build artifacts
- Planned release assets are downloadable archives plus SHA-256 checksum files
- Browser integration remains an explicit local install step rather than a silent system-wide installer
