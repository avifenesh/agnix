//! Prompt engineering validation schema helpers
//!
//! Provides detection functions for:
//! - PE-001: Critical content in middle ("lost in the middle")
//! - PE-002: Chain-of-thought phrases on simple tasks
//! - PE-003: Weak imperative language in critical sections
//! - PE-004: Ambiguous instructions
//! - PE-005: Redundant generic instructions
//! - PE-006: Negative-only instructions without positive alternative
//!
//! ## Security
//!
//! This module includes size limits to prevent ReDoS (Regular Expression Denial
//! of Service) attacks. Functions that use regex will return early for oversized
//! input.

use regex::Regex;

use crate::parsers::markdown::MAX_REGEX_INPUT_SIZE;
use crate::regex_util::static_regex;

static_regex!(fn critical_keyword_pattern, r"(?i)\b(critical|important|must|required|essential|mandatory|crucial|never|always)\b");
static_regex!(fn cot_phrase_pattern, r"(?i)\b(think\s+step\s+by\s+step|let'?s\s+think|reason\s+through|break\s+(?:it\s+)?down\s+into\s+steps|work\s+through\s+this\s+(?:step\s+by\s+step|systematically))\b");
static_regex!(fn simple_task_indicator_pattern, r"(?i)\b(read\s+(?:the\s+)?file|write\s+(?:the\s+)?file|copy\s+(?:the\s+)?file|move\s+(?:the\s+)?file|delete\s+(?:the\s+)?file|list\s+files|run\s+(?:the\s+)?(?:command|script)|execute\s+(?:the\s+)?(?:command|script)|format\s+(?:the\s+)?(?:code|output)|rename\s+(?:the\s+)?file|create\s+(?:a\s+)?(?:file|directory|folder)|check\s+(?:if|whether)\s+(?:file|directory)\s+exists)\b");
static_regex!(fn weak_language_pattern, r"(?i)\b(should|try\s+to|consider|maybe|might|could|possibly|preferably|ideally|optionally)\b");
static_regex!(fn critical_section_pattern, r"(?i)^#+\s*.*\b(critical|important|required|mandatory|rules|must|essential|security|danger)\b");
static_regex!(fn ambiguous_term_pattern, r"(?i)\b(usually|sometimes|if\s+possible|when\s+appropriate|as\s+needed|often|occasionally|generally|typically|normally|frequently|regularly|commonly)\b");

// ============================================================================
// PE-001: Critical Content in Middle ("Lost in the Middle")
// ============================================================================

/// Critical content found in the middle zone of document
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
/// the "lost in the middle" zone.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_critical_in_middle_pe(content: &str) -> Vec<CriticalInMiddle> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

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
            if (40.0..60.0).contains(&position_percent) {
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
// PE-002: Chain-of-Thought on Simple Tasks
// ============================================================================

/// Chain-of-thought phrase found on a simple task
#[derive(Debug, Clone)]
pub struct CotOnSimpleTask {
    pub line: usize,
    pub column: usize,
    pub phrase: String,
    pub task_indicator: String,
}

/// Find chain-of-thought phrases used on simple tasks
///
/// Research shows that CoT can actually hurt performance on simple, direct tasks
/// that don't require multi-step reasoning (Wei et al., 2022).
///
/// Only flags CoT phrases that are within proximity (5 lines) of a simple task indicator
/// to avoid false positives when complex and simple tasks are in the same document.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_cot_on_simple_tasks(content: &str) -> Vec<CotOnSimpleTask> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let cot_pattern = cot_phrase_pattern();
    let simple_pattern = simple_task_indicator_pattern();

    // Collect all simple task indicators with their line numbers
    let simple_tasks: Vec<_> = content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            simple_pattern
                .find(line)
                .map(|mat| (line_num, mat.as_str().to_string()))
        })
        .collect();

    if simple_tasks.is_empty() {
        return results;
    }

    // Find CoT phrases and check proximity to simple task indicators
    for (line_num, line) in content.lines().enumerate() {
        if let Some(mat) = cot_pattern.find(line) {
            // Only flag if CoT is within 5 lines of a simple task indicator
            for (task_line, task) in &simple_tasks {
                let distance = if line_num > *task_line {
                    line_num - task_line
                } else {
                    task_line - line_num
                };

                // Proximity threshold: 5 lines
                if distance <= 5 {
                    results.push(CotOnSimpleTask {
                        line: line_num + 1,
                        column: mat.start() + 1,
                        phrase: mat.as_str().to_string(),
                        task_indicator: task.clone(),
                    });
                    break; // Only report once per CoT phrase
                }
            }
        }
    }

    results
}

