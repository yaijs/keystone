#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage:
  install-keystone-macos.sh <browser|all> <dev|beta|prod> <extension-id> [install-dir]

examples:
  ./install-keystone-macos.sh chrome prod abcdefghijklmnopabcdefghijklmnop
  ./install-keystone-macos.sh brave prod abcdefghijklmnopabcdefghijklmnop "$HOME/Library/Application Support/Keystone"

notes:
  - run this script from the extracted GitHub Release folder
  - the script installs the binary into a stable per-user location
  - then it runs `keystone install ...` for the chosen browser target
  - on first launch, macOS Gatekeeper may require approval for the extracted binary
EOF
}

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
  usage
  exit 1
fi

browser="$1"
flavor="$2"
extension_id="$3"
install_root="${4:-$HOME/Library/Application Support/Keystone}"

case "$flavor" in
  dev|beta|prod) ;;
  *)
    echo "invalid flavor: $flavor" >&2
    usage
    exit 1
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_binary="${script_dir}/keystone"

if [ ! -f "$source_binary" ]; then
  echo "keystone binary not found next to this script: $source_binary" >&2
  exit 1
fi

mkdir -p "$install_root"
target_dir="${install_root}/${flavor}"
mkdir -p "$target_dir"

target_binary="${target_dir}/keystone"
cp "$source_binary" "$target_binary"
chmod +x "$target_binary"

if command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.quarantine "$target_binary" >/dev/null 2>&1 || true
fi

if [ -f "${script_dir}/README.md" ]; then
  cp "${script_dir}/README.md" "${target_dir}/README.md"
fi

if [ -f "${script_dir}/INSTALLER.md" ]; then
  cp "${script_dir}/INSTALLER.md" "${target_dir}/INSTALLER.md"
fi

echo "installed binary: $target_binary"
echo "registering browser manifest for target: $browser"
"$target_binary" install "$browser" "$flavor" "$extension_id" "$target_binary"
echo "done"
echo "next step: reload the extension and click 'Test Keystone Connection' in Y/TXT Options."
