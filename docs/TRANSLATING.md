# Translating agnix

agnix supports multiple languages for diagnostic messages, CLI output, and LSP labels. This guide explains how to contribute translations.

## Supported Locales

| Code    | Language              | File           |
|---------|-----------------------|----------------|
| `en`    | English               | `locales/en.yml` |
| `es`    | Spanish               | `locales/es.yml` |
| `zh-CN` | Chinese (Simplified)  | `locales/zh-CN.yml` |

## Adding a New Language

1. **Copy the English locale file** as your starting template:
   ```bash
   cp locales/en.yml locales/<code>.yml
   ```
   Use the [BCP 47 language tag](https://www.rfc-editor.org/info/bcp47) as the filename (e.g., `fr.yml`, `ja.yml`, `pt-BR.yml`).

2. **Translate all string values** in the new file. Keep the YAML keys unchanged; only modify the values.

3. **Register the locale** in two places:

   a. Add to `SUPPORTED_LOCALES` in `crates/agnix-core/src/i18n.rs`:
   ```rust
   pub const SUPPORTED_LOCALES: &[&str] = &["en", "es", "zh-CN", "fr"];
   ```

   b. Add display info to `SUPPORTED_LOCALES_DISPLAY` in `crates/agnix-cli/src/locale.rs`:
   ```rust
   const SUPPORTED_LOCALES_DISPLAY: &[(&str, &str)] = &[
       ("en", "English"),
       ("es", "Spanish / Espanol"),
       ("zh-CN", "Chinese Simplified / Zhongwen"),
       // Add your locale here:
       ("fr", "French / Francais"),
   ];
   ```

4. **Add a test** in `crates/agnix-core/src/lib.rs` under `i18n_tests` to verify your translations load correctly.

5. **Run the tests**:
   ```bash
   cargo test -- i18n_tests
   ```

## File Format

Locale files use YAML with nested keys. The filename determines the locale code (e.g., `en.yml` for English). **Do not** add a top-level locale key inside the file.

```yaml
# Correct - keys start at root level
rules:
  as_001:
    message: "SKILL.md must have YAML frontmatter between --- markers"

# Incorrect - do not wrap in locale key
en:
  rules:
    as_001:
      message: "..."
```

### Parameter Interpolation

Parameters use `%{name}` syntax:

```yaml
as_004:
  message: "Name '%{name}' must be 1-64 characters of lowercase letters, digits, and hyphens"
```

Parameters are filled at runtime. Keep the same parameter names across all locales.

### Escaping

- Percent signs: use `%%` to produce a literal `%` (e.g., `%{percent}%%` renders as `50%`)
- Quotes: use standard YAML quoting rules

## What to Translate

### Must Translate

- `rules.*` - All diagnostic messages, suggestions, assumptions, and fix descriptions
- `cli.*` - CLI output labels, status messages, error messages
- `lsp.*` - LSP suggestion labels
- `core.*` - Config warnings and error messages

### Must NOT Translate

- **Rule IDs** (e.g., `AS-004`, `CC-HK-001`) - These are identifiers, not messages
- **YAML keys** - Only translate string values
- **JSON/SARIF output** - Structured output is always in English
- **Technical terms** that are proper nouns (e.g., `SKILL.md`, `CLAUDE.md`, `frontmatter`, `kebab-case`, `MCP`, `JSON-RPC`)
- **Code examples** in messages (e.g., `'Bash(git:*)'`, `'applyTo: "**/*.ts"'`)
- **CLI flag names** (e.g., `--fix`, `--locale`, `--format`)

## Translation Guidelines

1. **Be concise** - Diagnostic messages should be quick to read
2. **Preserve formatting** - If the English uses quotes around a value, keep quotes in the translation
3. **Keep technical accuracy** - Do not alter the meaning of validation rules
4. **Match parameter positions** - Parameters can be reordered for natural grammar, but all parameters must be present
5. **Test your translations** - Run `cargo test` to verify no keys are missing

## Locale Detection Order

When running agnix, the locale is resolved in this order:

1. `--locale` CLI flag (highest priority)
2. `locale` field in `.agnix.toml` configuration
3. `AGNIX_LOCALE` environment variable
4. `LANG` / `LC_ALL` environment variables
5. System locale (via `sys-locale`)
6. Fallback to `en` (English)

## Testing a Specific Locale

```bash
# Via CLI flag
agnix --locale es .

# Via environment variable
AGNIX_LOCALE=zh-CN agnix .

# Via config file (.agnix.toml)
# locale = "es"
```

## Questions?

Open an issue on GitHub if you need help with translations or have questions about specific messages.
