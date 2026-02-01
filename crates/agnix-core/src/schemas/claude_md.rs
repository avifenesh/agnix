//! CLAUDE.md validation rules

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

static GENERIC_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
static NEGATIVE_PATTERN: OnceLock<Regex> = OnceLock::new();
static POSITIVE_PATTERN: OnceLock<Regex> = OnceLock::new();
static WEAK_LANGUAGE_PATTERN: OnceLock<Regex> = OnceLock::new();
static CRITICAL_SECTION_PATTERN: OnceLock<Regex> = OnceLock::new();
static CRITICAL_KEYWORD_PATTERN: OnceLock<Regex> = OnceLock::new();
static NPM_RUN_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Generic instruction patterns that Claude already knows
pub fn generic_patterns() -> &'static Vec<Regex> {
    GENERIC_PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\bbe\s+helpful").unwrap(),
            Regex::new(r"(?i)\bbe\s+accurate").unwrap(),
            Regex::new(r"(?i)\bthink\s+step\s+by\s+step").unwrap(),
            Regex::new(r"(?i)\bbe\s+concise").unwrap(),
            Regex::new(r"(?i)\bformat.*properly").unwrap(),
            Regex::new(r"(?i)\bprovide.*clear.*explanations").unwrap(),
            Regex::new(r"(?i)\bmake\s+sure\s+to").unwrap(),
            Regex::new(r"(?i)\balways\s+be").unwrap(),
        ]
    })
}

/// Check for generic instructions in content
pub fn find_generic_instructions(content: &str) -> Vec<GenericInstruction> {
    let mut results = Vec::new();
    let patterns = generic_patterns();

    for (line_num, line) in content.lines().enumerate() {
        for pattern in patterns {
            if let Some(mat) = pattern.find(line) {
                results.push(GenericInstruction {
                    line: line_num + 1,
                    column: mat.start(),
                    text: mat.as_str().to_string(),
                    pattern: pattern.as_str().to_string(),
                });
            }
        }
    }

    results
}

#[derive(Debug, Clone)]
pub struct GenericInstruction {
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub pattern: String,
}

// ============================================================================
// CC-MEM-009: Token Count Exceeded
// ============================================================================

/// Result when token count exceeds limit
#[derive(Debug, Clone)]
pub struct TokenCountExceeded {
    pub char_count: usize,
    pub estimated_tokens: usize,
    pub limit: usize,
}

/// Check if content exceeds token limit (~1500 tokens = ~6000 chars)
/// Returns Some if exceeded, None if within limit
pub fn check_token_count(content: &str) -> Option<TokenCountExceeded> {
    let char_count = content.len();
    let estimated_tokens = char_count / 4; // Rough approximation: 4 chars per token
    let limit = 1500;

    if estimated_tokens > limit {
        Some(TokenCountExceeded {
            char_count,
            estimated_tokens,
            limit,
        })
    } else {
        None
    }
}

// ============================================================================
// CC-MEM-006: Negative Without Positive
// ============================================================================

#[derive(Debug, Clone)]
pub struct NegativeInstruction {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

fn negative_pattern() -> &'static Regex {
    NEGATIVE_PATTERN.get_or_init(|| {
        // Match common negative instruction patterns
        Regex::new(r"(?i)\b(don't|do\s+not|never|avoid|shouldn't|should\s+not)\b").unwrap()
    })
}

fn positive_pattern() -> &'static Regex {
    POSITIVE_PATTERN.get_or_init(|| {
        // Match positive alternatives - words that indicate a suggested approach
        Regex::new(r"(?i)\b(instead|rather|prefer|better\s+to|alternative)\b").unwrap()
    })
}

/// Find negative instructions without positive alternatives
pub fn find_negative_without_positive(content: &str) -> Vec<NegativeInstruction> {
    let mut results = Vec::new();
    let neg_pattern = negative_pattern();
    let pos_pattern = positive_pattern();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(mat) = neg_pattern.find(line) {
            // Check current line for positive alternative
            let has_positive_same_line = pos_pattern.is_match(line);

            // Check next line for positive alternative
            let has_positive_next_line = lines
                .get(line_num + 1)
                .is_some_and(|next| pos_pattern.is_match(next));

            if !has_positive_same_line && !has_positive_next_line {
                results.push(NegativeInstruction {
                    line: line_num + 1,
                    column: mat.start(),
                    text: mat.as_str().to_string(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-007: Weak Constraint Language
// ============================================================================

#[derive(Debug, Clone)]
pub struct WeakConstraint {
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub section: String,
}

fn weak_language_pattern() -> &'static Regex {
    WEAK_LANGUAGE_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)\b(should|try\s+to|consider|maybe|might\s+want\s+to|could|possibly)\b")
            .unwrap()
    })
}

fn critical_section_pattern() -> &'static Regex {
    CRITICAL_SECTION_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)^#+\s*.*(critical|important|required|mandatory|rules|must|essential)")
            .unwrap()
    })
}

