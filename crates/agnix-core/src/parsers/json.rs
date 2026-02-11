//! JSON parser for hooks and plugin configs
//!
//! ## Security
//!
//! JSON parsing is handled by serde_json which is memory-safe and handles
//! malformed input gracefully (returns errors instead of panicking).

use crate::diagnostics::{CoreError, LintResult, ValidationError};
use serde::de::DeserializeOwned;

/// Parse JSON config file
pub fn parse_json_config<T: DeserializeOwned>(content: &str) -> LintResult<T> {
    let parsed: T = serde_json::from_str(content)
        .map_err(|e| CoreError::Validation(ValidationError::Other(e.into())))?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestConfig {
        name: String,
    }

    #[test]
    fn test_parse_valid_json() {
        let content = r#"{"name": "test"}"#;
        let result: LintResult<TestConfig> = parse_json_config(content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test");
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        let content = r#"{"name": }"#;
        let result: LintResult<TestConfig> = parse_json_config(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_returns_error() {
        let content = "";
        let result: LintResult<TestConfig> = parse_json_config(content);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::Value;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn parse_json_never_panics(content in ".*") {
            // Should never panic on any input - may return error but not panic
            let _: LintResult<Value> = parse_json_config(&content);
        }

        #[test]
        fn parse_json_valid_json_succeeds(
            key in "[a-z]+",
            value in "[a-zA-Z0-9 ]*"
        ) {
            let content = format!(r#"{{"{}": "{}"}}"#, key, value);
            let result: LintResult<Value> = parse_json_config(&content);
            prop_assert!(result.is_ok(), "Valid JSON should parse successfully");
        }

        #[test]
        fn parse_json_nested_succeeds(
            key1 in "[a-z]+",
            key2 in "[a-z]+",
            value in "[a-zA-Z0-9]*"
        ) {
            let content = format!(r#"{{"{}":{{"{}":"{}"}}}}"#, key1, key2, value);
            let result: LintResult<Value> = parse_json_config(&content);
            prop_assert!(result.is_ok(), "Valid nested JSON should parse successfully");
        }
    }
}
