//! Gemini CLI ignore file schema helpers
//!
//! Provides validation for .geminiignore files.
//!
//! Validates:
//! - Non-empty content (after stripping comments and whitespace)
//! - Glob pattern syntax (unmatched brackets)

/// An issue found in a .geminiignore file
#[derive(Debug, Clone)]
pub struct GeminiIgnoreIssue {
    /// 1-indexed line number
    pub line: usize,
    /// 0-indexed column
    pub column: usize,
    /// Description of the issue
    pub description: String,
}

/// Validate .geminiignore content
///
/// Checks for:
/// - Empty content (after stripping comments and whitespace)
/// - Lines with unmatched `[` brackets in glob patterns
pub fn validate_geminiignore(content: &str) -> Vec<GeminiIgnoreIssue> {
    let mut issues = Vec::new();

    // Check if content is effectively empty
    let has_content = content.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });

    if !has_content {
        issues.push(GeminiIgnoreIssue {
            line: 1,
            column: 0,
            description: "empty".to_string(),
        });
        return issues;
    }

    // Check each non-comment, non-empty line for syntax issues
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for unmatched brackets in glob patterns
        let open_count = trimmed.chars().filter(|&c| c == '[').count();
        let close_count = trimmed.chars().filter(|&c| c == ']').count();
        if open_count != close_count {
            issues.push(GeminiIgnoreIssue {
                line: line_num + 1,
                column: 0,
                description: format!("syntax_error:{}", trimmed),
            });
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_geminiignore() {
        let content = "node_modules/\n*.log\n.env\n";
        let issues = validate_geminiignore(content);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_valid_with_comments() {
        let content = "# Ignore build artifacts\nbuild/\ndist/\n";
        let issues = validate_geminiignore(content);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_empty_content() {
        let issues = validate_geminiignore("");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].description, "empty");
    }

    #[test]
    fn test_only_comments() {
        let content = "# Comment 1\n# Comment 2\n";
        let issues = validate_geminiignore(content);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].description, "empty");
    }

    #[test]
    fn test_only_whitespace() {
        let content = "   \n   \n";
        let issues = validate_geminiignore(content);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].description, "empty");
    }

    #[test]
    fn test_unmatched_bracket() {
        let content = "*.log\n[unclosed\nnode_modules/\n";
        let issues = validate_geminiignore(content);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line, 2);
        assert!(issues[0].description.starts_with("syntax_error:"));
    }

    #[test]
    fn test_valid_bracket_pattern() {
        let content = "*.[oa]\n*.log\n";
        let issues = validate_geminiignore(content);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_multiple_issues() {
        let content = "[bad\n*.log\n[also-bad\n";
        let issues = validate_geminiignore(content);
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn test_negation_patterns() {
        let content = "*.log\n!important.log\n";
        let issues = validate_geminiignore(content);
        assert!(issues.is_empty());
    }
}
