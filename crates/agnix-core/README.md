# agnix-core

Core validation engine for [agnix](https://github.com/avifenesh/agnix) - the agent configuration linter.

This crate provides the parsing, schema validation, and diagnostic generation for agent configurations including Skills, Hooks, MCP servers, Memory files, and Plugins.

## Features

- YAML/JSON/TOML/Markdown frontmatter parsing
- Schema validation against documented specifications
- Diagnostic generation with line/column locations
- Support for multiple agent configuration formats

## Usage

This is a library crate used by `agnix-cli`. For most users, install the CLI:

```bash
cargo install agnix-cli
```

For programmatic usage:

```rust
use agnix_core::{validate_project, LintConfig};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LintConfig::default();
    let result = validate_project(Path::new("."), &config)?;

    println!("checked {} files", result.files_checked);
    for diag in result.diagnostics {
        println!("{}:{} {} {}", diag.file.display(), diag.line, diag.rule, diag.message);
    }

    Ok(())
}
```

For project-level validation across multiple files:

```rust
use agnix_core::{validate_project_rules, LintConfig};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LintConfig::default();
    let result = validate_project_rules(Path::new("."), &config)?;

    for diag in result.diagnostics {
        println!("{}:{} {} {} {}", diag.file.display(), diag.line, diag.rule, diag.level, diag.message);
    }

    Ok(())
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
