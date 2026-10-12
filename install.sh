#!/bin/sh
# jbundle installer
# Usage: curl -sSL https://raw.githubusercontent.com/avelino/jbundle/main/install.sh | sh
set -e

REPO="avelino/jbundle"
INSTALL_DIR="${JBUNDLE_INSTALL_DIR:-/usr/local/bin}"

detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$OS" in
        linux)  PLATFORM="linux" ;;
        darwin) PLATFORM="darwin" ;;
        *)
            echo "Error: unsupported OS: $OS" >&2
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)   ARCH="aarch64" ;;
        *)
            echo "Error: unsupported architecture: $ARCH" >&2
            exit 1
            ;;
    esac

    echo "jbundle-${PLATFORM}-${ARCH}"
}

get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
    else
        echo "Error: curl or wget required" >&2
        exit 1
    fi
}

download() {
    URL="$1"
    DEST="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -sSL "$URL" -o "$DEST"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$URL" -O "$DEST"
    fi
}

main() {
    ARTIFACT=$(detect_platform)

    VERSION="${JBUNDLE_VERSION:-}"
    if [ -z "$VERSION" ]; then
        echo "Fetching latest version..."
        VERSION=$(get_latest_version)
        if [ -z "$VERSION" ]; then
            echo "Could not determine latest version, using pre-release..."
            VERSION="latest"
        fi
    fi

    if [ "$VERSION" = "latest" ]; then
        TARBALL="${ARTIFACT}.tar.gz"
    else
        TARBALL="${ARTIFACT}-${VERSION}.tar.gz"
    fi

    if [ "$VERSION" = "latest" ]; then
        DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${TARBALL}"
    else
        DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"
    fi

    echo "Downloading jbundle ${VERSION} for $(uname -s)/$(uname -m)..."
    echo "  ${DOWNLOAD_URL}"

    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    download "$DOWNLOAD_URL" "${TMP_DIR}/${TARBALL}"
    tar xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

    # Install
    if [ -w "$INSTALL_DIR" ]; then
        mv "${TMP_DIR}/jbundle" "${INSTALL_DIR}/jbundle"
    else
        echo "Installing to ${INSTALL_DIR} (requires sudo)..."
        sudo mv "${TMP_DIR}/jbundle" "${INSTALL_DIR}/jbundle"
    fi

    chmod +x "${INSTALL_DIR}/jbundle"

    echo ""
    echo "jbundle installed to ${INSTALL_DIR}/jbundle"
    echo ""
    echo "Run 'jbundle --help' to get started."
}

main
