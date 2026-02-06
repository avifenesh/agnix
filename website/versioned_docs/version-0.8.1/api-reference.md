# API Reference

## CLI

Primary commands:

```bash
agnix [OPTIONS] [PATH]
agnix schema [--output <FILE>]
agnix watch [PATH]
agnix telemetry <status|enable|disable>
```

Output formats:

- Human text (default)
- JSON (`--format json`)
- SARIF (`--format sarif`)

## MCP server

Install and run:

```bash
cargo install agnix-mcp
agnix-mcp
```

Available tools include:

- `validate_file`
- `validate_project`
- `get_rules`
- `get_rule_docs`

Specification references:

- [SPEC.md](https://github.com/avifenesh/agnix/blob/main/SPEC.md)
- [MCP docs](https://modelcontextprotocol.io)
