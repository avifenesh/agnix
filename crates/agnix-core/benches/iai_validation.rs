//! Deterministic benchmarks using iai-callgrind for CI-stable performance testing.
//!
//! These benchmarks measure instruction counts and cache statistics, which are
//! 100% reproducible across runs. This eliminates false positive regressions
//! from GitHub Actions' inherent wall-clock timing variance.
//!
//! **Requirements:**
//! - Valgrind must be installed (Linux: `apt-get install valgrind`, macOS: `brew install valgrind`)
//! - Not supported on Windows (use Criterion benchmarks instead)
//!
//! **Run with:** `cargo bench --bench iai_validation --package agnix-core`
//!
//! **What's measured:**
//! - Instruction count (CPU instructions executed)
//! - L1/L2 cache misses
//! - Estimated cycles (accounts for memory latency)
//!
//! **CI Integration:**
//! - Runs on every PR
//! - Automatically compares against baseline
//! - Blocks merge on regression (configurable threshold)

mod fixtures;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;
use std::path::PathBuf;

use agnix_core::{LintConfig, ValidatorRegistry, validate_file_with_registry, validate_project};

use fixtures::{create_scale_project, create_single_skill_file};

// Setup functions create the test data before benchmarking.
// These run once per benchmark, not during measurement.

fn setup_single_file() -> (PathBuf, LintConfig, ValidatorRegistry) {
    let temp_dir = create_single_skill_file();
    let skill_path = temp_dir.path().join("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();

    // Leak the TempDir to prevent cleanup (iai-callgrind runs in subprocess)
    std::mem::forget(temp_dir);

    (skill_path, config, registry)
}

fn setup_100_files() -> (PathBuf, LintConfig) {
    let temp_dir = create_scale_project(100);
    let project_path = temp_dir.path().to_path_buf();
    let config = LintConfig::default();

    std::mem::forget(temp_dir);

    (project_path, config)
}

fn setup_1000_files() -> (PathBuf, LintConfig) {
    let temp_dir = create_scale_project(1000);
    let project_path = temp_dir.path().to_path_buf();
    let config = LintConfig::default();

    std::mem::forget(temp_dir);

    (project_path, config)
}

// Benchmark single file validation (instruction count).
// Measures the core validation path for a realistic SKILL.md file.
// Instruction count directly correlates to CPU time without noise.
#[library_benchmark]
#[bench::single_skill(setup_single_file())]
fn bench_validate_single_file(setup: (PathBuf, LintConfig, ValidatorRegistry)) {
    let (skill_path, config, registry) = setup;
    let _ = black_box(validate_file_with_registry(
        black_box(&skill_path),
        &config,
        &registry,
    ));
}

// Benchmark 100-file project validation.
// Tests parallelization efficiency with a medium-sized project.
// Distribution: 70% SKILL.md, 15% hooks, 10% MCP, 5% misc.
#[library_benchmark]
#[bench::project_100(setup_100_files())]
fn bench_validate_100_files(setup: (PathBuf, LintConfig)) {
    let (project_path, config) = setup;
    let _ = black_box(validate_project(black_box(&project_path), &config));
}

// Benchmark 1000-file project validation.
// Target: instruction count should scale linearly with file count.
// Wall-clock equivalent target: < 5 seconds.
#[library_benchmark]
#[bench::project_1000(setup_1000_files())]
fn bench_validate_1000_files(setup: (PathBuf, LintConfig)) {
    let (project_path, config) = setup;
    let _ = black_box(validate_project(black_box(&project_path), &config));
}

library_benchmark_group!(
    name = validation_benches;
    benchmarks =
        bench_validate_single_file,
        bench_validate_100_files,
        bench_validate_1000_files
);

main!(library_benchmark_groups = validation_benches);
