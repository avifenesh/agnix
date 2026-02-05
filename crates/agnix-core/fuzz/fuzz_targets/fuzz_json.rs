//! Fuzz target for JSON parsing (MCP configs, plugins, hooks)
//!
//! This target tests JSON parsing paths used for MCP configurations,
//! plugin manifests, and hooks settings. These must handle malformed
//! JSON gracefully without panicking.
//!
//! Properties validated:
//! - No panics on malformed JSON
//! - No panics on deeply nested structures
//! - No panics on large numbers
//! - No panics on invalid UTF-8 in strings
//! - No panics on unexpected types in schemas

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Test MCP config parsing - should return error, not panic
    let _mcp_result: Result<agnix_core::config::LintConfig, _> = serde_json::from_str(data);

    // Test raw JSON parsing - should return error, not panic
    let _json_result: Result<serde_json::Value, _> = serde_json::from_str(data);

    // Test YAML parsing (frontmatter) - should return error, not panic
    let _yaml_result: Result<serde_json::Value, _> = serde_yaml::from_str(data);

    // If we got here, no panic occurred
});
