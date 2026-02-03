#!/bin/bash
set -euo pipefail

# Download agnix binary for the current platform
# Environment variables:
#   AGNIX_VERSION - Version to download (default: latest)
#   BUILD_FROM_SOURCE - Set to "true" to build from source instead of downloading
#   GITHUB_TOKEN - Optional token for authenticated API requests (avoids rate limits)

REPO="avifenesh/agnix"
VERSION="${AGNIX_VERSION:-latest}"
BUILD_FROM_SOURCE="${BUILD_FROM_SOURCE:-false}"

# Create bin directory
BIN_DIR="${GITHUB_WORKSPACE:-$(pwd)}/.agnix-bin"
mkdir -p "${BIN_DIR}"

# Build from source if requested (useful for testing before releases exist)
if [ "${BUILD_FROM_SOURCE}" = "true" ]; then
    echo "Building agnix from source..."

    # Ensure Rust is available
    if ! command -v cargo &> /dev/null; then
        echo "Error: cargo not found. Install Rust to build from source." >&2
        exit 1
    fi

    # Build release binary
    cargo build --release --bin agnix

    # Copy to bin directory (handle both Unix and Windows binaries)
    if [ -f "target/release/agnix.exe" ]; then
        cp "target/release/agnix.exe" "${BIN_DIR}/"
        chmod +x "${BIN_DIR}/agnix.exe" 2>/dev/null || true
    elif [ -f "target/release/agnix" ]; then
        cp "target/release/agnix" "${BIN_DIR}/"
        chmod +x "${BIN_DIR}/agnix" 2>/dev/null || true
    else
        echo "Error: Could not find built binary" >&2
        exit 1
    fi
    echo "${BIN_DIR}" >> "${GITHUB_PATH:-/dev/null}"
    echo "agnix built from source and installed to ${BIN_DIR}"
    exit 0
fi

# Validate version format to prevent path traversal attacks (only for download path)
# Accepts: "latest" or semver like "v0.1.0", "v0.1.0-beta", "v0.1.0-beta-1+build"
# Use printf to avoid echo interpreting flags like -n or -e
if [ "${VERSION}" != "latest" ]; then
    if ! printf '%s' "${VERSION}" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$'; then
        echo "Error: Invalid version format: ${VERSION}" >&2
        echo "Expected: 'latest' or semver like 'v0.1.0'" >&2
        exit 1
    fi
fi

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
        BINARY_NAME="agnix.exe"
        ;;
    *)
        echo "Error: Unsupported OS: ${OS}" >&2
        exit 1
        ;;
esac

# Set binary name (Windows uses .exe extension)
BINARY_NAME="${BINARY_NAME:-agnix}"
ARTIFACT_NAME="agnix-${TARGET}.${EXT}"

# Resolve version
if [ "${VERSION}" = "latest" ]; then
    echo "Fetching latest release version..."
    # Use GITHUB_TOKEN if available to avoid rate limits
    CURL_OPTS=(-sL)
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        CURL_OPTS+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
    fi
    # Use jq for robust JSON parsing (jq is a documented dependency)
    VERSION=$(curl "${CURL_OPTS[@]}" "https://api.github.com/repos/${REPO}/releases/latest" | jq -r '.tag_name // empty')
    if [ -z "${VERSION}" ]; then
        echo "Error: Could not determine latest version. No releases found." >&2
        echo "Please ensure a release exists at https://github.com/${REPO}/releases" >&2
        echo "Or set BUILD_FROM_SOURCE=true to build from source." >&2
        exit 1
    fi
fi

echo "Downloading agnix ${VERSION} for ${TARGET}..."

# Download URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT_NAME}"

# Download and extract
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "Downloading from ${DOWNLOAD_URL}..."
HTTP_CODE=$(curl -sL -w "%{http_code}" "${DOWNLOAD_URL}" -o "${TEMP_DIR}/${ARTIFACT_NAME}")

if [ "${HTTP_CODE}" != "200" ]; then
    echo "Error: Failed to download release (HTTP ${HTTP_CODE})" >&2
    echo "URL: ${DOWNLOAD_URL}" >&2
    exit 1
fi

echo "Extracting..."
case "${EXT}" in
    tar.gz)
        tar -xzf "${TEMP_DIR}/${ARTIFACT_NAME}" -C "${BIN_DIR}"
        ;;
    zip)
        unzip -q "${TEMP_DIR}/${ARTIFACT_NAME}" -d "${BIN_DIR}"
        ;;
esac

# Make executable (use correct binary name for platform)
chmod +x "${BIN_DIR}/${BINARY_NAME}" 2>/dev/null || true

# Add to PATH for subsequent steps
echo "${BIN_DIR}" >> "${GITHUB_PATH:-/dev/null}"

echo "agnix ${VERSION} installed to ${BIN_DIR}"
