#!/bin/bash
set -euo pipefail

# Run agnix with provided inputs
# Environment variables:
#   INPUT_PATH    - Path to validate
#   INPUT_STRICT  - Treat warnings as errors
#   INPUT_TARGET  - Target tool
#   INPUT_CONFIG  - Config file path
#   INPUT_FORMAT  - Output format
#   INPUT_VERBOSE - Verbose output
#
# Dependencies: jq (for JSON parsing)

# Sanitize string for GitHub workflow commands
# Escapes special characters that could be used for command injection
# Per GitHub docs: %, CR, LF, :, and , must be URL-encoded
# Uses bash parameter substitution for portability (sed is line-oriented)
sanitize_workflow_value() {
    local value="${1-}"
    # Encode % first to avoid double-encoding
    value=${value//'%'/%25}
    # Replace actual CR and LF characters
    value=${value//$'\r'/%0D}
    value=${value//$'\n'/%0A}
    # Encode additional special characters
    value=${value//:/%3A}
    value=${value//,/%2C}
    printf '%s' "$value"
}

BIN_DIR="${GITHUB_WORKSPACE:-$(pwd)}/.agnix-bin"

# Find agnix binary (handles both Unix and Windows)
AGNIX_BASE="${BIN_DIR}/agnix"
if [ -f "${AGNIX_BASE}" ]; then
    AGNIX="${AGNIX_BASE}"
elif [ -f "${AGNIX_BASE}.exe" ]; then
    AGNIX="${AGNIX_BASE}.exe"
else
    echo "Error: agnix binary not found in ${BIN_DIR} (expected 'agnix' or 'agnix.exe')." >&2
    exit 1
fi

# Build command arguments
ARGS=()

# Path (positional argument)
PATH_ARG="${INPUT_PATH:-.}"
ARGS+=("${PATH_ARG}")

# Strict mode
if [ "${INPUT_STRICT:-false}" = "true" ]; then
    ARGS+=("--strict")
fi

# Target tool
if [ -n "${INPUT_TARGET:-}" ] && [ "${INPUT_TARGET}" != "generic" ]; then
    ARGS+=("--target" "${INPUT_TARGET}")
fi

# Config file
if [ -n "${INPUT_CONFIG:-}" ]; then
    ARGS+=("--config" "${INPUT_CONFIG}")
fi

# Verbose
if [ "${INPUT_VERBOSE:-false}" = "true" ]; then
    ARGS+=("--verbose")
fi

# Output format - always use JSON internally for parsing
ORIGINAL_FORMAT="${INPUT_FORMAT:-text}"
ARGS+=("--format" "json")

echo "Running: agnix ${ARGS[*]}"

# Run agnix and capture output
set +e
OUTPUT=$("${AGNIX}" "${ARGS[@]}" 2>&1)
EXIT_CODE=$?
set -e

# Parse JSON output for errors and warnings
ERRORS=0
WARNINGS=0

if echo "${OUTPUT}" | jq -e '.summary' > /dev/null 2>&1; then
    ERRORS=$(echo "${OUTPUT}" | jq -r '.summary.errors // 0')
    WARNINGS=$(echo "${OUTPUT}" | jq -r '.summary.warnings // 0')
fi

# Set outputs
{
    echo "errors=${ERRORS}"
    echo "warnings=${WARNINGS}"
    if [ ${EXIT_CODE} -eq 0 ]; then
        echo "result=success"
    else
        echo "result=failure"
    fi
} >> "${GITHUB_OUTPUT:-/dev/stdout}"

# Generate GitHub annotations from diagnostics
# Use tab delimiter to handle Windows paths that contain colons (e.g., C:/path)
# Sanitize all fields to prevent workflow command injection
if echo "${OUTPUT}" | jq -e '.diagnostics' > /dev/null 2>&1; then
    echo "${OUTPUT}" | jq -r '.diagnostics[] | "\(.level)\t\(.file)\t\(.line)\t\(.column)\t\(.message) [\(.rule)]"' | while IFS=$'\t' read -r level file line col msg; do
        safe_file=$(sanitize_workflow_value "${file}")
        safe_line=$(sanitize_workflow_value "${line}")
        safe_col=$(sanitize_workflow_value "${col}")
        safe_msg=$(sanitize_workflow_value "${msg}")
        case "${level}" in
            error)
                echo "::error file=${safe_file},line=${safe_line},col=${safe_col}::${safe_msg}"
                ;;
            warning)
                echo "::warning file=${safe_file},line=${safe_line},col=${safe_col}::${safe_msg}"
                ;;
            info)
                echo "::notice file=${safe_file},line=${safe_line},col=${safe_col}::${safe_msg}"
                ;;
        esac
    done
fi

# Handle SARIF output
if [ "${ORIGINAL_FORMAT}" = "sarif" ]; then
    SARIF_FILE="${GITHUB_WORKSPACE:-$(pwd)}/agnix-results.sarif"

    # Build SARIF command arguments
    SARIF_ARGS=("${PATH_ARG}")
    if [ "${INPUT_STRICT:-false}" = "true" ]; then
        SARIF_ARGS+=("--strict")
    fi
    if [ -n "${INPUT_TARGET:-}" ] && [ "${INPUT_TARGET}" != "generic" ]; then
        SARIF_ARGS+=("--target" "${INPUT_TARGET}")
    fi
    if [ -n "${INPUT_CONFIG:-}" ]; then
        SARIF_ARGS+=("--config" "${INPUT_CONFIG}")
    fi
    if [ "${INPUT_VERBOSE:-false}" = "true" ]; then
        SARIF_ARGS+=("--verbose")
    fi
    SARIF_ARGS+=("--format" "sarif")

    # Re-run with SARIF format
    "${AGNIX}" "${SARIF_ARGS[@]}" > "${SARIF_FILE}" 2>/dev/null || true

    echo "sarif_file=${SARIF_FILE}" >> "${GITHUB_OUTPUT:-/dev/stdout}"
    echo "SARIF output written to ${SARIF_FILE}"
fi

# Print summary
echo ""
echo "agnix validation complete"
echo "  Errors: ${ERRORS}"
echo "  Warnings: ${WARNINGS}"

# Exit based on FAIL_ON_ERROR setting
# Default: true (action fails if agnix finds errors)
# Set to false to always succeed and check 'result' output
if [ "${FAIL_ON_ERROR:-true}" = "false" ]; then
    exit 0
else
    exit ${EXIT_CODE}
fi
