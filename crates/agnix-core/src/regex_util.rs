//! Utility macro for declaring lazily-compiled static regex patterns.
//!
//! Every regex literal in the crate should go through [`static_regex!`] so that
//! an invalid pattern produces a descriptive panic (with the pattern text)
//! instead of a bare `.unwrap()`.

/// Declare a module-private function that returns `&'static regex::Regex`,
/// backed by a `std::sync::OnceLock`. The pattern is compiled on first access
/// and cached forever.
///
/// The calling module must have `use regex::Regex;` in scope.
///
/// # Panics
///
/// Panics at runtime on first call if `$pattern` is not a valid regex. The
/// panic message includes the offending pattern text for easy debugging.
///
/// # Example
///
/// ```ignore
/// use regex::Regex;
/// use crate::regex_util::static_regex;
///
/// static_regex!(fn my_pattern, r"^hello\s+world$");
///
/// let re = my_pattern();
/// assert!(re.is_match("hello  world"));
/// ```
macro_rules! static_regex {
    (fn $fname:ident, $pattern:expr) => {
        fn $fname() -> &'static Regex {
            static STORE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
            STORE.get_or_init(|| {
                Regex::new($pattern).expect(concat!("BUG: invalid static regex: ", $pattern))
            })
        }
    };
}
pub(crate) use static_regex;
