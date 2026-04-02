# Packaging Plan

This document defines the first practical packaging target for Keystone.

## Goal

Make Keystone usable by normal users without requiring them to build Rust locally.

The first acceptable packaging level is:

- downloadable release artifacts on GitHub Releases
- one `keystone` binary artifact per OS
- checksums for every artifact
- one predictable helper-based install/integration story per OS
- browser integration still handled by the built-in `keystone install ...` command or the bundled helper scripts

This is intentionally not the final polished installer story.
It is the first honest release shape for a browser-extension-first local credential proxy.

## Release Artifacts

Initial release artifacts:

- Linux: `keystone-linux-x86_64.tar.gz`
- macOS: `keystone-macos-x86_64.tar.gz`
- Windows: `keystone-windows-x86_64.zip`

Each artifact should include:

- the `keystone` binary
- `README.md`
- `installer/README.md` as quick integration reference
- Linux should include `install-keystone-linux.sh`
- macOS should include `install-keystone-macos.sh`
- Windows should include `install-keystone-windows.ps1`
- a `.sha256` checksum file

## First User Flow

1. Download the correct release artifact for the OS.
2. Extract it to a known location.
3. Prefer the bundled OS-specific helper script from the extracted folder.
4. Otherwise run the binary directly or move it into a stable per-user location.
5. Connect it to the current browser:

```bash
keystone install <browser> <dev|beta|prod> <extension-id> <path-to-keystone-binary>
```

6. Open the client extension and test the Keystone connection.

## Intended Stable Binary Locations

These are the recommended support targets for documentation and future installers.

### Linux

- preferred user install location: `~/.local/bin/keystone`
- first release form: extracted binary from GitHub Release
- later improvement: `.deb` and/or AppImage

### macOS

- preferred first release form: extracted binary plus `install-keystone-macos.sh`
- later improvement: signed and notarized app/bundle

### Windows

- preferred first release form: extracted `keystone.exe` plus `install-keystone-windows.ps1`
- later improvement: MSI or installer package

## Current Scope

The current release automation still does not solve:

- code signing
- notarization
- OS-native installers
- automatic PATH registration
- automatic browser integration during install

Those are second-stage packaging improvements.

## Unified Product Shape

The unified part of Keystone should remain:

- one Rust binary named `keystone`
- one built-in admin UI
- one CLI surface:
  - `keystone serve`
  - `keystone status`
  - `keystone detect`
  - `keystone install ...`

Only the outer distribution layer should vary per OS.
Keystone should not promise more packaging maturity than it has actually tested.

## Release Pipeline

The first GitHub Actions release workflow should:

- trigger on version tags like `v0.1.0`
- build release binaries on Linux, macOS, and Windows
- archive the binaries in OS-specific release artifacts
- generate SHA-256 checksums
- publish everything to GitHub Releases

This repo now includes that first workflow in `.github/workflows/release.yml`.

## Next Packaging Steps

1. Publish the first tagged GitHub Release with downloadable artifacts.
2. Keep client integration docs aligned with downloadable release binaries instead of source builds.
3. Verify helper install flows with real smoke tests per OS.
4. Later add signed/native installers for macOS and Windows.
