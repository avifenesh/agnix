//! Shared i18n utilities for locale normalization.
//!
//! This module provides locale normalization and validation functions
//! used by both the CLI and LSP server to ensure consistent behavior.

/// Supported locale codes.
pub const SUPPORTED_LOCALES: &[&str] = &["en", "es", "zh-CN"];

/// Normalize a locale string to match supported locale codes.
///
/// Examples:
/// - "en_US.UTF-8" -> "en"
/// - "es_ES" -> "es"
/// - "zh_CN.UTF-8" -> "zh-CN"
/// - "zh-Hans" -> "zh-CN"
/// - " en " -> "en"
pub fn normalize_locale(locale: &str) -> String {
    // Trim whitespace first
    let locale = locale.trim();

    // Strip encoding suffix (e.g., ".UTF-8")
    let base = locale.split('.').next().unwrap_or(locale);

    // Handle zh variants
    let lower = base.to_lowercase();
    if lower.starts_with("zh")
        && (lower.contains("cn") || lower.contains("hans") || lower.contains("simplified"))
    {
        return "zh-CN".to_string();
    }

    // Try exact match (case-insensitive)
    for &code in SUPPORTED_LOCALES {
        if base.eq_ignore_ascii_case(code) {
            return code.to_string();
        }
    }

    // Try language-only match (e.g., "es_ES" -> "es")
    let lang = base.split(&['_', '-'][..]).next().unwrap_or(base);
    for &code in SUPPORTED_LOCALES {
        let code_lang = code.split('-').next().unwrap_or(code);
        if lang.eq_ignore_ascii_case(code_lang) {
            return code.to_string();
        }
    }

    lang.to_lowercase()
}

/// Check if a locale code is supported.
pub fn is_supported(locale: &str) -> bool {
    SUPPORTED_LOCALES.contains(&locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_english() {
        assert_eq!(normalize_locale("en"), "en");
        assert_eq!(normalize_locale("en_US"), "en");
        assert_eq!(normalize_locale("en_US.UTF-8"), "en");
    }

    #[test]
    fn test_normalize_spanish() {
        assert_eq!(normalize_locale("es"), "es");
        assert_eq!(normalize_locale("es_ES"), "es");
        assert_eq!(normalize_locale("es_ES.UTF-8"), "es");
    }

    #[test]
    fn test_normalize_chinese() {
        assert_eq!(normalize_locale("zh_CN"), "zh-CN");
        assert_eq!(normalize_locale("zh-CN"), "zh-CN");
        assert_eq!(normalize_locale("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(normalize_locale("zh-Hans"), "zh-CN");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_locale(" en "), "en");
        assert_eq!(normalize_locale("  es  "), "es");
        assert_eq!(normalize_locale("\ten\n"), "en");
    }

    #[test]
    fn test_unsupported_locale() {
        assert_eq!(normalize_locale("fr_FR"), "fr");
        assert!(!is_supported("fr"));
    }

    #[test]
    fn test_is_supported() {
        assert!(is_supported("en"));
        assert!(is_supported("es"));
        assert!(is_supported("zh-CN"));
        assert!(!is_supported("fr"));
        assert!(!is_supported("de"));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(normalize_locale("EN"), "en");
        assert_eq!(normalize_locale("ES"), "es");
        assert_eq!(normalize_locale("ZH-cn"), "zh-CN");
    }
}
