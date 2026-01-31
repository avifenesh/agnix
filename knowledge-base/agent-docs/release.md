# agnix Release Checklist

This repository is a Rust workspace:
- `crates/agnix-core` (library)
- `crates/agnix-cli` (binary: `agnix`)

## Pre-Release Checklist

- CI is green on `main`
- `CHANGELOG.md` updated with the release notes
- Workspace version bumped in `Cargo.toml` (`[workspace.package].version`)
- `cargo fmt --check` passes
- `cargo clippy --all-targets --all-features -D warnings` passes
- `cargo test` passes

## Tag and Release

```bash
git checkout main
git pull --ff-only

# if you bumped versions / changelog
git add -A
git commit -m "chore: release vX.Y.Z"

git tag vX.Y.Z
git push origin main --tags
```

## Optional: Publish to crates.io

If this project is published to crates.io, publish in dependency order:

```bash
cargo publish -p agnix-core
cargo publish -p agnix-cli
```

## Post-Release Verification

- GitHub Release created from the tag
- `cargo install agnix-cli` (or your chosen distribution method) installs successfully
- `agnix --help` runs and shows expected flags
