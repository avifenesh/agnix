//! Locale detection and initialization for agnix CLI.
//!
//! Locale resolution order:
//! 1. `--locale` CLI flag (highest priority)
//! 2. `AGNIX_LOCALE` environment variable
//! 3. `LANG` / `LC_ALL` environment variable
//! 4. System locale detection via `sys-locale`
//! 5. Fallback to "en" (English)

use agnix_core::i18n::{is_supported, normalize_locale};
use rust_i18n::set_locale;

/// Supported locales with their display names (for CLI display only).
const SUPPORTED_LOCALES_DISPLAY: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Spanish / Espanol"),
    ("zh-CN", "Chinese Simplified / Zhongwen"),
];

/// Detect the best locale from the environment.
///
/// Checks (in order):
/// 1. `AGNIX_LOCALE` environment variable
/// 2. `LANG` / `LC_ALL` environment variable (parsed to language code)
/// 3. System locale via `sys-locale`
/// 4. Falls back to "en"
pub fn detect_locale() -> String {
    // 1. AGNIX_LOCALE env var
    if let Ok(locale) = std::env::var("AGNIX_LOCALE") {
        let normalized = normalize_locale(&locale);
        if is_supported(&normalized) {
            return normalized;
        }
    }

    // 2. LANG / LC_ALL
    if let Ok(lang) = std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")) {
        let normalized = normalize_locale(&lang);
        if is_supported(&normalized) {
            return normalized;
        }
    }

    // 3. System locale
    if let Some(locale) = sys_locale::get_locale() {
        let normalized = normalize_locale(&locale);
        if is_supported(&normalized) {
            return normalized;
        }
    }

    // 4. Fallback
    "en".to_string()
}

/// Initialize the locale for the application.
///
/// Resolution order:
/// 1. `cli_locale` from `--locale` flag (highest priority)
/// 2. `config_locale` from `.agnix.toml` locale field
/// 3. Auto-detection (env vars, system locale, fallback to "en")
pub fn init(cli_locale: Option<&str>, config_locale: Option<&str>) {
    let explicit = cli_locale.or(config_locale);
    let locale = if let Some(l) = explicit {
        let normalized = normalize_locale(l);
        if is_supported(&normalized) {
            normalized
        } else {
            eprintln!(
                "Warning: unsupported locale '{}', falling back to 'en'",
                l
            );
            "en".to_string()
        }
    } else {
        detect_locale()
    };

    set_locale(&locale);
}

/// Print the list of supported locales.
pub fn print_supported_locales() {
    println!("Supported locales:");
    for &(code, name) in SUPPORTED_LOCALES_DISPLAY {
        println!("  {:<8} {}", code, name);
    }
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
    fn test_unsupported_locale() {
        // Returns language code even if not supported
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

    // ===== Edge case tests =====

    #[test]
    fn test_empty_string_locale() {
        let result = normalize_locale("");
        // Empty string normalizes to empty, which is not supported
        assert!(!is_supported(&result));
    }

    #[test]
    fn test_whitespace_only_locale() {
        let result = normalize_locale("   ");
        assert!(!is_supported(&result));
    }

    #[test]
    fn test_very_long_locale_string() {
        let long_locale = "en".to_string() + &"x".repeat(1000);
        let result = normalize_locale(&long_locale);
        // Should not match any supported locale
        assert!(!is_supported(&result));
    }

    #[test]
    fn test_special_characters_in_locale() {
        let result = normalize_locale("@#$%");
        assert!(!is_supported(&result));
    }

    #[test]
    fn test_locale_with_only_encoding() {
        // Just ".UTF-8" with no language
        let result = normalize_locale(".UTF-8");
        assert!(!is_supported(&result));
    }

    #[test]
    fn test_init_with_unsupported_locale_falls_back() {
        // This should print a warning and fall back to "en"
        init(Some("xx-YY"), None);
        // After init, locale should be "en" since "xx-YY" is unsupported
        let current = rust_i18n::locale();
        assert_eq!(&*current, "en");
    }

    #[test]
    fn test_init_with_empty_string_falls_back() {
        init(Some(""), None);
        let current = rust_i18n::locale();
        assert_eq!(&*current, "en");
    }

    #[test]
    fn test_normalize_case_insensitive() {
        assert_eq!(normalize_locale("EN"), "en");
        assert_eq!(normalize_locale("ES"), "es");
        assert_eq!(normalize_locale("ZH-cn"), "zh-CN");
    }
}
