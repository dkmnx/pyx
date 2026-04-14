#!/bin/bash
# Cross-platform install script for pyx
# Usage: curl -sSL https://raw.githubusercontent.com/dkmnx/pyx/main/scripts/install.sh | bash
#   or: ./install.sh [--version VERSION] [--from-source]

set -e

# Configuration
REPO="dkmnx/pyx"
INSTALL_DIR="${HOME}/.local/bin"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="linux";;
        Darwin*)    OS="macos";;
        *)          OS="unknown";;
    esac
}

# Detect architecture and return target string
get_target() {
    local os="$1"
    local arch="$(uname -m)"

    case "$os" in
        linux)
            case "$arch" in
                x86_64|amd64)   echo "x86_64-unknown-linux-gnu";;
                aarch64|arm64)  echo "aarch64-unknown-linux-gnu";;
                *)              log_error "Unsupported architecture: $arch"; exit 1;;
            esac
            ;;
        macos)
            case "$arch" in
                x86_64|amd64)   echo "x86_64-apple-darwin";;
                aarch64|arm64)  echo "aarch64-apple-darwin";;
                *)              log_error "Unsupported architecture: $arch"; exit 1;;
            esac
            ;;
        windows)
            case "$arch" in
                x86_64|amd64)   echo "x86_64-pc-windows-msvc";;
                *)              log_error "Unsupported architecture: $arch"; exit 1;;
            esac
            ;;
        *)
            log_error "Unsupported OS: $os"; exit 1
            ;;
    esac
}

# Get file extension
get_ext() {
    local target="$1"
    case "$target" in
        *windows*)   echo "zip";;
        *)          echo "tar.gz";;
    esac
}

# Get latest version from GitHub
get_latest_version() {
    local version

    # Try gh CLI first (authenticated)
    if command -v gh &> /dev/null; then
        version=$(gh release list --repo "${REPO}" --limit 1 2>/dev/null | awk '{print $1}' | sed 's/^v//')
    fi

    # Fallback to API
    if [[ -z "$version" ]]; then
        # Use /releases/latest to avoid pre-releases
        version=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"v?([^"]+)".*/\1/')
    fi

    # If /latest fails (no stable release), fall back to /releases and filter pre-releases
    if [[ -z "$version" ]]; then
        version=$(curl -sSL "https://api.github.com/repos/${REPO}/releases" 2>/dev/null \
            | grep -v '"prerelease": true' \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"v?([^"]+)".*/\1/')
    fi

    if [[ -z "$version" ]]; then
        log_error "Failed to fetch latest version"
        exit 1
    fi

    echo "$version"
}

# Check prerequisites
check_prereqs() {
    local from_source="$1"

    if [[ "$from_source" == "true" ]]; then
        if ! command -v cargo &> /dev/null; then
            log_error "Missing Rust (cargo). Install from: https://rustup.rs"
            exit 1
        fi
    fi

    if ! command -v curl &> /dev/null; then
        log_error "Missing curl"
        exit 1
    fi
}

# Build from source
build_from_source() {
    log_info "Building pyx from source..."

    if [[ ! -d ".git" ]]; then
        log_error "Not a git repository. Cannot build from source."
        exit 1
    fi

    CARGO_TERM_COLOR=never cargo build --release

    local binary="./target/release/pyx"
    if [[ ! -f "$binary" ]]; then
        log_error "Build failed: binary not found"
        exit 1
    fi

    echo "$binary"
}

