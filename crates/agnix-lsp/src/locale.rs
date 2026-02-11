//! Locale detection and initialization for the LSP server.
//!
//! Uses the same resolution order as the CLI (minus CLI flags):
//! 1. `AGNIX_LOCALE` environment variable
//! 2. `LANG` / `LC_ALL` environment variable
//! 3. System locale via `sys-locale`
//! 4. Fallback to "en" (English)

use agnix_core::i18n::{is_supported, normalize_locale};
use rust_i18n::set_locale;

/// Initialize locale from environment variables.
///
/// Called once at LSP server startup. Config-based locale is handled
/// separately when the workspace config is loaded during `initialize()`.
pub fn init_from_env() {
    let locale = detect_locale();
    set_locale(&locale);
}

/// Re-initialize locale from a config value (e.g., from .agnix.toml).
///
/// Only applies if the locale is supported; otherwise keeps the current locale.
pub fn init_from_config(config_locale: &str) {
    let normalized = normalize_locale(config_locale);
    if is_supported(&normalized) {
        set_locale(&normalized);
    }
}

/// Detect the best locale from the environment.
fn detect_locale() -> String {
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

/// Mutex to serialize tests that modify the process-global locale.
/// Rust tests run in parallel by default, and `rust_i18n::set_locale()` is process-wide,
/// so concurrent modifications cause flaky failures.
#[cfg(test)]
pub(crate) static LOCALE_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    fn test_unsupported_returns_language_code() {
        assert_eq!(normalize_locale("fr_FR"), "fr");
        assert!(!is_supported("fr"));
    }

    #[test]
    fn test_is_supported() {
        assert!(is_supported("en"));
        assert!(is_supported("es"));
        assert!(is_supported("zh-CN"));
        assert!(!is_supported("fr"));
    }

    #[test]
    fn test_init_from_env_does_not_panic() {
        let _guard = LOCALE_MUTEX.lock().unwrap();
        // Just ensure it doesn't panic with whatever environment is set
        init_from_env();
    }

    #[test]
    fn test_init_from_config_supported_and_unsupported() {
        let _guard = LOCALE_MUTEX.lock().unwrap();
        // Ensure known starting state
        set_locale("en");

        // Test supported locale application
        init_from_config("es");
        let current = rust_i18n::locale();
        assert_eq!(&*current, "es");

        // Test unsupported locale preserves current (which is now "es")
        init_from_config("xx-unsupported");
        let current = rust_i18n::locale();
        assert_eq!(
            &*current, "es",
            "unsupported locale should not change current"
        );

        // Reset for other tests
        set_locale("en");
    }
}
