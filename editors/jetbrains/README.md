# agnix JetBrains Integration (Scaffold)

This directory currently contains the JetBrains plugin scaffold for agnix.

## Current Status

- Gradle/IntelliJ project structure is present
- Packaging metadata and changelog are present
- Plugin implementation sources are not yet in this repository

This means JetBrains support is **not production-ready** from this tree yet.

## Recommended Usage Today

- Use the agnix CLI (`agnix`) directly for validation
- Use the VS Code extension for full IDE integration
- For JetBrains experimentation, wire `agnix-lsp` through [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij)

## Development Notes

If you want to continue JetBrains integration work, this scaffold is the starting point:

```bash
cd editors/jetbrains
./gradlew build
```

Add plugin sources under the standard Gradle IntelliJ layout (`src/main/kotlin`, resources, plugin.xml) before attempting release packaging.

## Links

- [agnix Repository](https://github.com/avifenesh/agnix)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix)
- [Issue Tracker](https://github.com/avifenesh/agnix/issues)
