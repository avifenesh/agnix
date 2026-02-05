#!/usr/bin/env bash
#
# Benchmark helper script for agnix
#
# Usage:
#   ./scripts/bench.sh iai       - Run deterministic benchmarks (matches CI)
#   ./scripts/bench.sh criterion - Run wall-clock benchmarks (fast dev feedback)
#   ./scripts/bench.sh bloat     - Check binary size breakdown
#   ./scripts/bench.sh all       - Run all benchmarks
#   ./scripts/bench.sh help      - Show this help
#
# Requirements:
#   - iai: Valgrind must be installed (Linux: apt-get install valgrind, macOS: brew install valgrind)
#   - bloat: cargo-bloat must be installed (cargo install cargo-bloat)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Valgrind is installed
check_valgrind() {
    if ! command -v valgrind &> /dev/null; then
        print_error "Valgrind is not installed"
        echo ""
        echo "Install Valgrind:"
        echo "  Linux:  sudo apt-get install valgrind"
        echo "  macOS:  brew install valgrind (experimental on ARM)"
        echo ""
        echo "Note: Valgrind is not available on Windows."
        echo "      Use 'criterion' benchmarks instead."
        return 1
    fi
    return 0
}

# Check if cargo-bloat is installed
check_cargo_bloat() {
    if ! cargo bloat --version &> /dev/null; then
        print_warning "cargo-bloat not installed, installing..."
        cargo install cargo-bloat
    fi
}

# Run iai-callgrind benchmarks (deterministic, CI-equivalent)
# Returns: 0=success, 1=valgrind missing, 2=benchmark failed
run_iai() {
    print_header "Running iai-callgrind benchmarks (deterministic)"

    if ! check_valgrind; then
        return 1  # Valgrind not available
    fi

    # Check if iai-callgrind-runner is installed
    if ! command -v iai-callgrind-runner &> /dev/null; then
        print_warning "iai-callgrind-runner not installed, installing..."
        cargo install iai-callgrind-runner --version 0.14.2
    fi

    echo "This matches what CI runs. Results are 100% reproducible."
    echo ""

    if ! cargo bench --bench iai_validation --package agnix-core; then
        return 2  # Benchmark failed (regression or error)
    fi

    print_success "iai-callgrind benchmarks complete"
}

# Run Criterion benchmarks (wall-clock, fast feedback)
run_criterion() {
    print_header "Running Criterion benchmarks (wall-clock)"

    echo "Fast feedback for development iteration."
    echo "Note: Results may vary based on system load."
    echo ""

    cargo bench --bench validation --package agnix-core

    print_success "Criterion benchmarks complete"
}

# Check binary size with cargo-bloat
run_bloat() {
    print_header "Checking binary size"

    check_cargo_bloat

    echo "Building release binary..."
    cargo build --release --package agnix-cli

    # Get binary size
    local binary_path
    if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
        binary_path="target/release/agnix.exe"
    else
        binary_path="target/release/agnix"
    fi

    if [[ -f "$binary_path" ]]; then
        local size
        if [[ "$OSTYPE" == "darwin"* ]]; then
            size=$(stat -f%z "$binary_path")
        else
            size=$(stat -c%s "$binary_path" 2>/dev/null || stat -f%z "$binary_path")
        fi

        local size_mb
        size_mb=$(echo "scale=2; $size / 1048576" | bc)

        echo ""
        echo "Binary: $binary_path"
        echo "Size: ${size_mb}MB ($size bytes)"
        echo ""

        # Check threshold (10MB)
        if [ "$size" -gt 10485760 ]; then
            print_warning "Binary exceeds 10MB threshold"
        else
            print_success "Binary size within threshold"
        fi
    else
        print_error "Binary not found at $binary_path"
        return 1
    fi

    echo ""
    echo "Top 20 crates by size:"
    cargo bloat --release --crates -n 20

    print_success "Binary size check complete"
}

# Run all benchmarks
run_all() {
    print_header "Running all benchmarks"

    local iai_failed=0
    local criterion_failed=0
    local bloat_failed=0

    # Try iai (may fail if Valgrind not available or benchmark regression)
    echo "1. iai-callgrind (deterministic)"
    local iai_result
    run_iai
    iai_result=$?
    if [ $iai_result -eq 0 ]; then
        print_success "iai-callgrind: passed"
    elif [ $iai_result -eq 1 ]; then
        print_warning "iai-callgrind: skipped (Valgrind not available)"
        iai_failed=1
    else
        print_error "iai-callgrind: FAILED (regression or error)"
        iai_failed=2
        return 1  # Exit run_all immediately on benchmark failure
    fi

    # Run criterion
    echo ""
    echo "2. Criterion (wall-clock)"
    if run_criterion; then
        print_success "Criterion: passed"
    else
        print_error "Criterion: failed"
        criterion_failed=1
    fi

    # Run bloat
    echo ""
    echo "3. Binary size"
    if run_bloat; then
        print_success "Binary size: passed"
    else
        print_error "Binary size: failed"
        bloat_failed=1
    fi

    # Summary
    print_header "Summary"

    local total_failed=$((iai_failed + criterion_failed + bloat_failed))
    if [ $total_failed -eq 0 ]; then
        print_success "All benchmarks passed"
    else
        if [ $iai_failed -eq 1 ]; then
            print_warning "iai-callgrind skipped (Valgrind not available)"
        fi
        if [ $criterion_failed -eq 1 ]; then
            print_error "Criterion benchmarks failed"
        fi
        if [ $bloat_failed -eq 1 ]; then
            print_error "Binary size check failed"
        fi
    fi

    return $((criterion_failed + bloat_failed))
}

# Show help
show_help() {
    cat << 'EOF'
Benchmark helper script for agnix

USAGE:
    ./scripts/bench.sh <command>

COMMANDS:
    iai         Run iai-callgrind benchmarks (deterministic, matches CI)
                Requires: Valgrind (Linux/macOS only)

    criterion   Run Criterion benchmarks (wall-clock timing)
                Best for: Fast feedback during development

    bloat       Check binary size breakdown
                Requires: cargo-bloat (auto-installed if missing)

    all         Run all benchmarks

    help        Show this help message

DEVELOPER WORKFLOW:
    1. Make changes
    2. Run './scripts/bench.sh criterion' for fast feedback (30sec-2min)
    3. Before PR, run './scripts/bench.sh iai' to match CI (2-5min)
    4. Submit PR knowing CI will pass

PERFORMANCE TARGETS:
    - Single file validation: < 100ms (typically < 10ms)
    - 1000 files: < 5 seconds
    - Memory: < 100MB peak
    - Binary size: < 10MB

PLATFORM NOTES:
    - iai-callgrind requires Valgrind (not available on Windows)
    - macOS ARM (M1/M2/M3): Valgrind support is experimental
    - Windows users: Use 'criterion' only, CI runs iai on Linux
EOF
}

# Main entry point
main() {
    local command="${1:-help}"

    case "$command" in
        iai)
            run_iai
            ;;
        criterion)
            run_criterion
            ;;
        bloat)
            run_bloat
            ;;
        all)
            run_all
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@"
