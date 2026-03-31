#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage:
  install-linux-browser.sh detect
  install-linux-browser.sh install <browser|all> <dev|beta|prod> <extension-id> <binary-path>

supported browsers:
  chrome
  chromium
  opera
  vivaldi
  brave
EOF
}

browser_dir() {
  case "$1" in
    chrome) echo "${HOME}/.config/google-chrome/NativeMessagingHosts" ;;
    chromium) echo "${HOME}/.config/chromium/NativeMessagingHosts" ;;
    opera) echo "${HOME}/.config/opera/NativeMessagingHosts" ;;
    vivaldi) echo "${HOME}/.config/vivaldi/NativeMessagingHosts" ;;
    brave) echo "${HOME}/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts" ;;
    *) return 1 ;;
  esac
}

supported_browsers() {
  printf '%s\n' chrome chromium opera vivaldi brave
}

host_wrapper_dir() {
  echo "${HOME}/.local/share/keystone/native-hosts"
}

detect_browsers() {
  local found=0
  while IFS= read -r browser; do
    local dir
    dir="$(browser_dir "$browser")"
    local browser_root
    browser_root="$(dirname "$dir")"
    if [ -d "$browser_root" ]; then
      printf '%s\t%s\n' "$browser" "$dir"
      found=1
    fi
  done < <(supported_browsers)

  if [ "$found" -eq 0 ]; then
    echo "no supported Chromium-family browser config directories detected" >&2
    return 1
  fi
}

host_id_for_flavor() {
  case "$1" in
    dev) echo "com.ytxt.keystone.dev" ;;
    beta) echo "com.ytxt.keystone.beta" ;;
    prod) echo "com.ytxt.keystone" ;;
    *) return 1 ;;
  esac
}

install_one() {
  local browser="$1"
  local flavor="$2"
  local extension_id="$3"
  local binary_path="$4"

  local manifest_dir
  manifest_dir="$(browser_dir "$browser")"
  mkdir -p "$manifest_dir"

  local host_id
  host_id="$(host_id_for_flavor "$flavor")"
  local manifest_path="${manifest_dir}/${host_id}.json"
  local wrapper_dir
  wrapper_dir="$(host_wrapper_dir)"
  mkdir -p "$wrapper_dir"
  local wrapper_path="${wrapper_dir}/${host_id}"

  cat > "$wrapper_path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
export KEYSTONE_FLAVOR="${flavor}"
exec "${binary_path}" "\$@"
EOF
  chmod +x "$wrapper_path"

  cargo run --bin keystone-dev -- manifest "$flavor" "$wrapper_path" "$extension_id" > "$manifest_path"

  echo "installed ${browser}: ${manifest_path}"
  echo "wrapper: ${wrapper_path}"
}

if [ "$#" -lt 1 ]; then
  usage
  exit 1
fi

command="$1"
shift

case "$command" in
  detect)
    detect_browsers
    ;;
  install)
    if [ "$#" -ne 4 ]; then
      usage
      exit 1
    fi

    target="$1"
    flavor="$2"
    extension_id="$3"
    binary_path="$4"

    case "$flavor" in
      dev|beta|prod) ;;
      *)
        echo "invalid flavor: $flavor" >&2
        exit 1
        ;;
    esac

    if [ "$target" = "all" ]; then
      while IFS=$'\t' read -r browser _dir; do
        install_one "$browser" "$flavor" "$extension_id" "$binary_path"
      done < <(detect_browsers)
      exit 0
    fi

    if ! browser_dir "$target" >/dev/null 2>&1; then
      echo "unsupported browser: $target" >&2
      exit 1
    fi

    install_one "$target" "$flavor" "$extension_id" "$binary_path"
    ;;
  *)
    usage
    exit 1
    ;;
esac
