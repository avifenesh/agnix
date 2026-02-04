# agnix-rules

Validation rules for [agnix](https://github.com/avifenesh/agnix) - the agent configuration linter.

This crate provides the rule definitions used by agnix to validate agent configurations including Skills, Hooks, MCP servers, Memory files, and Plugins.

## Usage

```rust
use agnix_rules::RULES_DATA;

// RULES_DATA is a static array of (rule_id, rule_name) tuples
for (id, name) in RULES_DATA {
    println!("{}: {}", id, name);
}
```

## Rule Categories

- **AS-xxx**: Agent Skills
- **CC-xxx**: Claude Code (Hooks, Skills, Memory, etc.)
- **MCP-xxx**: Model Context Protocol
- **COP-xxx**: GitHub Copilot
- **CUR-xxx**: Cursor
- **XML-xxx**: XML/XSLT based configs
- **XP-xxx**: Cross-platform rules

For full rule documentation, see the [VALIDATION-RULES.md](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md).

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
