//! CLAUDE.md validation rules

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::regex_util::static_regex;

static GENERIC_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

static_regex!(fn negative_pattern, r"(?i)\b(don't|do\s+not|never|avoid|shouldn't|should\s+not)\b");
static_regex!(fn positive_pattern, r"(?i)\b(instead|rather|prefer|better\s+to|alternative)\b");
static_regex!(fn weak_language_pattern, r"(?i)\b(should|try\s+to|consider|maybe|might\s+want\s+to|could|possibly)\b");
static_regex!(fn critical_section_pattern, r"(?i)^#+\s*.*(critical|important|required|mandatory|rules|must|essential)");
static_regex!(fn critical_keyword_pattern, r"(?i)\b(critical|important|must|required|essential|mandatory|crucial)\b");
static_regex!(fn npm_run_pattern, r"npm\s+run\s+([a-zA-Z0-9_:-]+)");

/// Generic instruction patterns that Claude already knows
pub fn generic_patterns() -> &'static Vec<Regex> {
    GENERIC_PATTERNS.get_or_init(|| {
        const PATTERNS: &[&str] = &[
            r"(?i)\bbe\s+helpful",
            r"(?i)\bbe\s+accurate",
            r"(?i)\bthink\s+step\s+by\s+step",
            r"(?i)\bbe\s+concise",
            r"(?i)\bformat.*properly",
            r"(?i)\bprovide.*clear.*explanations",
            r"(?i)\bmake\s+sure\s+to",
            r"(?i)\balways\s+be",
            // Role-play identity preambles
            r"(?i)^#+\s*(?:you\s+are|your\s+role)\b",
            r"(?i)^\s*-?\s*you\s+are\s+a\s+(?:helpful|expert|senior|skilled|experienced)\s+(?:assistant|ai|large\s+language\s+model)\b",
            // Generic programming principles without project context
            r"(?i)\bfollow\s+(?:best\s+practices|coding\s+standards|clean\s+code)\b",
            r"(?i)\bwrite\s+clean\s+(?:and\s+)?(?:maintainable|readable)\s+code\b",
        ];
        PATTERNS
            .iter()
            .map(|&p| Regex::new(p).unwrap_or_else(|e| panic!("BUG: invalid regex '{}': {}", p, e)))
            .collect()
    })
}

/// Check for generic instructions in content
pub fn find_generic_instructions(content: &str) -> Vec<GenericInstruction> {
    let mut results = Vec::new();
    let patterns = generic_patterns();
    let mut byte_offset: usize = 0;
    let content_len = content.len();

    // Detect line ending type: CRLF (2 bytes) or LF (1 byte)
    let line_ending_len = if content.contains("\r\n") { 2 } else { 1 };

    for (line_num, line) in content.lines().enumerate() {
        let line_start = byte_offset;
        // Calculate the end of line: line length + line ending bytes (if not at end of file)
        let line_end = if byte_offset + line.len() < content_len {
            byte_offset + line.len() + line_ending_len // Include line ending
        } else {
            byte_offset + line.len() // Last line may not have trailing newline
        };

        for pattern in patterns {
            if let Some(mat) = pattern.find(line) {
                results.push(GenericInstruction {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    text: mat.as_str().to_string(),
                    pattern: pattern.as_str().to_string(),
                    start_byte: line_start,
                    end_byte: line_end,
                });
            }
        }

        // Move to next line
        byte_offset = line_end;
    }

    results
}

