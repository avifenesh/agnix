//! Fuzz target for YAML frontmatter parsing
//!
//! This target tests the `split_frontmatter()` function which is the first
//! step in parsing any agent config file. It must handle arbitrary input
//! without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Test split_frontmatter() - should never panic on any input
    let parts = agnix_core::__internal::split_frontmatter(data);

    // Verify invariants:
    // 1. Offsets must be within bounds
    assert!(parts.frontmatter_start <= data.len());
    assert!(parts.body_start <= data.len());

    // 2. If has_frontmatter but no closing, frontmatter should be empty
    if parts.has_frontmatter && !parts.has_closing {
        assert!(parts.frontmatter.is_empty());
    }
});
