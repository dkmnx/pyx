#!/usr/bin/env sh
set -eu

REPO="dkmnx/pyx"
BINARY="pyx"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

warn() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
die() { printf '\033[31merror: %s\033[0m\n' "$*" >&2; exit 1; }

command -v curl >/dev/null || die "curl is required"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    linux)  os="linux" ;;
    darwin) os="darwin" ;;
    *)      die "unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="arm64" ;;
    *)            die "unsupported architecture: $ARCH" ;;
esac

VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep -m1 '"tag_name"' \
    | sed 's/.*"v\(.*\)".*/\1/')

FILENAME="${BINARY}_${VERSION}_${os}_${arch}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${FILENAME}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

printf 'Downloading %s %s...\n' "$BINARY" "$VERSION"
curl -fsSL "$URL" | tar xz -C "$TMPDIR"

if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
    printf 'Installing %s (requires sudo)...\n' "$INSTALL_DIR"
    sudo mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
fi

chmod +x "${INSTALL_DIR}/${BINARY}"
printf 'Installed %s to %s\n' "$BINARY" "${INSTALL_DIR}/${BINARY}"
