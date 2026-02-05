//! Benchmarks for the validation pipeline hot paths.
//!
//! Run with: cargo bench --package agnix-core
//!
//! These benchmarks measure:
//! - File type detection speed
//! - Validator registry construction
//! - Single file validation (various file types)
//! - Project validation throughput
//! - Frontmatter parsing speed

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::Path;
use tempfile::TempDir;

use agnix_core::{
    detect_file_type, validate_file, validate_file_with_registry, validate_project, LintConfig,
    ValidatorRegistry,
};

/// Benchmark file type detection - the first step in validation dispatch.
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
        // Deep paths
        ("nested_skill", Path::new(".claude/skills/deploy/SKILL.md")),
        ("nested_agent", Path::new("agents/helper.md")),
    ];

    let mut group = c.benchmark_group("detect_file_type");
    for (name, path) in paths {
        group.bench_with_input(BenchmarkId::new("path", name), path, |b, p| {
            b.iter(|| detect_file_type(black_box(p)))
        });
    }
    group.finish();
}

/// Benchmark validator registry construction.
fn bench_validator_registry(c: &mut Criterion) {
    c.bench_function("ValidatorRegistry::with_defaults", |b| {
        b.iter(ValidatorRegistry::with_defaults)
    });
}

/// Benchmark single file validation with realistic content.
fn bench_validate_single_file(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();

    // Create test files with realistic content
    let skill_content = r#"---
name: code-review
description: Use when reviewing code for quality, style, and potential issues
version: 1.0.0
model: sonnet
---

# Code Review Skill

This skill helps review code for:
- Style consistency
- Potential bugs
- Performance issues
- Security vulnerabilities

## Usage

Invoke this skill when you need a thorough code review.
"#;

    let claude_md_content = r#"# Project Memory

## Architecture
- Rust workspace with multiple crates
- Core validation engine in agnix-core
- CLI interface in agnix-cli

## Commands
```bash
cargo test
cargo build --release
```

## Guidelines
- Follow Rust idioms
- Keep functions small
- Write tests for new features
"#;

    let mcp_content = r#"{
    "name": "file-search",
    "description": "Search for files in the workspace by name or content pattern",
    "inputSchema": {
        "type": "object",
        "properties": {
            "pattern": {
                "type": "string",
                "description": "Search pattern (glob or regex)"
            },
            "type": {
                "type": "string",
                "enum": ["glob", "regex"],
                "default": "glob"
            }
        },
        "required": ["pattern"]
    }
}"#;

    let hooks_content = r#"{
    "hooks": {
        "PreToolExecution": [
            {
                "matcher": "Bash",
                "hooks": [
                    {
                        "type": "command",
                        "command": "echo 'Running command'"
                    }
                ]
            }
        ]
    }
}"#;

    // Write test files
    let skill_path = temp.path().join("SKILL.md");
    let claude_path = temp.path().join("CLAUDE.md");
    let mcp_path = temp.path().join("tools.mcp.json");
    let hooks_path = temp.path().join("settings.json");

    std::fs::write(&skill_path, skill_content).unwrap();
    std::fs::write(&claude_path, claude_md_content).unwrap();
    std::fs::write(&mcp_path, mcp_content).unwrap();
    std::fs::write(&hooks_path, hooks_content).unwrap();

    let mut group = c.benchmark_group("validate_single_file");

    // Set throughput based on file size
    group.throughput(Throughput::Bytes(skill_content.len() as u64));
    group.bench_function("skill_md", |b| {
        b.iter(|| validate_file_with_registry(black_box(&skill_path), &config, &registry))
    });

    group.throughput(Throughput::Bytes(claude_md_content.len() as u64));
    group.bench_function("claude_md", |b| {
        b.iter(|| validate_file_with_registry(black_box(&claude_path), &config, &registry))
    });

    group.throughput(Throughput::Bytes(mcp_content.len() as u64));
    group.bench_function("mcp_json", |b| {
        b.iter(|| validate_file_with_registry(black_box(&mcp_path), &config, &registry))
    });

    group.throughput(Throughput::Bytes(hooks_content.len() as u64));
    group.bench_function("hooks_json", |b| {
        b.iter(|| validate_file_with_registry(black_box(&hooks_path), &config, &registry))
    });

    group.finish();
}

