//! Build script for agnix-rules.
//!
//! Generates Rust code from rules.json at compile time.
//! Supports both local crate builds (crates.io) and workspace builds (development).
//!
//! Generated constants:
//! - `RULES_DATA`: All rule (id, name) tuples
//! - `VALID_TOOLS`: Unique tool names from evidence.applies_to.tool
//! - `TOOL_RULE_PREFIXES`: Mapping of (prefix, tool) for tool-specific rules

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum allowed file size for rules.json (5 MB)
const MAX_RULES_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// Find the workspace root by searching for Cargo.toml with [workspace]
fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|path| {
            path.join("Cargo.toml")
                .exists()
                .then(|| fs::read_to_string(path.join("Cargo.toml")).ok())
                .flatten()
                .is_some_and(|content| {
                    content.contains("[workspace]") || content.contains("[workspace.")
                })
        })
        .map(|p| p.to_path_buf())
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);

    // Try crate-local rules.json first (for crates.io builds)
    // Then fall back to workspace knowledge-base/rules.json (for development)
    let crate_rules = manifest_path.join("rules.json");
    let workspace_rules =
        find_workspace_root(manifest_path).map(|root| root.join("knowledge-base/rules.json"));

    // Watch crate-local path for changes (always, in case file is added later)
    println!("cargo:rerun-if-changed={}", crate_rules.display());

    let rules_path = if crate_rules.exists() {
        crate_rules
    } else if let Some(ws_rules) = workspace_rules {
        if ws_rules.exists() {
            // Also watch workspace rules for development builds
            println!("cargo:rerun-if-changed={}", ws_rules.display());
            ws_rules
        } else {
            panic!(
                "Could not find rules.json at {} or {}",
                manifest_path.join("rules.json").display(),
                ws_rules.display()
            );
        }
    } else {
        panic!(
            "Could not find rules.json at {} (no workspace root found)",
            manifest_path.join("rules.json").display()
        );
    };

    // Validate file size before reading (defense against DoS)
    let file_size = fs::metadata(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to get metadata for {}: {}", rules_path.display(), e))
        .len();
    if file_size > MAX_RULES_FILE_SIZE {
        panic!(
            "rules.json at {} is too large ({} bytes, max {} bytes)",
            rules_path.display(),
            file_size,
            MAX_RULES_FILE_SIZE
        );
    }

    let rules_json = fs::read_to_string(&rules_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read rules.json at {}: {}",
            rules_path.display(),
            e
        )
    });

    // Parse to validate JSON structure
    let rules: serde_json::Value = serde_json::from_str(&rules_json).unwrap_or_else(|e| {
        panic!(
            "Failed to parse rules.json at {}: {}",
            rules_path.display(),
            e
        )
    });

    // Extract just the rules array and generate Rust code
    let rules_array = rules["rules"]
        .as_array()
        .expect("rules.json must have a 'rules' array");

    let mut generated_code = String::new();
    generated_code.push_str("// Auto-generated from rules.json by build.rs\n");
    generated_code.push_str("// Do not edit manually!\n\n");
    generated_code.push_str("/// Rule data as (id, name) tuples.\n");
    generated_code.push_str("/// \n");
    generated_code.push_str(
        "/// This is the complete list of validation rules from knowledge-base/rules.json.\n",
    );
    generated_code.push_str("pub const RULES_DATA: &[(&str, &str)] = &[\n");

    // Escape special characters for Rust string literal (defense-in-depth)
    let escape_str = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    };

    // Validate rule ID format (e.g., AS-001, CC-HK-001, MCP-001)
    let is_valid_id = |id: &str| -> bool {
        !id.is_empty()
            && id.len() <= 20
            && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };

    // Validate rule name (non-empty, reasonable length, no control characters)
    let is_valid_name = |name: &str| -> bool {
        !name.is_empty() && name.len() <= 200 && !name.chars().any(|c| c.is_control() && c != ' ')
    };

    for (idx, rule) in rules_array.iter().enumerate() {
        let id = rule["id"]
            .as_str()
            .unwrap_or_else(|| panic!("rule[{}] must have string 'id' field", idx));
        let name = rule["name"]
            .as_str()
            .unwrap_or_else(|| panic!("rule[{}] must have string 'name' field", idx));

        // Validate fields before code generation
        if !is_valid_id(id) {
            panic!(
                "rule[{}] has invalid id '{}': must be 1-20 alphanumeric/hyphen characters",
                idx, id
            );
        }
        if !is_valid_name(name) {
            panic!(
                "rule[{}] '{}' has invalid name: must be 1-200 chars, no control characters",
                idx, id
            );
        }

        let escaped_id = escape_str(id);
        let escaped_name = escape_str(name);
        generated_code.push_str(&format!(
            "    (\"{}\", \"{}\"),\n",
            escaped_id, escaped_name
        ));
    }

    generated_code.push_str("];\n\n");

    // =========================================================================
    // Extract unique tools from evidence.applies_to.tool
    // =========================================================================
    let mut tools: BTreeSet<String> = BTreeSet::new();

    for rule in rules_array {
        if let Some(tool) = rule
            .get("evidence")
            .and_then(|e| e.get("applies_to"))
            .and_then(|a| a.get("tool"))
            .and_then(|t| t.as_str())
            .filter(|t| !t.is_empty())
        {
            tools.insert(tool.to_string());
        }
    }

    // Generate VALID_TOOLS constant
    generated_code
        .push_str("/// Valid tool names derived from rules.json evidence.applies_to.tool.\n");
    generated_code.push_str("/// \n");
    generated_code
        .push_str("/// These are the tools that have at least one rule specifically for them.\n");
    generated_code.push_str("pub const VALID_TOOLS: &[&str] = &[\n");
    for tool in &tools {
        generated_code.push_str(&format!("    \"{}\",\n", escape_str(tool)));
    }
    generated_code.push_str("];\n\n");

    // =========================================================================
    // Derive prefix-to-tool mappings from rule IDs
    // =========================================================================
    // Group rules by their prefix and track:
    // 1. Which tools are specified for rules with this prefix
    // 2. Whether any rule with this prefix has NO tool (making it generic)
    #[derive(Default)]
    struct PrefixInfo {
        tools: BTreeSet<String>,
        has_generic: bool, // true if any rule in this prefix has no tool specified
    }

    let mut prefix_info: BTreeMap<String, PrefixInfo> = BTreeMap::new();

    for rule in rules_array {
        let id = rule["id"].as_str().unwrap_or("");
        let tool = rule
            .get("evidence")
            .and_then(|e| e.get("applies_to"))
            .and_then(|a| a.get("tool"))
            .and_then(|t| t.as_str());

        if let Some(prefix) = extract_rule_prefix(id) {
            let info = prefix_info.entry(prefix).or_default();
            if let Some(tool_name) = tool {
                if !tool_name.is_empty() {
                    info.tools.insert(tool_name.to_string());
                } else {
                    info.has_generic = true;
                }
            } else {
                // No tool specified = generic rule, applies to all tools
                info.has_generic = true;
            }
        }
    }

    // Only include prefixes that:
    // 1. Have exactly one tool specified AND
    // 2. Have NO generic rules (all rules specify that tool)
    generated_code.push_str("/// Mapping of rule ID prefixes to their associated tools.\n");
    generated_code.push_str("/// \n");
    generated_code.push_str(
        "/// Derived from rules.json: for each prefix, this is the tool that all rules\n",
    );
    generated_code
        .push_str("/// with that prefix apply to. Only includes prefixes where ALL rules\n");
    generated_code
        .push_str("/// consistently specify the same tool (excludes generic prefixes).\n");
    generated_code.push_str("pub const TOOL_RULE_PREFIXES: &[(&str, &str)] = &[\n");

    for (prefix, info) in &prefix_info {
        // Only include if: exactly one tool AND no generic rules
        if info.tools.len() == 1 && !info.has_generic {
            let tool = info.tools.iter().next().unwrap();
            generated_code.push_str(&format!(
                "    (\"{}\", \"{}\"),\n",
                escape_str(prefix),
                escape_str(tool)
            ));
        }
    }
    generated_code.push_str("];\n");

    // =========================================================================
    // Extract authoring metadata catalogs from top-level authoring section
    // =========================================================================
    let authoring = rules
        .get("authoring")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let authoring_version = authoring
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0");

    // Validate authoring version (basic semver-like shape)
    let is_valid_version = |version: &str| -> bool {
        !version.is_empty()
            && version.len() <= 32
            && version
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c.is_ascii_alphabetic())
    };

    if !is_valid_version(authoring_version) {
        panic!(
            "authoring.version '{}' is invalid: expected a short semver-like string",
            authoring_version
        );
    }

    let mut authoring_families: BTreeSet<String> = BTreeSet::new();
    if let Some(families) = authoring.get("families").and_then(|f| f.as_array()) {
        for (idx, family) in families.iter().enumerate() {
            let id = family
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    panic!(
                        "authoring.families[{}].id must be a string in rules.json",
                        idx
                    )
                });
            let valid_family = !id.is_empty()
                && id.len() <= 64
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !valid_family {
                panic!(
                    "authoring.families[{}].id '{}' is invalid: use lowercase letters, digits, and hyphens",
                    idx, id
                );
            }
            authoring_families.insert(id.to_string());
        }
    }

    let authoring_json_str = serde_json::to_string(&authoring)
        .expect("BUG: failed to serialize authoring catalog to JSON string");

    generated_code.push_str("\n/// Authoring catalog schema version.\n");
    generated_code.push_str(&format!(
        "pub const AUTHORING_VERSION: &str = \"{}\";\n\n",
        escape_str(authoring_version)
    ));

    generated_code
        .push_str("/// Authoring family IDs derived from rules.json authoring.families.\n");
    generated_code.push_str("pub const AUTHORING_FAMILIES: &[&str] = &[\n");
    for family in &authoring_families {
        generated_code.push_str(&format!("    \"{}\",\n", escape_str(family)));
    }
    generated_code.push_str("];\n\n");

    generated_code.push_str(
        "/// Raw authoring catalog JSON (top-level `authoring` section from rules.json).\n",
    );
    generated_code.push_str(
        "/// This is generated at build time to keep rules.json as the source of truth.\n",
    );
    generated_code.push_str(&format!(
        "pub const AUTHORING_CATALOG_JSON: &str = \"{}\";\n",
        escape_str(&authoring_json_str)
    ));

    // Write to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("rules_data.rs");
    fs::write(&dest_path, generated_code).expect("Failed to write generated rules");
}

/// Extract the rule prefix from a rule ID.
///
/// Examples:
/// - "CC-HK-001" -> "CC-HK-"
/// - "COP-001" -> "COP-"
/// - "AS-001" -> "AS-"
fn extract_rule_prefix(rule_id: &str) -> Option<String> {
    // Find the last hyphen. If it exists and is followed by only digits,
    // we've found our prefix. This is more efficient than splitting into a vector
    // and correctly handles edge cases like trailing hyphens.
    rule_id
        .rsplit_once('-')
        .filter(|(_, suffix)| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .map(|(prefix, _)| format!("{}-", prefix))
}
