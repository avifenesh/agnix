# agnix Configuration Schema

This directory contains JSON Schema definitions for agnix configuration files.

## Files

- `agnix.json` - JSON Schema for `.agnix.toml` configuration files

## Usage

### VS Code

The VS Code extension automatically uses this schema for validation and autocompletion in `.agnix.toml` files.

### Manual Schema Validation

You can validate your configuration against the schema using any JSON Schema validator. For example, with `ajv`:

```bash
# Convert TOML to JSON first
cat .agnix.toml | yq -o json | ajv validate -s schemas/agnix.json
```

### Regenerating the Schema

The schema is generated from the Rust types in `agnix-core`. To regenerate:

```bash
agnix schema --output schemas/agnix.json
```

Or to output to stdout:

```bash
agnix schema
```

## Schema Contents

The schema defines the structure for:

- `severity` - Minimum severity level to report (Error, Warning, Info)
- `rules` - Enable/disable validation rule categories
- `exclude` - Glob patterns for paths to exclude from validation
- `tools` - Target tools to validate for (claude-code, cursor, codex, copilot)
- `target` - Single target tool (deprecated, use `tools` instead)
- `tool_versions` - Pin specific tool versions for version-aware validation
- `spec_revisions` - Pin specific specification revisions

See the [configuration documentation](../README.md#configuration) for detailed usage information.
