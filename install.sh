#!/bin/sh
# install.sh — forgeplan installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ForgePlan/forgeplan/main/install.sh | sh
#
# Installs forgeplan to ~/.cargo/bin/forgeplan (or /usr/local/bin as fallback).
# Supports: Linux x86_64, macOS arm64, macOS x86_64.
# Requires: curl or wget.
#
# Security note: piping to sh executes unverified remote code.
# For verified install, use: cargo install forgeplan-cli
# Or download manually from https://github.com/ForgePlan/forgeplan/releases
#
# Exit codes:
#   0 — success
#   1 — unsupported OS or architecture
#   2 — download failed
#   3 — install failed

set -eu

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REPO="ForgePlan/forgeplan"
BASE_URL="https://github.com/${REPO}/releases/latest/download"
BINARY_NAME="forgeplan"

# ---------------------------------------------------------------------------
# Logging helpers
# ---------------------------------------------------------------------------

log_info() {
    printf '[info]  %s\n' "$1"
}

log_error() {
    printf '[error] %s\n' "$1" >&2
}

log_success() {
    printf '[ok]    %s\n' "$1"
}

# ---------------------------------------------------------------------------
# Detect OS
# ---------------------------------------------------------------------------

detect_os() {
    os="$(uname -s 2>/dev/null)" || {
        log_error "Cannot determine operating system (uname -s failed)"
        exit 1
    }
    case "$os" in
        Linux)  printf 'linux' ;;
        Darwin) printf 'macos' ;;
        *)
            log_error "Unsupported operating system: ${os}"
            log_error "forgeplan supports Linux and macOS only."
            exit 1
            ;;
    esac
}

# ---------------------------------------------------------------------------
# Detect architecture
# ---------------------------------------------------------------------------

detect_arch() {
    arch="$(uname -m 2>/dev/null)" || {
        log_error "Cannot determine CPU architecture (uname -m failed)"
        exit 1
    }
    case "$arch" in
        x86_64)         printf 'x86_64' ;;
        amd64)          printf 'x86_64' ;;
        aarch64)        printf 'aarch64' ;;
        arm64)          printf 'aarch64' ;;
        *)
            log_error "Unsupported CPU architecture: ${arch}"
            log_error "forgeplan supports x86_64 and aarch64/arm64 only."
            exit 1
            ;;
    esac
}

# ---------------------------------------------------------------------------
# Map OS + arch to release binary name
# ---------------------------------------------------------------------------

map_binary() {
    _os="$1"
    _arch="$2"

    case "${_os}/${_arch}" in
        linux/x86_64)   printf 'forgeplan-x86_64-unknown-linux-gnu' ;;
        macos/aarch64)  printf 'forgeplan-aarch64-apple-darwin' ;;
        macos/x86_64)   printf 'forgeplan-x86_64-apple-darwin' ;;
        *)
            log_error "No prebuilt binary for ${_os}/${_arch}."
            exit 1
            ;;
    esac
}

# ---------------------------------------------------------------------------
# Determine install directory (no sudo required)
# ---------------------------------------------------------------------------

choose_install_dir() {
    # Prefer ~/.cargo/bin if it exists or can be created
    cargo_bin="${HOME}/.cargo/bin"
    if [ -d "$cargo_bin" ] || mkdir -p "$cargo_bin" 2>/dev/null; then
        printf '%s' "$cargo_bin"
        return
    fi

    # Fall back to /usr/local/bin only if writable without sudo
    local_bin="/usr/local/bin"
    if [ -w "$local_bin" ]; then
        printf '%s' "$local_bin"
        return
    fi

    log_error "Cannot find a writable install directory."
    log_error "Tried: ${cargo_bin}, ${local_bin}"
    log_error "Please create ${cargo_bin} and re-run, or add it to PATH."
    exit 3
}

# ---------------------------------------------------------------------------
# Download with curl, falling back to wget
# ---------------------------------------------------------------------------

download() {
    _url="$1"
    _dest="$2"

    if command -v curl >/dev/null 2>&1; then
        log_info "Downloading with curl..."
        curl --fail --silent --show-error --location \
             --retry 3 --retry-delay 2 \
             --output "$_dest" "$_url" || {
            log_error "curl download failed for: ${_url}"
            exit 2
        }
    elif command -v wget >/dev/null 2>&1; then
        log_info "Downloading with wget..."
        wget --quiet --tries=3 --output-document="$_dest" "$_url" || {
            log_error "wget download failed for: ${_url}"
            exit 2
        }
    else
        log_error "Neither curl nor wget is available."
        log_error "Please install curl or wget and re-run."
        exit 2
    fi
}

# ---------------------------------------------------------------------------
# Cleanup trap
# ---------------------------------------------------------------------------

# tmpfile is set later; define trap before creating the file
tmpfile=""
cleanup() {
    if [ -n "$tmpfile" ] && [ -f "$tmpfile" ]; then
        rm -f -- "$tmpfile"
    fi
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    log_info "Detecting platform..."
    detected_os="$(detect_os)"
    detected_arch="$(detect_arch)"
    log_info "  OS:   ${detected_os}"
    log_info "  Arch: ${detected_arch}"

    release_binary="$(map_binary "$detected_os" "$detected_arch")"
    download_url="${BASE_URL}/${release_binary}"
    log_info "  Binary: ${release_binary}"

    install_dir="$(choose_install_dir)"
    install_path="${install_dir}/${BINARY_NAME}"
    log_info "  Install path: ${install_path}"

    # Create a secure temporary file for the download
    tmpfile="$(mktemp)" || {
        log_error "Failed to create temporary file."
        exit 2
    }

    log_info "Downloading forgeplan from GitHub..."
    log_info "  URL: ${download_url}"
    download "$download_url" "$tmpfile"

    # Verify the download produced a non-empty file
    if [ ! -s "$tmpfile" ]; then
        log_error "Downloaded file is empty. The release binary may not exist yet."
        log_error "Check: https://github.com/${REPO}/releases"
        exit 2
    fi

    # Move binary into place
    mkdir -p -- "$install_dir" 2>/dev/null || true
    cp -- "$tmpfile" "$install_path" || {
        log_error "Failed to copy binary to ${install_path}"
        exit 3
    }
    chmod +x -- "$install_path" || {
        log_error "Failed to make ${install_path} executable."
        exit 3
    }

    log_success "Installed forgeplan to ${install_path}"

    # Verify installation
    log_info "Verifying installation..."
    if "$install_path" --version >/dev/null 2>&1; then
        version_output="$("$install_path" --version 2>&1)"
        log_success "forgeplan installed successfully: ${version_output}"
    else
        log_error "Installed binary did not respond to --version."
        log_error "Binary is at: ${install_path}"
        exit 3
    fi

    # Remind user to add install_dir to PATH if needed
    case ":${PATH}:" in
        *":${install_dir}:"*)
            # already on PATH
            ;;
        *)
            printf '\n'
            printf 'NOTE: %s is not on your PATH.\n' "$install_dir"
            printf 'Add the following line to your shell profile (~/.profile, ~/.bashrc, ~/.zshrc):\n'
            printf '\n'
            printf '    export PATH="%s:$PATH"\n' "$install_dir"
            printf '\n'
            ;;
    esac
}

main "$@"
