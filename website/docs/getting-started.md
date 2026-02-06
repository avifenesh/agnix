# Getting Started

## Install agnix

```bash
npm install -g agnix
```

## Validate your repository

```bash
agnix .
```

## Apply auto-fixes

```bash
agnix --fix .
```

## Recommended first workflow

1. Run `agnix --target claude-code .` for tool-specific checks.
2. Add `.agnix.toml` for project defaults.
3. Integrate `agnix` into CI with SARIF output for code scanning.

Reference:

- [Configuration](./configuration.md)
- [Installation](./installation.md)
- [Troubleshooting](./troubleshooting.md)