#[derive(Debug, Clone)]
pub struct GenericInstruction {
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub pattern: String,
    /// Byte offset of the start of the line containing this instruction
    pub start_byte: usize,
    /// Byte offset of the end of the line (including newline if present)
    pub end_byte: usize,
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

/// Find negative instructions without positive alternatives
pub fn find_negative_without_positive(content: &str) -> Vec<NegativeInstruction> {
    let mut results = Vec::new();
    let neg_pattern = negative_pattern();
    let pos_pattern = positive_pattern();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(mat) = neg_pattern.find(line) {
            // Check current line for positive alternative using transition words
            let has_positive_transition = pos_pattern.is_match(line);

            // Check if there's a positive imperative before the negative
            // e.g., "Use X, don't use Y" or "Fetch fresh data, don't cache"
            let has_positive_before = if mat.start() > 0 {
                let before_negative = &line[..mat.start()].trim();
                // Look for imperative verb patterns before the negative
                // Match: starts with verb, contains comma or semicolon before negative
                before_negative.len() > 5
                    && (before_negative.contains(',') || before_negative.contains(';'))
                    && !before_negative.starts_with("//")
                    && !before_negative.starts_with('#')
            } else {
                false
            };

            // Check next line for positive alternative
            let has_positive_next_line = lines
                .get(line_num + 1)
                .is_some_and(|next| pos_pattern.is_match(next));

            if !has_positive_transition && !has_positive_before && !has_positive_next_line {
                results.push(NegativeInstruction {
                    line: line_num + 1,
                    column: mat.start() + 1,
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
    /// Byte offset of the start of the weak constraint word
    pub start_byte: usize,
    /// Byte offset of the end of the weak constraint word
    pub end_byte: usize,
}

/// Find weak constraint language in critical sections
pub fn find_weak_constraints(content: &str) -> Vec<WeakConstraint> {
    let mut results = Vec::new();
    let weak_pattern = weak_language_pattern();
    let section_pattern = critical_section_pattern();
    let mut byte_offset: usize = 0;
    let content_len = content.len();

    // Detect line ending type: CRLF (2 bytes) or LF (1 byte)
    let line_ending_len = if content.contains("\r\n") { 2 } else { 1 };

    let mut current_section: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        let line_start = byte_offset;

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
                    column: mat.start() + 1,
                    text: mat.as_str().to_string(),
                    section: section_name.clone(),
                    start_byte: line_start + mat.start(),
                    end_byte: line_start + mat.end(),
                });
            }
        }

        // Move to next line: line length + line ending bytes (if not at end of file)
        byte_offset = if byte_offset + line.len() < content_len {
            byte_offset + line.len() + line_ending_len
        } else {
            byte_offset + line.len()
        };
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
                    column: mat.start() + 1,
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
    fn test_regex_patterns_compile() {
        let _ = generic_patterns();
        let _ = negative_pattern();
        let _ = positive_pattern();
        let _ = weak_language_pattern();
        let _ = critical_section_pattern();
        let _ = critical_keyword_pattern();
        let _ = npm_run_pattern();
    }

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

    #[test]
    fn test_detect_role_play_header() {
        let content = "# You Are an Expert\nDo this specific task.";
        let results = find_generic_instructions(content);
        assert!(
            results.iter().any(|r| r.pattern.contains("you\\s+are")),
            "Should detect role-play header '# You Are'"
        );
    }

    #[test]
    fn test_detect_role_play_preamble() {
        let content = "You are a helpful assistant for this project.";
        let results = find_generic_instructions(content);
        assert!(
            !results.is_empty(),
            "Should detect 'You are a helpful assistant'"
        );
    }

    #[test]
    fn test_no_false_positive_role_play() {
        // "You are a developer" should not match because "developer" isn't in the descriptor list
        let content = "You are a developer who writes Rust code.";
        let results = find_generic_instructions(content);
        let role_play: Vec<_> = results
            .iter()
            .filter(|r| r.pattern.contains("you\\s+are\\s+a"))
            .collect();
        assert!(
            role_play.is_empty(),
            "Should not match 'You are a developer' (not a generic role-play)"
        );
    }

    #[test]
    fn test_detect_generic_programming_principles() {
        let content = "Follow best practices when writing code.";
        let results = find_generic_instructions(content);
        assert!(
            results
                .iter()
                .any(|r| r.text.to_lowercase().contains("follow best practices")),
            "Should detect 'follow best practices'"
        );

        let content2 = "Write clean and maintainable code.";
        let results2 = find_generic_instructions(content2);
        assert!(
            !results2.is_empty(),
            "Should detect 'write clean and maintainable code'"
        );
    }

    #[test]
    fn test_no_false_positive_specific_standards() {
        // Project-specific references should not match
        let content = "Follow the airbnb style guide for JavaScript.";
        let results = find_generic_instructions(content);
        assert!(
            results.is_empty(),
            "Project-specific style references should not trigger"
        );
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

    #[test]
    fn test_negative_with_positive_before_comma() {
        // Pattern: "DO X, don't do Y" should be accepted
        let content = "Fetch web resources fresh, don't rely on cached data";
        let results = find_negative_without_positive(content);
        assert!(results.is_empty(), "Should recognize positive before comma");

        // Another example
        let content2 = "Use const or let, never use var";
        let results2 = find_negative_without_positive(content2);
        assert!(
            results2.is_empty(),
            "Should recognize positive before comma"
        );

        // But standalone negative should still trigger
        let content3 = "Don't use var";
        let results3 = find_negative_without_positive(content3);
        assert_eq!(results3.len(), 1, "Standalone negative should trigger");
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

    // ===== Byte offset tests for GenericInstruction =====

    #[test]
    fn test_generic_instruction_byte_offsets_single_line() {
        let content = "Be helpful and accurate.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        assert_eq!(inst.start_byte, 0);
        assert_eq!(inst.end_byte, 24); // No trailing newline
    }

    #[test]
    fn test_generic_instruction_byte_offsets_multiline() {
        let content = "Line one.\nBe helpful and accurate.\nLine three.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        // "Be helpful and accurate." is on line 2, starting at byte 10
        assert_eq!(inst.start_byte, 10);
        // Ends at byte 35 (including newline)
        assert_eq!(inst.end_byte, 35);
    }

    #[test]
    fn test_generic_instruction_byte_offsets_last_line_no_newline() {
        let content = "Line one.\nBe helpful and accurate.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        assert_eq!(inst.start_byte, 10);
        // Last line, no trailing newline
        assert_eq!(inst.end_byte, 34);
    }

    #[test]
    fn test_generic_instruction_delete_produces_expected() {
        let content = "Line one.\nBe helpful and accurate.\nLine three.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        let mut modified = content.to_string();
        modified.replace_range(inst.start_byte..inst.end_byte, "");
        assert_eq!(modified, "Line one.\nLine three.");
    }

    // ===== Byte offset tests for WeakConstraint =====

    #[test]
    fn test_weak_constraint_byte_offsets() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);

        let wc = &results[0];
        assert_eq!(wc.text.to_lowercase(), "should");
        // "should" starts at byte 22 (after "# Critical Rules\n\nYou ")
        assert_eq!(wc.start_byte, 22);
        // "should" ends at byte 28
        assert_eq!(wc.end_byte, 28);
    }

    #[test]
    fn test_weak_constraint_replace_produces_expected() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);

