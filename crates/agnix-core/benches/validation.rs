//! Benchmarks for the validation pipeline hot paths.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::Path;

use agnix_core::{detect_file_type, validate_file, LintConfig, ValidatorRegistry};

fn bench_detect_file_type(c: &mut Criterion) {
    let paths = [
        ("skill", Path::new("SKILL.md")),
        ("claude_md", Path::new("CLAUDE.md")),
        ("agents_md", Path::new("AGENTS.md")),
        ("hooks", Path::new("settings.json")),
        ("plugin", Path::new("plugin.json")),
        ("mcp", Path::new("mcp.json")),
        ("generic_md", Path::new("README.md")),
        ("unknown", Path::new("file.txt")),
    ];

    let mut group = c.benchmark_group("detect_file_type");
    for (name, path) in paths {
        group.bench_with_input(BenchmarkId::new("path", name), path, |b, p| {
            b.iter(|| detect_file_type(black_box(p)))
        });
    }
    group.finish();
}

fn bench_validator_registry(c: &mut Criterion) {
    c.bench_function("ValidatorRegistry::with_defaults", |b| {
        b.iter(|| ValidatorRegistry::with_defaults())
    });
}

fn bench_validate_file(c: &mut Criterion) {
    let config = LintConfig::default();

    // Use test fixtures if available
    let fixtures = [
        (
            "skill",
            Path::new("tests/fixtures/valid/skills/basic-valid/SKILL.md"),
        ),
        (
            "claude_md",
            Path::new("tests/fixtures/valid/claude-md/minimal/CLAUDE.md"),
        ),
    ];

    let mut group = c.benchmark_group("validate_file");
    for (name, path) in fixtures {
        if path.exists() {
            group.bench_with_input(BenchmarkId::new("fixture", name), path, |b, p| {
                b.iter(|| validate_file(black_box(p), black_box(&config)))
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_detect_file_type,
    bench_validator_registry,
    bench_validate_file,
);
criterion_main!(benches);
