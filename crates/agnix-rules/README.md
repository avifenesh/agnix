# agnix-rules

Validation rules for [agnix](https://github.com/avifenesh/agnix) - the agent configuration linter.

This crate provides the rule definitions used by agnix to validate agent configurations including Skills, Hooks, MCP servers, Memory files, and Plugins.

## Usage

```rust
use agnix_rules::{RULES_DATA, VALID_TOOLS, TOOL_RULE_PREFIXES};

// RULES_DATA is a static array of (rule_id, rule_name) tuples
for (id, name) in RULES_DATA {
    println!("{}: {}", id, name);
}

for tool in VALID_TOOLS {
    println!("Tool: {}", tool);
}

for (prefix, tool) in TOOL_RULE_PREFIXES {
    println!("Prefix {} -> {}", prefix, tool);
}
```

## Rule Categories

- **AS-xxx**: Agent Skills
- **CC-SK / CC-HK / CC-AG / CC-MEM / CC-PL**: Claude Code rule families
- **AGM-xxx**: AGENTS.md rules
- **COP-xxx**: GitHub Copilot
- **CUR-xxx**: Cursor
- **MCP-xxx**: Model Context Protocol
- **PE-xxx**: Prompt Engineering
- **REF-xxx**: Import/reference validation
- **VER-xxx**: Version awareness
- **XML-xxx**: XML validation
- **XP-xxx**: Cross-platform compatibility

For full rule documentation, see the [VALIDATION-RULES.md](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md).

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