        let wc = &results[0];
        let mut modified = content.to_string();
        modified.replace_range(wc.start_byte..wc.end_byte, "must");
        assert_eq!(
            modified,
            "# Critical Rules\n\nYou must follow the coding style."
        );
    }

    #[test]
    fn test_weak_constraint_try_to() {
        let content = "# Critical Rules\n\nTry to follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);

        let wc = &results[0];
        assert_eq!(wc.text.to_lowercase(), "try to");
        // Verify byte offsets allow correct replacement
        let mut modified = content.to_string();
        modified.replace_range(wc.start_byte..wc.end_byte, "must");
        assert_eq!(
            modified,
            "# Critical Rules\n\nmust follow the coding style."
        );
    }

    #[test]
    fn test_weak_constraint_multiple_on_same_line() {
        let content = "# Critical Rules\n\nYou should consider doing this.";
        let results = find_weak_constraints(content);

        // The regex finds only the first match per line
        // In this case, "should" is found first
        assert!(!results.is_empty());
        assert_eq!(results[0].text.to_lowercase(), "should");
    }

    #[test]
    fn test_weak_constraint_multiline() {
        let content = "# Critical Rules\n\nYou should do this.\nYou could do that.";
        let results = find_weak_constraints(content);

        // Two weak constraints on two lines
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].text.to_lowercase(), "should");
        assert_eq!(results[1].text.to_lowercase(), "could");

        // Verify byte offsets are correct for both
        let mut modified = content.to_string();
        // Apply fixes from end to start to avoid offset issues
        modified.replace_range(results[1].start_byte..results[1].end_byte, "must");
        modified.replace_range(results[0].start_byte..results[0].end_byte, "must");
        assert_eq!(
            modified,
            "# Critical Rules\n\nYou must do this.\nYou must do that."
        );
    }

    // ===== CRLF line ending tests =====

    #[test]
    fn test_generic_instruction_byte_offsets_crlf() {
        // CRLF line endings (Windows style)
        let content = "Line one.\r\nBe helpful and accurate.\r\nLine three.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        // "Be helpful and accurate." is on line 2, starting at byte 11 (after "Line one.\r\n")
        assert_eq!(inst.start_byte, 11);
        // Ends at byte 37 (including CRLF)
        assert_eq!(inst.end_byte, 37);
    }

    #[test]
    fn test_generic_instruction_delete_produces_expected_crlf() {
        let content = "Line one.\r\nBe helpful and accurate.\r\nLine three.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        let mut modified = content.to_string();
        modified.replace_range(inst.start_byte..inst.end_byte, "");
        assert_eq!(modified, "Line one.\r\nLine three.");
    }

    #[test]
    fn test_generic_instruction_byte_offsets_crlf_last_line() {
        // CRLF with match on last line (no trailing newline)
        let content = "Line one.\r\nBe helpful and accurate.";
        let results = find_generic_instructions(content);
        assert!(!results.is_empty());

        let inst = &results[0];
        assert_eq!(inst.start_byte, 11);
        // Last line, no trailing CRLF
        assert_eq!(inst.end_byte, 35);
    }

    #[test]
    fn test_weak_constraint_byte_offsets_crlf() {
        let content = "# Critical Rules\r\n\r\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);

        let wc = &results[0];
        assert_eq!(wc.text.to_lowercase(), "should");
        // "should" starts at byte 24 (after "# Critical Rules\r\n\r\nYou ")
        assert_eq!(wc.start_byte, 24);
        // "should" ends at byte 30
        assert_eq!(wc.end_byte, 30);
    }

    #[test]
    fn test_weak_constraint_replace_produces_expected_crlf() {
        let content = "# Critical Rules\r\n\r\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);

        let wc = &results[0];
        let mut modified = content.to_string();
        modified.replace_range(wc.start_byte..wc.end_byte, "must");
        assert_eq!(
            modified,
            "# Critical Rules\r\n\r\nYou must follow the coding style."
        );
    }

    #[test]
    fn test_weak_constraint_multiline_crlf() {
        let content = "# Critical Rules\r\n\r\nYou should do this.\r\nYou could do that.";
        let results = find_weak_constraints(content);

        // Two weak constraints on two lines
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].text.to_lowercase(), "should");
        assert_eq!(results[1].text.to_lowercase(), "could");

        // Verify byte offsets are correct for both
        let mut modified = content.to_string();
        // Apply fixes from end to start to avoid offset issues
        modified.replace_range(results[1].start_byte..results[1].end_byte, "must");
        modified.replace_range(results[0].start_byte..results[0].end_byte, "must");
        assert_eq!(
            modified,
            "# Critical Rules\r\n\r\nYou must do this.\r\nYou must do that."
        );
    }
}