// ============================================================================
// PE-003: Weak Imperative Language in Critical Sections
// ============================================================================

/// Weak language found in critical section
#[derive(Debug, Clone)]
pub struct WeakLanguageInCritical {
    pub line: usize,
    pub column: usize,
    pub weak_term: String,
    pub section_name: String,
    /// Byte offset of the weak term in the full content
    pub byte_offset: usize,
}

/// Find weak imperative language in critical sections
///
/// Critical sections should use strong language (must/always/never) rather than
/// weak language (should/try/consider) to ensure compliance.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_weak_imperative_language(content: &str) -> Vec<WeakLanguageInCritical> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let weak_pattern = weak_language_pattern();
    let section_pattern = critical_section_pattern();

    let mut current_section: Option<String> = None;
    let mut byte_pos = 0usize;

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
                results.push(WeakLanguageInCritical {
                    line: line_num + 1,
                    column: mat.start() + 1, // 1-indexed for diagnostics
                    weak_term: mat.as_str().to_string(),
                    section_name: section_name.clone(),
                    byte_offset: byte_pos + mat.start(),
                });
            }
        }

        // Advance byte_pos past this line plus its actual line terminator
        byte_pos += line.len();
        if content.as_bytes().get(byte_pos) == Some(&b'\r') {
            byte_pos += 1;
        }
        if content.as_bytes().get(byte_pos) == Some(&b'\n') {
            byte_pos += 1;
        }
    }

    results
}

// ============================================================================
// PE-004: Ambiguous Instructions
// ============================================================================

/// Ambiguous instruction found
#[derive(Debug, Clone)]
pub struct AmbiguousInstruction {
    pub line: usize,
    pub column: usize,
    pub term: String,
    pub context: String,
}

/// Find ambiguous terms in instructions
///
/// Instructions should be specific and measurable. Terms like "usually" or
/// "if possible" create ambiguity about when the instruction applies.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_ambiguous_instructions(content: &str) -> Vec<AmbiguousInstruction> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let pattern = ambiguous_term_pattern();
    let mut in_code_block = false;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();

        // Track fenced code block state
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        // Skip content inside code blocks
        if in_code_block {
            continue;
        }

        // Skip comment lines and shebang
        if trimmed.starts_with("//") || trimmed.starts_with("#!") {
            continue;
        }

        for mat in pattern.find_iter(line) {
            // Skip ambiguous terms inside parentheses - these are typically
            // descriptive/explanatory text, not instructions.
            // e.g., "linting errors (usually part of the build)" is describing, not instructing
            let before = &line[..mat.start()];
            let after = &line[mat.end()..];
            if before
                .rfind('(')
                .is_some_and(|p| before[p..].find(')').is_none())
                && after.find(')').is_some()
            {
                continue;
            }

            // Extract context using UTF-8 safe slicing to avoid panics on multi-byte chars
            let target_start = mat.start().saturating_sub(20);
            let target_end = (mat.end() + 20).min(line.len());

            let start = line
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= target_start)
                .last()
                .unwrap_or(0);
            let end = line
                .char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= target_end)
                .unwrap_or(line.len());
            let context = line[start..end].to_string();

            results.push(AmbiguousInstruction {
                line: line_num + 1,
                column: mat.start() + 1,
                term: mat.as_str().to_string(),
                context,
            });
        }
    }

    results
}

// ============================================================================
// PE-005: Redundant Generic Instructions
// ============================================================================

static_regex!(fn redundant_instruction_pattern, r"(?i)\b(be helpful|be accurate|be concise|follow instructions|do your best|try your hardest|respond accurately|answer correctly|be thorough|be detailed|be precise|be clear|be professional|be consistent|be efficient)\b");

/// Redundant generic instruction found in content
#[derive(Debug, Clone)]
pub struct RedundantInstruction {
    pub line: usize,
    pub column: usize,
    pub phrase: String,
    /// Byte offset of the phrase in the full content
    pub byte_offset: usize,
    /// Byte length of the matched phrase
    pub byte_len: usize,
}

