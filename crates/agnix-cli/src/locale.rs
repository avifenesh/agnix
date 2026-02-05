//! Locale detection and initialization for agnix CLI.
//!
//! Locale resolution order:
//! 1. `--locale` CLI flag (highest priority)
//! 2. `AGNIX_LOCALE` environment variable
//! 3. `LANG` / `LC_ALL` environment variable
//! 4. System locale detection via `sys-locale`
//! 5. Fallback to "en" (English)

use rust_i18n::set_locale;

/// Supported locales with their display names.
pub const SUPPORTED_LOCALES: &[(&str, &str)] = &[
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

/// Normalize a locale string to match our supported locale codes.
///
/// Examples:
/// - "en_US.UTF-8" -> "en"
/// - "es_ES" -> "es"
/// - "zh_CN.UTF-8" -> "zh-CN"
/// - "zh-Hans" -> "zh-CN"
fn normalize_locale(locale: &str) -> String {
    // Strip encoding suffix (e.g., ".UTF-8")
    let base = locale.split('.').next().unwrap_or(locale);

    // Handle zh variants
    let lower = base.to_lowercase();
    if lower.starts_with("zh") {
        // zh_CN, zh-CN, zh-Hans, zh_Hans -> zh-CN
        if lower.contains("cn") || lower.contains("hans") || lower.contains("simplified") {
            return "zh-CN".to_string();
        }
        // Other zh variants fall through to language-only matching
    }

    // Try exact match first (case-insensitive)
    for &(code, _) in SUPPORTED_LOCALES {
        if base.eq_ignore_ascii_case(code) {
            return code.to_string();
        }
    }

    // Try language-only match (e.g., "es_ES" -> "es")
    let lang = base.split(&['_', '-'][..]).next().unwrap_or(base);
    for &(code, _) in SUPPORTED_LOCALES {
        let code_lang = code.split('-').next().unwrap_or(code);
        if lang.eq_ignore_ascii_case(code_lang) {
            return code.to_string();
        }
    }

    // Not supported, return as-is for the caller to handle
    lang.to_lowercase()
}

/// Check if a locale code is supported.
fn is_supported(locale: &str) -> bool {
    SUPPORTED_LOCALES.iter().any(|&(code, _)| code == locale)
}

/// Print the list of supported locales.
pub fn print_supported_locales() {
    println!("Supported locales:");
    for &(code, name) in SUPPORTED_LOCALES {
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
}
