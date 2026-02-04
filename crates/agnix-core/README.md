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
cargo install agnix
```

For programmatic usage:

```rust
use agnix_core::validate_project;
use std::path::Path;

let diagnostics = validate_project(Path::new("."));
for diag in diagnostics {
    println!("{}: {}", diag.rule, diag.message);
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
