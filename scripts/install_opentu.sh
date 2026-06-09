#!/usr/bin/env bash
set -euo pipefail

REPO="${OPENTU_REPO:-AiW520/opentu}"
TAG="${OPENTU_TAG:-latest}"
FORMAT="${OPENTU_FORMAT:-appimage}"
TMP_DIR="$(mktemp -d)"
MOUNT_DIR=""
RESOLVED_TAG=""

cleanup() {
  if [[ -n "$MOUNT_DIR" && -d "$MOUNT_DIR" ]]; then
    hdiutil detach "$MOUNT_DIR" -quiet 2>/dev/null || true
    rm -rf "$MOUNT_DIR"
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

OS="$(uname -s)"
ARCH="$(uname -m)"
CANDIDATES=()

add_asset_candidate() {
  CANDIDATES+=("$1")
}

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64)
        add_asset_candidate "Opentu-macos-aarch64.dmg"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-macos-aarch64-${TAG}.dmg"
        ;;
      x86_64)
        add_asset_candidate "Opentu-macos-x86_64.dmg"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-macos-x86_64-${TAG}.dmg"
        ;;
      *)             echo "Unsupported macOS arch: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    FORMAT="$(printf '%s' "$FORMAT" | tr '[:upper:]' '[:lower:]')"
    case "$ARCH:$FORMAT" in
      x86_64:appimage|amd64:appimage)
        add_asset_candidate "Opentu-linux-x86_64.AppImage"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-linux-x86_64-${TAG}.AppImage"
        ;;
      aarch64:appimage|arm64:appimage)
        add_asset_candidate "Opentu-linux-aarch64.AppImage"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-linux-aarch64-${TAG}.AppImage"
        ;;
      x86_64:deb|amd64:deb)
        add_asset_candidate "Opentu-linux-x86_64.deb"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-linux-amd64-${TAG}.deb"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-linux-x86_64-${TAG}.deb"
        ;;
      x86_64:rpm|amd64:rpm)
        add_asset_candidate "Opentu-linux-x86_64.rpm"
        [[ "$TAG" != "latest" ]] && add_asset_candidate "Opentu-linux-x86_64-${TAG}.rpm"
        ;;
      *:deb|*:rpm)
        echo "Unsupported Linux arch for $FORMAT: $ARCH" >&2
        exit 1
        ;;
      *)
        echo "Unsupported Linux arch or format: arch=$ARCH format=$FORMAT" >&2
        echo "Supported OPENTU_FORMAT values: appimage, deb, rpm" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

resolve_latest_tag() {
  local latest_url final_url tag

  [[ "$TAG" == "latest" ]] || return 0
  [[ -n "$RESOLVED_TAG" ]] && return 0
  command -v curl >/dev/null 2>&1 || return 0

  latest_url="https://github.com/${REPO}/releases/latest"
  final_url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$latest_url" 2>/dev/null || true)"
  tag="${final_url##*/}"

  if [[ -n "$tag" && "$tag" != "latest" && "$tag" != "$final_url" ]]; then
    RESOLVED_TAG="$tag"
  fi
}

asset_urls() {
  local file="$1"
  if [[ "$TAG" == "latest" ]]; then
    printf 'https://github.com/%s/releases/latest/download/%s\n' "$REPO" "$file"
    resolve_latest_tag
    if [[ -n "$RESOLVED_TAG" ]]; then
      printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPO" "$RESOLVED_TAG" "$file"
    fi
  else
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPO" "$TAG" "$file"
  fi
}

download_to_file() {
  local url="$1"
  local output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 5 --retry-delay 2 --retry-max-time 120 --connect-timeout 15 "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget --tries=5 --waitretry=2 -O "$output" "$url"
  else
    echo "curl or wget is required to download Opentu." >&2
    exit 1
  fi
}

FILE=""
for candidate in "${CANDIDATES[@]}"; do
  echo "Downloading $candidate ..."
  while IFS= read -r DOWNLOAD_URL; do
    [[ -n "$DOWNLOAD_URL" ]] || continue
    if download_to_file "$DOWNLOAD_URL" "$TMP_DIR/$candidate"; then
      FILE="$candidate"
      break
    fi
    rm -f "$TMP_DIR/$candidate"
  done < <(asset_urls "$candidate")

  [[ -n "$FILE" ]] && break
done

if [[ -z "$FILE" ]]; then
  echo "Could not find a matching Opentu release asset." >&2
  echo "Repository: $REPO" >&2
  echo "Tag: $TAG" >&2
  echo "Tried:" >&2
  for candidate in "${CANDIDATES[@]}"; do
    while IFS= read -r DOWNLOAD_URL; do
      [[ -n "$DOWNLOAD_URL" ]] || continue
      echo "  - $DOWNLOAD_URL" >&2
    done < <(asset_urls "$candidate")
  done
  exit 1
fi

case "$(echo "$FILE" | tr '[:upper:]' '[:lower:]')" in
  *.dmg)
    MOUNT_DIR="$(mktemp -d)"
    hdiutil attach "$TMP_DIR/$FILE" -mountpoint "$MOUNT_DIR" -nobrowse -quiet
    app_path=""
    for app in "$MOUNT_DIR"/*.app; do
      [[ -e "$app" ]] || continue
      app_path="$app"
      break
    done
    if [[ -z "${app_path:-}" ]]; then
      echo "No .app found in DMG" >&2
      exit 1
    fi
    dest_dir="${OPENTU_INSTALL_DIR:-/Applications}"
    [[ ! -w "$dest_dir" ]] && dest_dir="$HOME/Applications"
    mkdir -p "$dest_dir"
    app_name="$(basename "$app_path")"
    rm -rf "$dest_dir/$app_name"
    ditto "$app_path" "$dest_dir/$app_name"
    hdiutil detach "$MOUNT_DIR" -quiet || true
    rm -rf "$MOUNT_DIR"
    MOUNT_DIR=""
    xattr -dr com.apple.quarantine "$dest_dir/$app_name" 2>/dev/null || true
    echo "Installed to $dest_dir/$app_name"
    ;;
  *.appimage)
    install_root="$HOME/.local/share/opentu"
    bin_root="$HOME/.local/bin"
    install_path="$install_root/opentu.AppImage"
    mkdir -p "$install_root" "$bin_root"
    mv "$TMP_DIR/$FILE" "$install_path"
    chmod +x "$install_path"
    ln -sf "$install_path" "$bin_root/opentu"
    echo "Installed to $install_path"
    echo "Symlink created at $bin_root/opentu"
    case ":$PATH:" in
      *":$bin_root:"*) ;;
      *) echo "Add $bin_root to PATH to run: opentu" ;;
    esac
    ;;
  *.deb|*.rpm)
    download_dir="${OPENTU_DOWNLOAD_DIR:-$HOME/Downloads}"
    mkdir -p "$download_dir"
    mv "$TMP_DIR/$FILE" "$download_dir/$FILE"
    echo "Downloaded to $download_dir/$FILE"
    ;;
esac
