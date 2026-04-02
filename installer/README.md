# Installer Notes

Keystone release artifacts now ship with helper scripts for the supported release flows:

- `install-keystone-linux.sh`
- `install-keystone-macos.sh`
- `install-keystone-windows.ps1`

That helper is meant for the extracted GitHub Release folder. It:

- copies the released `keystone` binary into a stable per-user location
- ensures it is executable
- keeps the chosen flavor in a stable subdirectory
- runs `keystone install ...` for the chosen browser target and extension ID

Example:

```bash
./install-keystone-linux.sh chrome prod your_extension_id
```

Optional custom install root:

```bash
./install-keystone-linux.sh brave prod your_extension_id ~/.local/opt/keystone
```

macOS example:

```bash
./install-keystone-macos.sh chrome prod your_extension_id
```

Windows PowerShell example:

```powershell
.\install-keystone-windows.ps1 chrome prod your_extension_id
```

The first real polished installer still needs to go further:

- handle code signing, notarization, and Windows SmartScreen reputation
- run a post-install smoke test that validates `bridge.hello`
- ensure the launched host process knows its intended flavor; the current installers do this via a small wrapper next to the Native Messaging manifest target

Current example host id:

- `com.ytxt.keystone`

Flavor-separated manifests included:

- `com.ytxt.keystone.dev`
- `com.ytxt.keystone.beta`
- `com.ytxt.keystone`

Rule:

- install exactly one manifest for the chosen flavor
- do not mix dev, beta, and prod IDs in one manifest

Quick local Linux Chromium-browser path:

```bash
cargo run --bin keystone -- detect
cargo run --bin keystone -- install chrome dev yourdevextensionid /absolute/path/to/target/debug/keystone
```

Supported browser targets vary by OS:

- Linux: `chrome`, `chromium`, `brave`, `opera`, `vivaldi`
- macOS: `chrome`, `chromium`, `brave`, `vivaldi`
- Windows: `chrome`, `chromium`, `brave`, `vivaldi` (pending live smoke test)

The older shell script remains available, but the main `keystone` binary is now the preferred entry point for local install and status flows.

On Linux and macOS this installs:

- the browser manifest into the browser's `NativeMessagingHosts` directory
- a flavor-specific wrapper into the per-user Keystone data directory

On Windows this installs:

- the browser manifest into Keystone's per-user support directory
- the required browser registry key pointing to that manifest
- a flavor-specific `.cmd` wrapper in the per-user Keystone data directory

The wrapper is the manifest target and launches the real binary with the correct `KEYSTONE_FLAVOR`.
