# VER-001: No Tool/Spec Versions Pinned

This fixture tests the VER-001 rule which warns when no tool or spec versions
are explicitly pinned in the .agnix.toml configuration.

## Expected Behavior

- When no versions are pinned: VER-001 info diagnostic
- When any version is pinned: No VER-001 diagnostic
