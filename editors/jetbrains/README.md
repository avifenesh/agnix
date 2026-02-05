# agnix JetBrains Plugin

Real-time validation for AI agent configuration files in JetBrains IDEs.

<!-- Plugin description -->
**agnix** is a linter for AI agent configuration files. It validates SKILL.md, CLAUDE.md, AGENTS.md, MCP configs, and more against 100+ validation rules.

## Features

- Real-time validation as you type
- 100+ validation rules across multiple categories
- Quick fixes for common issues
- Hover documentation for configuration fields
- Auto-download of LSP binary from GitHub releases

## Supported Files

- **SKILL.md** - Agent Skills specification
- **CLAUDE.md / AGENTS.md** - Project memory files
- **.claude/settings.json** - Claude Code settings
- **\*.mcp.json** - MCP server configurations
- **.cursor/rules/\*.mdc** - Cursor rules
- **.github/copilot-instructions.md** - GitHub Copilot instructions
<!-- Plugin description end -->

## Installation

### From JetBrains Marketplace

1. Open **Settings/Preferences** > **Plugins**
2. Search for "agnix"
3. Click **Install**
4. Restart your IDE

### Manual Installation

1. Download the latest release from [GitHub Releases](https://github.com/avifenesh/agnix/releases)
2. Open **Settings/Preferences** > **Plugins**
3. Click the gear icon > **Install Plugin from Disk...**
4. Select the downloaded `.zip` file
5. Restart your IDE

## Requirements

- JetBrains IDE 2023.2 or later (IntelliJ IDEA, WebStorm, PyCharm, etc.)
- [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin (installed automatically as dependency)

## Configuration

Open **Settings/Preferences** > **Tools** > **agnix** to configure:

- **Enable**: Toggle validation on/off
- **LSP binary path**: Custom path to agnix-lsp (leave empty for auto-detection)
- **Auto-download**: Automatically download LSP binary if not found
- **Trace level**: Debug LSP communication (off, messages, verbose)
- **CodeLens**: Show inline rule annotations

## Usage

1. Open any supported file (e.g., `SKILL.md`, `CLAUDE.md`)
2. Issues appear automatically in the **Problems** panel
3. Hover over highlighted text for details
4. Use quick fixes (lightbulb icon) to resolve issues

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Show Quick Fixes | Alt+Enter |
| Go to Problems | F2 |

### Context Menu

Right-click in the editor to access:
- **agnix** > **Validate Current File**
- **agnix** > **Restart Language Server**
- **agnix** > **Settings**

## Validation Rules

agnix includes 100+ validation rules organized by category:

| Category | Prefix | Description |
|----------|--------|-------------|
| Agent Skills | AS-* | SKILL.md structure and fields |
| Claude Skills | CC-SK-* | Claude-specific skill rules |
| Claude Hooks | CC-HK-* | Hooks configuration |
| Claude Agents | CC-AG-* | Agent definitions |
| Claude Plugins | CC-PL-* | Plugin manifests |
| Prompt Engineering | PE-* | Prompt quality |
| MCP | MCP-* | Model Context Protocol |
| Memory Files | AGM-* | AGENTS.md validation |
| GitHub Copilot | COP-* | Copilot instructions |
| Cursor | CUR-* | Cursor rules |
| XML | XML-* | XML tag formatting |
| Cross-Platform | XP-* | Multi-tool compatibility |

See [VALIDATION-RULES.md](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md) for details.

## Troubleshooting

### Language server not starting

1. Check **Settings** > **Tools** > **agnix** for correct configuration
2. Verify agnix-lsp binary exists at the configured path
3. Try **Tools** > **agnix** > **Restart Language Server**
4. Check the IDE log for errors (**Help** > **Show Log in Explorer/Finder**)

### Binary not found

The plugin can automatically download the LSP binary:
1. Enable **Auto-download** in settings
2. Or manually install: `cargo install agnix-lsp`
3. Or download from [GitHub Releases](https://github.com/avifenesh/agnix/releases)

### No validation appearing

1. Ensure the plugin is enabled in settings
2. Check that the file type is supported
3. Save the file to trigger validation
4. Check the **Problems** panel (View > Tool Windows > Problems)

## Development

### Building from Source

```bash
cd editors/jetbrains
./gradlew build
```

### Running in Sandbox IDE

```bash
./gradlew runIde
```

### Running Tests

```bash
./gradlew test
```

## License

MIT License - see [LICENSE](https://github.com/avifenesh/agnix/blob/main/LICENSE) for details.

## Links

- [agnix GitHub Repository](https://github.com/avifenesh/agnix)
- [Issue Tracker](https://github.com/avifenesh/agnix/issues)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix)