/// Benchmark project validation with multiple files.
fn bench_validate_project(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let config = LintConfig::default();

    // Create a realistic project structure
    let skills = [
        ("code-review", "Use when reviewing code"),
        ("test-runner", "Use when running tests"),
        ("deploy", "Use when deploying to production"),
        ("refactor", "Use when refactoring code"),
        ("debug", "Use when debugging issues"),
    ];

    for (name, desc) in skills {
        let skill_dir = temp.path().join("skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: {}\n---\n# {}\n\nSkill body content.",
                name, desc, name
            ),
        )
        .unwrap();
    }

    // Add CLAUDE.md
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\n## Guidelines\n\n- Write clean code\n- Test everything",
    )
    .unwrap();

    // Add MCP config
    std::fs::write(
        temp.path().join("mcp.json"),
        r#"{"name": "tool", "description": "A tool", "inputSchema": {"type": "object"}}"#,
    )
    .unwrap();

    let mut group = c.benchmark_group("validate_project");

    // Small project (5 skills + 2 other files = 7 files)
    group.throughput(Throughput::Elements(7));
    group.bench_function("small_project_7_files", |b| {
        b.iter(|| validate_project(black_box(temp.path()), &config))
    });

    group.finish();

    // Create a larger project
    let large_temp = TempDir::new().unwrap();
    for i in 0..50 {
        let skill_dir = large_temp
            .path()
            .join("skills")
            .join(format!("skill-{}", i));
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: skill-{}\ndescription: Skill number {}\n---\n# Skill {}\n\nBody.",
                i, i, i
            ),
        )
        .unwrap();
    }

    std::fs::write(
        large_temp.path().join("CLAUDE.md"),
        "# Project\n\nGuidelines.",
    )
    .unwrap();

    let mut group = c.benchmark_group("validate_project");
    group.throughput(Throughput::Elements(51));
    group.bench_function("medium_project_51_files", |b| {
        b.iter(|| validate_project(black_box(large_temp.path()), &config))
    });

    group.finish();
}

/// Benchmark validation with and without registry caching.
fn bench_registry_caching(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let config = LintConfig::default();

    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: test\ndescription: Test skill\n---\n# Test",
    )
    .unwrap();

    let mut group = c.benchmark_group("registry_caching");

    // Without caching - creates new registry each time
    group.bench_function("without_cache", |b| {
        b.iter(|| validate_file(black_box(&skill_path), &config))
    });

    // With caching - reuses registry
    let registry = ValidatorRegistry::with_defaults();
    group.bench_function("with_cache", |b| {
        b.iter(|| validate_file_with_registry(black_box(&skill_path), &config, &registry))
    });

    group.finish();
}

/// Benchmark frontmatter parsing speed.
fn bench_frontmatter_parsing(c: &mut Criterion) {
    use agnix_core::parsers::frontmatter::split_frontmatter;

    let small_frontmatter = "---\nname: test\n---\nBody";

    let medium_frontmatter = r#"---
name: complex-skill
description: A more complex skill with multiple fields
version: 1.0.0
model: sonnet
triggers:
  - pattern: "review code"
  - pattern: "check this"
dependencies:
  - other-skill
  - helper-skill
---

# Complex Skill

This is the body with more content.
"#;

    let large_frontmatter = format!(
        "---\nname: large\ndescription: {}\n---\n{}",
        "A".repeat(500),
        "Body content. ".repeat(100)
    );

    let mut group = c.benchmark_group("frontmatter_parsing");

    group.throughput(Throughput::Bytes(small_frontmatter.len() as u64));
    group.bench_function("small_50_bytes", |b| {
        b.iter(|| split_frontmatter(black_box(small_frontmatter)))
    });

    group.throughput(Throughput::Bytes(medium_frontmatter.len() as u64));
    group.bench_function("medium_300_bytes", |b| {
        b.iter(|| split_frontmatter(black_box(medium_frontmatter)))
    });

    group.throughput(Throughput::Bytes(large_frontmatter.len() as u64));
    group.bench_function("large_2kb", |b| {
        b.iter(|| split_frontmatter(black_box(&large_frontmatter)))
    });

    group.finish();
}

fn bench_import_cache(c: &mut Criterion) {
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory with files that have overlapping imports
    // This simulates a real-world scenario where multiple markdown files
    // reference the same set of shared documentation files.
    let temp = TempDir::new().unwrap();

    // Create shared files that will be imported multiple times
    for i in 0..5 {
        let content = format!("# Shared Doc {}\n\nShared content {}", i, i);
        fs::write(temp.path().join(format!("shared{}.md", i)), content).unwrap();
    }

    // Create main files that each import all shared files
    for i in 0..10 {
        let imports: Vec<String> = (0..5).map(|j| format!("@shared{}.md", j)).collect();
        let content = format!("# Main Doc {}\n\nReferences: {}\n", i, imports.join(", "));
        fs::write(temp.path().join(format!("main{}.md", i)), content).unwrap();
    }

    // Create a CLAUDE.md that references all main files (to trigger import traversal)
    let main_imports: Vec<String> = (0..10).map(|i| format!("@main{}.md", i)).collect();
    fs::write(
        temp.path().join("CLAUDE.md"),
        format!("# Project\n\nFiles: {}\n", main_imports.join(", ")),
    )
    .unwrap();

    let mut group = c.benchmark_group("import_cache");

    // Benchmark project validation with shared cache (default behavior)
    group.bench_function("project_with_shared_cache", |b| {
        b.iter(|| {
            let config = LintConfig::default();
            validate_project(black_box(temp.path()), black_box(&config))
        })
    });

    // Benchmark single-file validation (no shared cache, baseline)
    // This shows the overhead of re-parsing imports for each file
    group.bench_function("single_file_no_cache", |b| {
        let claude_path = temp.path().join("CLAUDE.md");
        b.iter(|| {
            let config = LintConfig::default();
            validate_file(black_box(&claude_path), black_box(&config))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_detect_file_type,
    bench_validator_registry,
    bench_validate_single_file,
    bench_validate_project,
    bench_registry_caching,
    bench_frontmatter_parsing,
    bench_import_cache,
);
criterion_main!(benches);