# Download binary with checksum verification
download_binary() {
    local version="$1"
    local target="$2"
    local ext="$3"

    log_info "Downloading pyx v${version} for ${target}..."

    local base_url="https://github.com/${REPO}/releases/download/v${version}"
    local checksum_url="${base_url}/SHA256SUMS.txt"

    local tmp_dir=$(mktemp -d)
    # Don't set trap here - we'll handle cleanup differently

    # Download checksums using gh if available (avoids rate limits)
    log_info "Fetching checksums..."
    local checksum_file="${tmp_dir}/SHA256SUMS.txt"
    if command -v gh &> /dev/null; then
        gh release download "v${version}" --repo "${REPO}" --pattern "SHA256SUMS.txt" --dir "$tmp_dir" 2>/dev/null || \
        curl -sSL "$checksum_url" -o "$checksum_file"
    else
        curl -sSL "$checksum_url" -o "$checksum_file"
    fi

    if [[ ! -s "$checksum_file" ]]; then
        log_error "Failed to download checksums"
        rm -rf "$tmp_dir"
        exit 1
    fi

    # Build goreleaser-style archive name: pyx_<version>_<os>_<arch>.tar.gz
    local gos garch
    case "$target" in
        *linux*)   gos="linux" ;;
        *apple*|*darwin*) gos="darwin" ;;
        *)         gos="linux" ;;
    esac
    case "$target" in
        *x86_64*|*amd64*) garch="x86_64" ;;
        *aarch64*|*arm64*) garch="arm64" ;;
        *)                 garch="x86_64" ;;
    esac
    local filename="pyx_${version}_${gos}_${garch}.${ext}"
    local archive="${tmp_dir}/${filename}"

    log_info "Downloading binary..."
    if command -v gh &> /dev/null; then
        gh release download "v${version}" --repo "${REPO}" --pattern "${filename}" --dir "$tmp_dir" 2>/dev/null || \
        curl -sSL "${base_url}/${filename}" -o "$archive"
    else
        curl -sSL "${base_url}/${filename}" -o "$archive"
    fi

    # Verify checksum of archive
    log_info "Verifying checksum..."
    cd "$tmp_dir"

    local computed
    computed=$(sha256sum "./${filename}" | awk '{print $1}')

    # Search for matching checksum (handle path prefixes in checksum file)
    local found=false
    while IFS= read -r line; do
        local checksum file
        checksum=$(echo "$line" | awk '{print $1}')
        # Remove any path prefix and * marker
        file=$(echo "$line" | awk '{print $2}' | tr -d '*')
        file=$(basename "$file")

        if [[ "$file" == "${filename}" ]]; then
            found=true
            if [[ "$checksum" == "$computed" ]]; then
                log_info "Checksum verified!"
            else
                log_error "Checksum mismatch!"
                log_error "Expected: $checksum"
                log_error "Got:      $computed"
                rm -rf "$tmp_dir"
                exit 1
            fi
            break
        fi
    done < "$checksum_file"

    if [[ "$found" == "false" ]]; then
        log_error "Archive not found in checksums file"
        log_error "Looking for: ${filename}"
        rm -rf "$tmp_dir"
        exit 1
    fi

    # Extract archive
    if [[ "$ext" == "tar.gz" ]]; then
        tar -xzf "$filename" || { log_error "Failed to extract archive"; rm -rf "$tmp_dir"; exit 1; }
    else
        unzip -q "$filename" || { log_error "Failed to extract archive"; rm -rf "$tmp_dir"; exit 1; }
    fi

    # Verify binary exists
    if [[ ! -f "${tmp_dir}/pyx" ]]; then
        log_error "Binary not found after extraction"
        rm -rf "$tmp_dir"
        exit 1
    fi

    # Copy binary to install location before returning
    # This avoids the trap-fires-on-subshell-exit issue
    mkdir -p "$INSTALL_DIR"
    cp "${tmp_dir}/pyx" "${INSTALL_DIR}/pyx"
    chmod +x "${INSTALL_DIR}/pyx"

    # Cleanup temp dir and return install path
    rm -rf "$tmp_dir"
    echo "${INSTALL_DIR}/pyx"
}

# Add to PATH
add_to_path() {
    local path_line="export PATH=\"\${HOME}/.local/bin:\${PATH}\""

    local shell_config=""
    case "${SHELL:-$(basename "$SHELL")}" in
        *zsh*)  shell_config="${HOME}/.zshrc";;
        *bash*) shell_config="${HOME}/.bashrc";;
        *fish*) shell_config="${HOME}/.config/fish/config.fish"; path_line="set -gx PATH \$HOME/.local/bin \$PATH";;
        *)      shell_config="${HOME}/.profile";;
    esac

    if [[ -f "$shell_config" ]]; then
        if ! grep -q "${INSTALL_DIR}" "$shell_config" 2>/dev/null; then
            echo "" >> "$shell_config"
            echo "# Added by pyx installer" >> "$shell_config"
            echo "$path_line" >> "$shell_config"
            log_info "Added ${INSTALL_DIR} to PATH in ${shell_config}"
            log_info "Restart your shell or run: source ${shell_config}"
        else
            log_info "${INSTALL_DIR} already in PATH"
        fi
    else
        log_warn "Could not detect shell config. Manually add ${INSTALL_DIR} to your PATH."
    fi
}

# Main
main() {
    local version=""
    local from_source=false

    # Parse args
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --version)
                version="$2"
                shift 2
                ;;
            --from-source)
                from_source=true
                shift
                ;;
            *)
                echo "Usage: $0 [--version VERSION] [--from-source]"
                exit 1
                ;;
        esac
    done

    echo "=========================================="
    echo "           pyx Installer"
    echo "=========================================="
    echo ""

    detect_os
    log_info "Detected: ${OS} ($(uname -m))"

    local target
    target=$(get_target "$OS")
    local ext
    ext=$(get_ext "$target")
    log_info "Target: ${target}"

    check_prereqs "$from_source"

    local src=""

    if [[ "$from_source" == "true" ]]; then
        src=$(build_from_source)
    else
        if [[ -z "$version" ]]; then
            version=$(get_latest_version)
        fi
        log_info "Installing version: ${version}"
        src=$(download_binary "$version" "$target" "$ext")
    fi

    add_to_path

    echo ""
    log_info "Installation complete!"
    log_info "Run 'pyx --help' to get started."
}

main "$@"
