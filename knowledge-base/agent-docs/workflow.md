# agnix Development Workflow

This is the recommended workflow for changes to agnix (code or documentation).

## 1. Work Isolation

- Prefer a dedicated branch per change.
- For parallel work, prefer `git worktree` so multiple branches can be edited/tested concurrently.

## 2. Local Validation (Pre-PR)

Run the standard Rust checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -D warnings
cargo test
```

If you changed only documentation, still run `cargo test` before opening a PR when practical.

## 3. Documentation Consistency Rules

If you modify the knowledge base:
- Keep rule counts consistent across `SPEC.md`, `CLAUDE.md`, and `knowledge-base/INDEX.md`
- Update `knowledge-base/VALIDATION-RULES.md` sources when facts change
- For cross-platform content, follow support tiers ordering (S tier first, then A)

## 4. Pull Request Checklist

- `git status` clean
- CI checks passing
- Address review comments and resolve threads
- Merge only after required workflows succeed
