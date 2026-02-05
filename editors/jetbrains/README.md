# agnix JetBrains Plugin

JetBrains IDE integration for agnix using LSP4IJ.

<!-- Plugin description -->
Real-time validation for AI agent configuration files in JetBrains IDEs.

Features:

- Validation for `SKILL.md`, `CLAUDE.md`, `AGENTS.md`, `.claude/settings.json`, `*.mcp.json`, `.cursor/rules/*.mdc`, and related files
- Diagnostics with quick fixes and hover docs through `agnix-lsp`
- Automatic `agnix-lsp` install/update via LSP4IJ server installer flow
- Actions to restart the server, validate current file, and open settings

![agnix diagnostics in JetBrains](https://raw.githubusercontent.com/avifenesh/agnix/main/editors/jetbrains/assets/jetbrains-validation.png)
<!-- Plugin description end -->

## Requirements

- IntelliJ Platform 2023.3+
- Java 17

## Build From Source

```bash
cd editors/jetbrains
./gradlew test
./gradlew buildPlugin
```

Built plugin zip:

```text
editors/jetbrains/build/distributions/agnix-<version>.zip
```

## Run In Sandbox IDE

```bash
cd editors/jetbrains
./gradlew runIde
```

After the sandbox IDE launches:

1. Open a project with agnix config files.
2. Open `Tools > agnix > Settings`.
3. Confirm diagnostics appear for invalid files and clear after fixes.
4. Use `Tools > agnix > Restart Language Server` and verify reconnect.

## Real IDE Test Matrix

Run these checks against real installs (not only sandbox):

1. IntelliJ IDEA Community 2023.3+
2. WebStorm 2023.3+
3. PyCharm Community 2023.3+

For each IDE, verify:

1. Plugin installs from zip without startup errors.
2. `agnix-lsp` auto-download works (with auto-download enabled).
3. Manual path override works (`Settings > Tools > agnix > LSP binary path`).
4. Diagnostics and hover work on supported files.
5. Unrelated `settings.json` files (for example `.vscode/settings.json`) do not activate agnix diagnostics.

## Troubleshooting

- If `agnix-lsp` is not detected, set `LSP binary path` explicitly.
- For download issues, verify internet access to GitHub release asset domains.
- Enable trace logging with `Trace level = Messages` or `Verbose`.