/// Find redundant generic instructions that LLMs already follow by default
///
/// Instructions like "be helpful" or "be accurate" waste context window tokens
/// without adding value, since LLMs already behave this way by default.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_redundant_instructions(content: &str) -> Vec<RedundantInstruction> {
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let pattern = redundant_instruction_pattern();
    let mut in_code_block = false;
    let mut byte_pos = 0usize;

    for (line_num, line) in content.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            byte_pos += line.len();
            if content.as_bytes().get(byte_pos) == Some(&b'\r') {
                byte_pos += 1;
            }
            if content.as_bytes().get(byte_pos) == Some(&b'\n') {
                byte_pos += 1;
            }
            continue;
        }
        if in_code_block {
            byte_pos += line.len();
            if content.as_bytes().get(byte_pos) == Some(&b'\r') {
                byte_pos += 1;
            }
            if content.as_bytes().get(byte_pos) == Some(&b'\n') {
                byte_pos += 1;
            }
            continue;
        }

        for mat in pattern.find_iter(line) {
            results.push(RedundantInstruction {
                line: line_num + 1,
                column: mat.start() + 1,
                phrase: mat.as_str().to_string(),
                byte_offset: byte_pos + mat.start(),
                byte_len: mat.as_str().len(),
            });
        }

        byte_pos += line.len();
        if content.as_bytes().get(byte_pos) == Some(&b'\r') {
            byte_pos += 1;
        }
        if content.as_bytes().get(byte_pos) == Some(&b'\n') {
            byte_pos += 1;
        }
    }

    results
}

// ============================================================================
// PE-006: Negative-Only Instructions
// ============================================================================

static_regex!(fn negative_only_pattern, r"(?i)^[*\-]?\s*(don't|do not|never|avoid|refrain from)\b");
static_regex!(fn positive_alternative_pattern, r"(?i)\b(instead|rather|prefer|use\s+\S+\s+instead|better to|should\s+\S+\s+instead)\b");

/// Negative-only instruction found in content
#[derive(Debug, Clone)]
pub struct NegativeOnlyInstruction {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

/// Find negative instructions that lack a positive alternative
///
/// Instructions are more effective when they tell the LLM what to do
/// instead of only what not to do. For example, "Don't use global variables.
/// Instead, pass values as function parameters." is better than just
/// "Don't use global variables."
///
/// Checks 3 lines (current + 2 following) for a positive alternative.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_negative_only_instructions(content: &str) -> Vec<NegativeOnlyInstruction> {
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let neg_pattern = negative_only_pattern();
    let pos_pattern = positive_alternative_pattern();
    let lines: Vec<&str> = content.lines().collect();
    let mut results = Vec::new();
    let mut in_code_block = false;

