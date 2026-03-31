# Installer Notes

These files are examples only.

The first real installer must:

- place the Native Messaging manifest in the correct per-OS browser location
- write the actual installed binary path
- use the intended extension ID for the selected build flavor
- run a post-install smoke test that validates `bridge.hello`
- ensure the launched host process knows its intended flavor; on Linux the current installer does this via a small wrapper script that exports `KEYSTONE_FLAVOR`

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

Supported Linux browser targets currently include:

- `chrome`
- `chromium`
- `brave`
- `opera`
- `vivaldi`

The shell script remains available, but the main `keystone` binary is now the preferred entry point for local install and status flows.

On Linux this installs:

- the browser manifest into the browser's `NativeMessagingHosts` directory
- a flavor-specific wrapper into `~/.local/share/keystone/native-hosts/`

The wrapper is the manifest target and launches the real binary with the correct `KEYSTONE_FLAVOR`.
