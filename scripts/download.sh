#!/bin/bash
set -euo pipefail

# Download agnix binary for the current platform
# Environment variables:
#   AGNIX_VERSION - Version to download (default: latest)

REPO="avifenesh/agnix"
VERSION="${AGNIX_VERSION:-latest}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

# Map to release artifact name
case "${OS}" in
    Linux)
        case "${ARCH}" in
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                EXT="tar.gz"
                ;;
            *)
                echo "Error: Unsupported Linux architecture: ${ARCH}" >&2
                exit 1
                ;;
        esac
        ;;
    Darwin)
        case "${ARCH}" in
            x86_64)
                TARGET="x86_64-apple-darwin"
                EXT="tar.gz"
                ;;
            arm64)
                TARGET="aarch64-apple-darwin"
                EXT="tar.gz"
                ;;
            *)
                echo "Error: Unsupported macOS architecture: ${ARCH}" >&2
                exit 1
                ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        TARGET="x86_64-pc-windows-msvc"
        EXT="zip"
        ;;
    *)
        echo "Error: Unsupported OS: ${OS}" >&2
        exit 1
        ;;
esac

ARTIFACT_NAME="agnix-${TARGET}.${EXT}"

# Resolve version
if [ "${VERSION}" = "latest" ]; then
    echo "Fetching latest release version..."
    VERSION=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "${VERSION}" ]; then
        echo "Error: Could not determine latest version" >&2
        exit 1
    fi
fi

echo "Downloading agnix ${VERSION} for ${TARGET}..."

# Download URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT_NAME}"

# Create bin directory
BIN_DIR="${GITHUB_WORKSPACE:-$(pwd)}/.agnix-bin"
mkdir -p "${BIN_DIR}"

# Download and extract
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "Downloading from ${DOWNLOAD_URL}..."
curl -sL "${DOWNLOAD_URL}" -o "${TEMP_DIR}/${ARTIFACT_NAME}"

echo "Extracting..."
case "${EXT}" in
    tar.gz)
        tar -xzf "${TEMP_DIR}/${ARTIFACT_NAME}" -C "${BIN_DIR}"
        ;;
    zip)
        unzip -q "${TEMP_DIR}/${ARTIFACT_NAME}" -d "${BIN_DIR}"
        ;;
esac

# Make executable
chmod +x "${BIN_DIR}/agnix" 2>/dev/null || true

# Add to PATH for subsequent steps
echo "${BIN_DIR}" >> "${GITHUB_PATH:-/dev/null}"

echo "agnix ${VERSION} installed to ${BIN_DIR}"