    for (line_num, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        if let Some(mat) = neg_pattern.find(line) {
            // Check current line and next 2 lines for positive alternative,
            // but skip lines inside fenced code blocks
            let window_end = (line_num + 3).min(lines.len());
            let mut window_in_code = false;
            let has_positive = lines[line_num..window_end].iter().any(|l| {
                if l.trim_start().starts_with("```") {
                    window_in_code = !window_in_code;
                    return false;
                }
                if window_in_code {
                    return false;
                }
                pos_pattern.is_match(l)
            });

            if !has_positive {
                results.push(NegativeOnlyInstruction {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    text: line.trim().to_string(),
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = critical_keyword_pattern();
        let _ = cot_phrase_pattern();
        let _ = simple_task_indicator_pattern();
        let _ = weak_language_pattern();
        let _ = critical_section_pattern();
        let _ = ambiguous_term_pattern();
        let _ = redundant_instruction_pattern();
        let _ = negative_only_pattern();
        let _ = positive_alternative_pattern();
    }

    // ===== PE-001: Critical Content in Middle =====

    #[test]
    fn test_find_critical_in_middle() {
        // Create 20 lines with "critical" at line 10 (50%)
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        assert_eq!(results.len(), 1);
        assert!(results[0].position_percent > 40.0);
        assert!(results[0].position_percent < 60.0);
        assert_eq!(results[0].keyword.to_lowercase(), "critical");
    }

    #[test]
    fn test_critical_at_top_no_issue() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[1] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_critical_at_bottom_no_issue() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[18] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_short_document_skipped() {
        let content = "Critical info here.\nAnother line.";
        let results = find_critical_in_middle_pe(content);
        // Document too short (< 10 lines)
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_keywords_in_middle() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[9] = "This is important and essential.".to_string();
        lines[10] = "This is critical and mandatory.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        // Should find multiple keywords in the middle zone
        assert!(results.len() >= 2);
    }

    // ===== PE-002: Chain-of-Thought on Simple Tasks =====

    #[test]
    fn test_cot_on_simple_read_file() {
        let content = r#"# Read File Skill

When the user asks to read the file, think step by step:
1. First check if file exists
2. Then read contents
"#;
        let results = find_cot_on_simple_tasks(content);
        assert_eq!(results.len(), 1);
        assert!(
            results[0]
                .phrase
                .to_lowercase()
                .contains("think step by step")
        );
    }

    #[test]
    fn test_cot_on_simple_copy_file() {
        let content = r#"# Copy File Utility

Let's think through copying the file:
- Source path
- Destination path
"#;
        let results = find_cot_on_simple_tasks(content);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_no_cot_on_complex_task() {
        let content = r#"# Code Review Skill

When reviewing code, think step by step:
1. Check for security issues
2. Verify logic correctness
3. Assess performance
"#;
        // This has CoT but is not a simple task, so no matches
        let results = find_cot_on_simple_tasks(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_simple_task_without_cot() {
        let content = r#"# Read File Skill

Read the file and return its contents.
"#;
        // Simple task but no CoT, so no issue
        let results = find_cot_on_simple_tasks(content);
        assert!(results.is_empty());
    }

    // ===== PE-003: Weak Imperative Language =====

    #[test]
    fn test_weak_language_in_critical_section() {
        let content = r#"# Critical Rules

You should follow the coding style.
Code could be formatted better.
"#;
        let results = find_weak_imperative_language(content);
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|r| r.weak_term.to_lowercase() == "should")
        );
        assert!(
            results
                .iter()
                .any(|r| r.weak_term.to_lowercase() == "could")
        );
    }

    #[test]
    fn test_weak_language_outside_critical_section() {
        let content = r#"# General Guidelines

You should follow the coding style.
"#;
        // Not in a critical section
        let results = find_weak_imperative_language(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_weak_language_section_boundary() {
        let content = r#"# Important Security Rules

You should sanitize inputs.

# Other Info

You could do this too.
"#;
        let results = find_weak_imperative_language(content);
        // Only "should" in critical section should be flagged
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].weak_term.to_lowercase(), "should");
    }

    #[test]
    fn test_multiple_critical_sections() {
        let content = r#"# Critical Rules

You should do A.

# General Section

Normal content.

# Mandatory Requirements

You might want to consider B.
"#;
        let results = find_weak_imperative_language(content);
        assert_eq!(results.len(), 2);
    }

    // ===== PE-004: Ambiguous Instructions =====

    #[test]
    fn test_find_ambiguous_usually() {
        let content = "Usually format the output as JSON.";
        let results = find_ambiguous_instructions(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].term.to_lowercase(), "usually");
    }

    #[test]
    fn test_find_ambiguous_if_possible() {
        let content = "Include tests if possible.";
        let results = find_ambiguous_instructions(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].term.to_lowercase().contains("if possible"));
    }

    #[test]
    fn test_find_multiple_ambiguous() {
        let content = r#"Usually do X.
Sometimes do Y.
When appropriate, do Z.
"#;
        let results = find_ambiguous_instructions(content);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_skip_code_blocks() {
        let content = r#"```rust
// Usually this is fine in comments
fn usually_called() {}
```"#;
        let results = find_ambiguous_instructions(content);
        // Should skip entire fenced code block contents
        assert!(results.is_empty());
    }

    #[test]
    fn test_skip_multiline_code_blocks() {
        let content = r#"Some text here.

```
function usually_runs() {
  // usually in code
}
```

More text after."#;
        let results = find_ambiguous_instructions(content);
        // Should skip all lines inside the fenced code block
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_ambiguous_in_clear_instructions() {
        let content = r#"# Rules

Always format output as JSON.
Never include sensitive data.
"#;
        let results = find_ambiguous_instructions(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_ambiguous_context_captured() {
        let content = "This rule is generally applicable to all files.";
        let results = find_ambiguous_instructions(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].context.contains("generally"));
    }

    // ===== Boundary Condition Tests =====

    #[test]
    fn test_pe_001_exactly_ten_lines_boundary() {
        let lines: Vec<String> = (0..10).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        // No critical keyword in this content, so should be empty
        assert!(results.is_empty());
    }

    #[test]
    fn test_pe_001_nine_lines_under_minimum() {
        let lines: Vec<String> = (0..9).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        // Should be empty because content is shorter than 10 lines
        assert!(results.is_empty());
    }

    #[test]
    fn test_pe_001_eleven_lines_just_above_minimum() {
        let mut lines: Vec<String> = (0..11).map(|i| format!("Line {}", i)).collect();
        lines[5] = "This is critical information at 45%.".to_string();
        let content = lines.join("\n");

        let results = find_critical_in_middle_pe(&content);
        // Line 5 out of 11 = 45%, which is in the 40-60% zone
        assert_eq!(results.len(), 1);
        assert!(results[0].position_percent >= 40.0 && results[0].position_percent <= 60.0);
    }

    #[test]
    fn test_pe_003_word_boundary_hypercritical() {
        let content = r#"# Hypercritical Information

You should do X.
"#;
        let results = find_weak_imperative_language(content);
        // With word boundaries, "Hypercritical" should NOT match "critical"
        // so this should not be detected as a critical section
        assert!(
            results.is_empty(),
            "Hypercritical should not match critical with word boundaries"
        );
    }

    #[test]
    fn test_pe_003_critical_case_insensitive() {
        let content = r#"# CRITICAL INFORMATION

You should do X.
"#;
        let results = find_weak_imperative_language(content);
        // Should match despite case difference
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].weak_term.to_lowercase(), "should");
    }

    #[test]
    fn test_pe_003_important_header_detected() {
        let content = r#"# Important Configuration

You should enable this.
"#;
        let results = find_weak_imperative_language(content);
        // "Important" should trigger critical section recognition
        assert_eq!(results.len(), 1);
        assert!(results[0].section_name.to_lowercase().contains("important"));
    }

    #[test]
    fn test_pe_003_required_header_detected() {
        let content = r#"# Required Fields

Code could be cleaner.
"#;
        let results = find_weak_imperative_language(content);
        // "Required" should trigger critical section
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].weak_term.to_lowercase(), "could");
    }

    #[test]
    fn test_pe_004_inline_code_backticks_still_flagged() {
        let content = "Format with `usually` for clarity.";
        let results = find_ambiguous_instructions(content);
        // Current behavior: inline code is still flagged
        // This documents the behavior; could be improved in future
        assert!(!results.is_empty());
    }

    #[test]
    fn test_pe_004_comment_line_skipped() {
        let content = "// Usually this is in a comment";
        let results = find_ambiguous_instructions(content);
        // Comment lines should be skipped
        assert!(results.is_empty());
    }

    #[test]
    fn test_pe_004_shebang_skipped() {
        let content = "#!/usr/bin/env usually";
        let results = find_ambiguous_instructions(content);
        // Shebang lines should be skipped
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_string_all_validators() {
        let empty = "";

        let critical = find_critical_in_middle_pe(empty);
        let cot = find_cot_on_simple_tasks(empty);
        let weak = find_weak_imperative_language(empty);
        let ambiguous = find_ambiguous_instructions(empty);

        assert!(
            critical.is_empty(),
            "Empty content should have no critical in middle"
        );
        assert!(cot.is_empty(), "Empty content should have no CoT issues");
        assert!(
            weak.is_empty(),
            "Empty content should have no weak language"
        );
        assert!(
            ambiguous.is_empty(),
            "Empty content should have no ambiguous terms"
        );
    }

    #[test]
    fn test_single_line_all_validators() {
        let single = "This is critical.";

        let critical = find_critical_in_middle_pe(single);
        let cot = find_cot_on_simple_tasks(single);
        let weak = find_weak_imperative_language(single);
        let ambiguous = find_ambiguous_instructions(single);

        // Single line is too short for PE-001 (< 10 lines)
        assert!(critical.is_empty());
        // No simple task or CoT phrase
        assert!(cot.is_empty());
        // No critical section header
        assert!(weak.is_empty());
        // No ambiguous terms in this specific line
        assert!(ambiguous.is_empty());
    }

    // ===== CRLF byte offset tests =====

    #[test]
    fn test_pe_003_crlf_byte_offsets() {
        // Use CRLF line endings to verify byte offsets account for \r\n
        let content = "# Critical Rules\r\n\r\nYou should follow the style.\r\nAnother line.\r\n";
        let results = find_weak_imperative_language(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].weak_term.to_lowercase(), "should");
        // Verify the byte offset points to the actual position of "should"
        let found =
            &content[results[0].byte_offset..results[0].byte_offset + results[0].weak_term.len()];
        assert_eq!(
            found.to_lowercase(),
            results[0].weak_term.to_lowercase(),
            "Byte offset should correctly point to the weak term in CRLF content"
        );
    }

    #[test]
    fn test_pe_005_crlf_byte_offsets() {
        let content = "# Rules\r\n\r\nBe helpful in all interactions.\r\nAnother line.\r\n";
        let results = find_redundant_instructions(content);
        assert_eq!(results.len(), 1);
        // Verify the byte offset points to the actual position of "Be helpful"
        let found = &content[results[0].byte_offset..results[0].byte_offset + results[0].byte_len];
        assert_eq!(
            found.to_lowercase(),
            results[0].phrase.to_lowercase(),
            "Byte offset should correctly point to the redundant phrase in CRLF content"
        );
    }

    // ===== ReDoS Protection Tests =====

    #[test]
    fn test_find_critical_in_middle_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE (65536 bytes)
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_critical_in_middle_pe(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_find_cot_on_simple_tasks_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_cot_on_simple_tasks(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_find_weak_imperative_language_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_weak_imperative_language(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_find_ambiguous_instructions_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_ambiguous_instructions(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    // ===== PE-005: Redundant Generic Instructions =====

    #[test]
    fn test_find_redundant_be_helpful() {
        let content = "Be helpful and accurate when responding.";
        let results = find_redundant_instructions(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].phrase.to_lowercase().contains("be helpful"));
    }

    #[test]
    fn test_find_redundant_be_concise() {
        let content = "# Rules\n\nBe concise in your responses.";
        let results = find_redundant_instructions(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].phrase.to_lowercase().contains("be concise"));
    }

    #[test]
    fn test_find_multiple_redundant() {
        let content = "Be helpful.\nBe accurate.\nBe concise.";
        let results = find_redundant_instructions(content);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_no_redundant_in_specific_instructions() {
        let content =
            "Format all output as JSON with 2-space indentation.\nAlways include error codes.";
        let results = find_redundant_instructions(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_redundant_skips_code_blocks() {
        let content = "```\nBe helpful and accurate.\n```";
        let results = find_redundant_instructions(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_redundant_case_insensitive() {
        let content = "BE HELPFUL in all interactions.";
        let results = find_redundant_instructions(content);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_redundant_instructions_oversized_input() {
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_redundant_instructions(&large_content);
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    // ===== PE-006: Negative-Only Instructions =====

    #[test]
    fn test_find_negative_only_dont() {
        let content = "Don't use global variables.";
        let results = find_negative_only_instructions(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Don't"));
    }

    #[test]
    fn test_find_negative_only_never() {
        let content = "Never use console.log in production.";
        let results = find_negative_only_instructions(content);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_negative_only_avoid() {
        let content = "Avoid inline styles.";
        let results = find_negative_only_instructions(content);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_negative_with_positive_alternative() {
        let content = "Don't use global variables. Instead, pass values as function parameters.";
        let results = find_negative_only_instructions(content);
        assert!(
            results.is_empty(),
            "Should not flag when positive alternative is on same line"
        );
    }

    #[test]
    fn test_negative_with_positive_on_next_line() {
        let content = "Don't use global variables.\nInstead, pass values as function parameters.";
        let results = find_negative_only_instructions(content);
        assert!(
            results.is_empty(),
            "Should not flag when positive alternative is within 2 lines"
        );
    }

    #[test]
    fn test_negative_with_positive_two_lines_away() {
        let content =
            "Don't use global variables.\nSome explanation.\nUse function parameters instead.";
        let results = find_negative_only_instructions(content);
        assert!(
            results.is_empty(),
            "Should not flag when positive alternative is within window"
        );
    }

    #[test]
    fn test_negative_without_positive_too_far() {
        let content =
            "Don't use global variables.\nLine 2.\nLine 3.\nLine 4.\nUse functions instead.";
        let results = find_negative_only_instructions(content);
        assert_eq!(
            results.len(),
            1,
            "Should flag when positive alternative is beyond 2-line window"
        );
    }

    #[test]
    fn test_negative_skips_code_blocks() {
        let content = "```\nDon't use global variables.\n```";
        let results = find_negative_only_instructions(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_negative_only() {
        let content = "Don't use globals.\nNever use eval.\nAvoid inline styles.";
        let results = find_negative_only_instructions(content);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_negative_with_list_marker() {
        let content = "- Don't use global variables.";
        let results = find_negative_only_instructions(content);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_negative_only_instructions_oversized_input() {
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_negative_only_instructions(&large_content);
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_empty_content_pe_005_pe_006() {
        let empty = "";
        let redundant = find_redundant_instructions(empty);
        let negative = find_negative_only_instructions(empty);
        assert!(redundant.is_empty());
        assert!(negative.is_empty());
    }
}