/// Find weak constraint language in critical sections
pub fn find_weak_constraints(content: &str) -> Vec<WeakConstraint> {
    let mut results = Vec::new();
    let weak_pattern = weak_language_pattern();
    let section_pattern = critical_section_pattern();

    let mut current_section: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        // Check if this is a header line
        if line.starts_with('#') {
            if section_pattern.is_match(line) {
                current_section = Some(line.trim_start_matches('#').trim().to_string());
            } else {
                // New non-critical header ends the critical section
                current_section = None;
            }
        }

        // Check for weak language in critical sections
        if let Some(section_name) = &current_section {
            if let Some(mat) = weak_pattern.find(line) {
                results.push(WeakConstraint {
                    line: line_num + 1,
                    column: mat.start(),
                    text: mat.as_str().to_string(),
                    section: section_name.clone(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-008: Critical Content in Middle
// ============================================================================

#[derive(Debug, Clone)]
pub struct CriticalInMiddle {
    pub line: usize,
    pub column: usize,
    pub keyword: String,
    pub position_percent: f64,
}

fn critical_keyword_pattern() -> &'static Regex {
    CRITICAL_KEYWORD_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)\b(critical|important|must|required|essential|mandatory|crucial)\b")
            .unwrap()
    })
}

/// Find critical content positioned in the middle of the document (40-60%)
///
/// Based on "Lost in the Middle" research (Liu et al., 2023, TACL):
/// LLMs have lower recall for content in the middle of documents, but better
/// recall for content at the START and END. The 40-60% range is specifically
/// the "lost in the middle" zone. Content at 70%+ (near the end) is actually
/// well-recalled, so we intentionally only flag the middle zone.
pub fn find_critical_in_middle(content: &str) -> Vec<CriticalInMiddle> {
    let mut results = Vec::new();
    let pattern = critical_keyword_pattern();
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if total_lines < 10 {
        // Too short to meaningfully apply this rule
        return results;
    }

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(mat) = pattern.find(line) {
            let position_percent = (line_num as f64 / total_lines as f64) * 100.0;

            // Flag if in the middle 40-60% of the document (lost in the middle zone)
            if position_percent > 40.0 && position_percent < 60.0 {
                results.push(CriticalInMiddle {
                    line: line_num + 1,
                    column: mat.start(),
                    keyword: mat.as_str().to_string(),
                    position_percent,
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-004: Invalid npm Script Reference
// ============================================================================

#[derive(Debug, Clone)]
pub struct NpmScriptReference {
    pub line: usize,
    pub column: usize,
    pub script_name: String,
}

fn npm_run_pattern() -> &'static Regex {
    NPM_RUN_PATTERN.get_or_init(|| Regex::new(r"npm\s+run\s+([a-zA-Z0-9_:-]+)").unwrap())
}

/// Extract npm script references from content
pub fn extract_npm_scripts(content: &str) -> Vec<NpmScriptReference> {
    let mut results = Vec::new();
    let pattern = npm_run_pattern();

    for (line_num, line) in content.lines().enumerate() {
        for cap in pattern.captures_iter(line) {
            if let Some(script_match) = cap.get(1) {
                results.push(NpmScriptReference {
                    line: line_num + 1,
                    column: cap.get(0).map(|m| m.start()).unwrap_or(0),
                    script_name: script_match.as_str().to_string(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-010: README Duplication
// ============================================================================

/// Calculate text overlap between two texts as a percentage (0.0 - 1.0)
/// Uses word-set Jaccard similarity
pub fn calculate_text_overlap(text1: &str, text2: &str) -> f64 {
    // Normalize and extract words
    let text1_lower = text1.to_lowercase();
    let text2_lower = text2.to_lowercase();

    let words1: HashSet<&str> = text1_lower
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .collect();

    let words2: HashSet<&str> = text2_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    // Jaccard similarity: intersection / union
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Result when README duplication is detected
#[derive(Debug, Clone)]
pub struct ReadmeDuplication {
    pub overlap_percent: f64,
    pub threshold: f64,
}

/// Check if content duplicates README beyond threshold
pub fn check_readme_duplication(claude_md: &str, readme: &str) -> Option<ReadmeDuplication> {
    let overlap = calculate_text_overlap(claude_md, readme);
    let threshold = 0.40; // 40% overlap threshold

    if overlap > threshold {
        Some(ReadmeDuplication {
            overlap_percent: overlap * 100.0,
            threshold: threshold * 100.0,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_generic_instructions() {
        let content = "Be helpful and accurate when responding.\nUse project-specific guidelines.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());
        assert!(results[0].text.to_lowercase().contains("helpful"));
    }

    #[test]
    fn test_no_generic_instructions() {
        let content = "Use the coding style defined in .editorconfig\nFollow team conventions";
        let results = find_generic_instructions(content);
        assert!(results.is_empty());
    }

    // CC-MEM-009 tests
    #[test]
    fn test_check_token_count_under_limit() {
        let content = "Short content that is well under the limit.";
        assert!(check_token_count(content).is_none());
    }

    #[test]
    fn test_check_token_count_over_limit() {
        // Create content > 6000 chars (1500 tokens * 4 chars/token)
        let content = "x".repeat(6100);
        let result = check_token_count(&content);
        assert!(result.is_some());
        let exceeded = result.unwrap();
        assert!(exceeded.estimated_tokens > 1500);
        assert_eq!(exceeded.limit, 1500);
    }

    // CC-MEM-006 tests
    #[test]
    fn test_find_negative_without_positive() {
        let content = "Don't use var in JavaScript.\nNever use global variables.";
        let results = find_negative_without_positive(content);
        assert_eq!(results.len(), 2);
        assert!(results[0].text.to_lowercase().contains("don"));
    }

    #[test]
    fn test_negative_with_positive_same_line() {
        let content = "Don't use var, instead prefer const or let.";
        let results = find_negative_without_positive(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_negative_with_positive_next_line() {
        let content = "Don't use var.\nUse const instead of var.";
        let results = find_negative_without_positive(content);
        assert!(results.is_empty());
    }

    // CC-MEM-007 tests
    #[test]
    fn test_find_weak_constraints_in_critical() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text.to_lowercase(), "should");
    }

    #[test]
    fn test_find_weak_constraints_outside_critical() {
        let content = "# General Guidelines\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_weak_constraints_section_ends() {
        let content =
            "# Critical Rules\n\nMust follow style.\n\n# Other\n\nYou should do this too.";
        let results = find_weak_constraints(content);
        // "should" is in non-critical section, so no results
        assert!(results.is_empty());
    }

    // CC-MEM-008 tests
    #[test]
    fn test_find_critical_in_middle() {
        // Create 20 lines with "critical" at line 10 (50%)
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle(&content);
        assert_eq!(results.len(), 1);
        assert!(results[0].position_percent > 40.0);
        assert!(results[0].position_percent < 60.0);
    }

    #[test]
    fn test_critical_at_top() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[1] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle(&content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_critical_at_bottom() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[18] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle(&content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_short_document_no_critical_middle() {
        let content = "Critical info here.\nAnother line.";
        let results = find_critical_in_middle(content);
        // Document too short (< 10 lines)
        assert!(results.is_empty());
    }

    // CC-MEM-004 tests
    #[test]
    fn test_extract_npm_scripts() {
        let content = "Run tests with npm run test\nBuild with npm run build";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].script_name, "test");
        assert_eq!(results[1].script_name, "build");
    }

    #[test]
    fn test_extract_npm_scripts_with_colon() {
        let content = "Run npm run test:unit for unit tests";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].script_name, "test:unit");
    }

    #[test]
    fn test_no_npm_scripts() {
        let content = "Use cargo test for testing.";
        let results = extract_npm_scripts(content);
        assert!(results.is_empty());
    }

    // CC-MEM-010 tests
    #[test]
    fn test_calculate_text_overlap_identical() {
        let text = "This is some sample text with enough words to test overlap calculation.";
        let overlap = calculate_text_overlap(text, text);
        assert!((overlap - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_text_overlap_different() {
        let text1 = "This project uses Rust for performance.";
        let text2 = "Python is great for machine learning.";
        let overlap = calculate_text_overlap(text1, text2);
        assert!(overlap < 0.3);
    }

    #[test]
    fn test_check_readme_duplication_detected() {
        let claude_md =
            "This is a project about Rust validation. It validates agent configurations.";
        let readme = "This is a project about Rust validation. It validates agent configurations.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_some());
    }

    #[test]
    fn test_check_readme_duplication_not_detected() {
        let claude_md = "Project-specific instructions for Claude. Focus on these guidelines.";
        let readme = "Welcome to the project. Installation: npm install. Usage: npm start.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_none());
    }
}
