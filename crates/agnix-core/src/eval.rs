//! Evaluation harness for measuring rule efficacy
//!
//! This module provides types and functions to evaluate the effectiveness of
//! validation rules by comparing expected vs actual diagnostics against labeled
//! test cases.

use crate::{Diagnostic, LintConfig, LintError, file_utils::safe_read_file, validate_file};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A single evaluation case with expected rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// Path to the file to validate (relative to manifest directory)
    pub file: PathBuf,
    /// Expected rule IDs that should fire (e.g., ["AS-004", "CC-SK-006"])
    pub expected: Vec<String>,
    /// Optional description of what this case tests
    #[serde(default)]
    pub description: Option<String>,
}

/// Result of evaluating a single case
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    /// The original case
    pub case: EvalCase,
    /// Actual rule IDs that fired
    pub actual: Vec<String>,
    /// True positives: rules that were expected and did fire
    pub true_positives: Vec<String>,
    /// False positives: rules that fired but were not expected
    pub false_positives: Vec<String>,
    /// False negatives: rules that were expected but did not fire
    pub false_negatives: Vec<String>,
}

impl EvalResult {
    /// Check if this case passed (no false positives or false negatives)
    pub fn passed(&self) -> bool {
        self.false_positives.is_empty() && self.false_negatives.is_empty()
    }
}

/// Metrics for a single rule across all cases
#[derive(Debug, Clone, Default, Serialize)]
pub struct RuleMetrics {
    /// Rule ID
    pub rule_id: String,
    /// True positives count
    pub tp: usize,
    /// False positives count
    pub fp: usize,
    /// False negatives count
    pub fn_count: usize,
}

impl RuleMetrics {
    /// Create new metrics for a rule
    pub fn new(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            tp: 0,
            fp: 0,
            fn_count: 0,
        }
    }

    /// Calculate precision: TP / (TP + FP)
    /// Returns 1.0 if denominator is 0 (no predictions made)
    pub fn precision(&self) -> f64 {
        let denom = self.tp + self.fp;
        if denom == 0 {
            1.0
        } else {
            self.tp as f64 / denom as f64
        }
    }

    /// Calculate recall: TP / (TP + FN)
    /// Returns 1.0 if denominator is 0 (no actual positives)
    pub fn recall(&self) -> f64 {
        let denom = self.tp + self.fn_count;
        if denom == 0 {
            1.0
        } else {
            self.tp as f64 / denom as f64
        }
    }

    /// Calculate F1 score: 2 * precision * recall / (precision + recall)
    /// Returns 0.0 if both precision and recall are 0
    pub fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        let denom = p + r;
        if denom == 0.0 {
            0.0
        } else {
            2.0 * p * r / denom
        }
    }
}

/// Summary of evaluation across all cases
#[derive(Debug, Clone, Serialize)]
pub struct EvalSummary {
    /// Total number of cases evaluated
    pub cases_run: usize,
    /// Number of cases that passed
    pub cases_passed: usize,
    /// Number of cases that failed
    pub cases_failed: usize,
    /// Per-rule metrics
    pub rules: HashMap<String, RuleMetrics>,
    /// Overall precision across all rules
    pub overall_precision: f64,
    /// Overall recall across all rules
    pub overall_recall: f64,
    /// Overall F1 score
    pub overall_f1: f64,
}

impl EvalSummary {
    /// Create a new summary from evaluation results
    pub fn from_results(results: &[EvalResult]) -> Self {
        let mut rules: HashMap<String, RuleMetrics> = HashMap::new();

        // Aggregate metrics for each rule
        for result in results {
            // True positives
            for rule_id in &result.true_positives {
                rules
                    .entry(rule_id.clone())
                    .or_insert_with(|| RuleMetrics::new(rule_id))
                    .tp += 1;
            }

            // False positives
            for rule_id in &result.false_positives {
                rules
                    .entry(rule_id.clone())
                    .or_insert_with(|| RuleMetrics::new(rule_id))
                    .fp += 1;
            }

            // False negatives
            for rule_id in &result.false_negatives {
                rules
                    .entry(rule_id.clone())
                    .or_insert_with(|| RuleMetrics::new(rule_id))
                    .fn_count += 1;
            }
        }

        // Calculate overall metrics
        let total_tp: usize = rules.values().map(|m| m.tp).sum();
        let total_fp: usize = rules.values().map(|m| m.fp).sum();
        let total_fn: usize = rules.values().map(|m| m.fn_count).sum();

        let overall_precision = if total_tp + total_fp == 0 {
            1.0
        } else {
            total_tp as f64 / (total_tp + total_fp) as f64
        };

        let overall_recall = if total_tp + total_fn == 0 {
            1.0
        } else {
            total_tp as f64 / (total_tp + total_fn) as f64
        };

        let overall_f1 = if overall_precision + overall_recall == 0.0 {
            0.0
        } else {
            2.0 * overall_precision * overall_recall / (overall_precision + overall_recall)
        };

        let cases_passed = results.iter().filter(|r| r.passed()).count();

        Self {
            cases_run: results.len(),
            cases_passed,
            cases_failed: results.len() - cases_passed,
            rules,
            overall_precision,
            overall_recall,
            overall_f1,
        }
    }

    /// Format summary as JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Format summary as CSV
    pub fn to_csv(&self) -> String {
        let mut lines = vec!["rule_id,tp,fp,fn,precision,recall,f1".to_string()];

        let mut sorted_rules: Vec<_> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|(k, _)| *k);

        for (rule_id, metrics) in sorted_rules {
            lines.push(format!(
                "{},{},{},{},{:.4},{:.4},{:.4}",
                rule_id,
                metrics.tp,
                metrics.fp,
                metrics.fn_count,
                metrics.precision(),
                metrics.recall(),
                metrics.f1()
            ));
        }

        // Add overall row
        let total_tp: usize = self.rules.values().map(|m| m.tp).sum();
        let total_fp: usize = self.rules.values().map(|m| m.fp).sum();
        let total_fn: usize = self.rules.values().map(|m| m.fn_count).sum();
        lines.push(format!(
            "OVERALL,{},{},{},{:.4},{:.4},{:.4}",
            total_tp,
            total_fp,
            total_fn,
            self.overall_precision,
            self.overall_recall,
            self.overall_f1
        ));

        lines.join("\n")
    }

    /// Format summary as Markdown table
    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            format!("## Evaluation Summary"),
            String::new(),
            format!(
                "**Cases**: {} run, {} passed, {} failed",
                self.cases_run, self.cases_passed, self.cases_failed
            ),
            format!(
                "**Overall**: precision={:.2}%, recall={:.2}%, F1={:.2}%",
                self.overall_precision * 100.0,
                self.overall_recall * 100.0,
                self.overall_f1 * 100.0
            ),
            String::new(),
            "### Per-Rule Metrics".to_string(),
            String::new(),
            "| Rule | TP | FP | FN | Precision | Recall | F1 |".to_string(),
            "|------|----|----|----|-----------:|-------:|----:|".to_string(),
        ];

        let mut sorted_rules: Vec<_> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|(k, _)| *k);

        for (rule_id, metrics) in sorted_rules {
            lines.push(format!(
                "| {} | {} | {} | {} | {:.2}% | {:.2}% | {:.2}% |",
                rule_id,
                metrics.tp,
                metrics.fp,
                metrics.fn_count,
                metrics.precision() * 100.0,
                metrics.recall() * 100.0,
                metrics.f1() * 100.0
            ));
        }

        lines.join("\n")
    }
}

/// Evaluation manifest containing multiple test cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalManifest {
    /// List of evaluation cases
    pub cases: Vec<EvalCase>,
}

impl EvalManifest {
    /// Load a manifest from a YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, EvalError> {
        let content = safe_read_file(path.as_ref()).map_err(|e| EvalError::Read {
            path: path.as_ref().to_path_buf(),
            source: e,
        })?;

        serde_yaml::from_str(&content).map_err(|e| EvalError::Parse {
            path: path.as_ref().to_path_buf(),
            message: e.to_string(),
        })
    }

    /// Get the base directory for resolving relative file paths
    fn base_dir<P: AsRef<Path>>(manifest_path: P) -> PathBuf {
        manifest_path
            .as_ref()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Errors that can occur during evaluation
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("Failed to read manifest: {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: LintError,
    },

    #[error("Failed to read file: {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse manifest {path}: {message}")]
    Parse { path: PathBuf, message: String },

    #[error("Validation error for {path}: {message}")]
    Validation { path: PathBuf, message: String },

    #[error("Path traversal attempt detected: {path} escapes base directory {base_dir}")]
    PathTraversal { path: PathBuf, base_dir: PathBuf },
}

/// Validate that a file path is safe to access
///
/// Security model:
/// - Reject absolute paths (only relative paths allowed in manifests)
/// - Canonicalize both base directory and file path
/// - Verify canonical file path starts with canonical base path
/// - Return canonical path to prevent TOCTOU vulnerabilities
fn validate_path_within_base(file: &Path, base_dir: &Path) -> Result<PathBuf, EvalError> {
    // Reject absolute paths - only relative paths allowed in manifests
    if file.is_absolute() {
        return Err(EvalError::PathTraversal {
            path: file.to_path_buf(),
            base_dir: base_dir.to_path_buf(),
        });
    }

    let joined = base_dir.join(file);

    // Canonicalize to resolve ".." and symlinks
    let canonical_file = joined.canonicalize().map_err(|e| EvalError::Io {
        path: joined.clone(),
        source: e,
    })?;

    // Canonicalize base directory
    let canonical_base = base_dir.canonicalize().map_err(|e| EvalError::Io {
        path: base_dir.to_path_buf(),
        source: e,
    })?;

    // Verify file path is within base directory (prevents path traversal)
    if !canonical_file.starts_with(&canonical_base) {
        return Err(EvalError::PathTraversal {
            path: file.to_path_buf(),
            base_dir: base_dir.to_path_buf(),
        });
    }

    // Return canonical path to prevent TOCTOU vulnerabilities
    Ok(canonical_file)
}

/// Evaluate a single case against the validator
pub fn evaluate_case(case: &EvalCase, base_dir: &Path, config: &LintConfig) -> EvalResult {
    // Validate path doesn't escape base directory
    let file_path = match validate_path_within_base(&case.file, base_dir) {
        Ok(path) => path,
        Err(_) => {
            // Return error result for path traversal attempts
            return EvalResult {
                case: case.clone(),
                actual: vec!["eval::path-traversal".to_string()],
                true_positives: vec![],
                false_positives: vec!["eval::path-traversal".to_string()],
                false_negatives: case.expected.clone(),
            };
        }
    };

    // Run validation
    let diagnostics = match validate_file(&file_path, config) {
        Ok(diags) => diags,
        Err(e) => {
            // If validation fails, treat it as if no rules fired
            // but include the error as a special diagnostic
            vec![Diagnostic::error(
                file_path.clone(),
                0,
                0,
                "eval::error",
                format!("Validation failed: {}", e),
            )]
        }
    };

    // Extract actual rule IDs (deduplicated)
    let actual: Vec<String> = diagnostics
        .iter()
        .map(|d| d.rule.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Calculate TP, FP, FN using set operations
    let expected_set: HashSet<&str> = case.expected.iter().map(|s| s.as_str()).collect();
    let actual_set: HashSet<&str> = actual.iter().map(|s| s.as_str()).collect();

    let true_positives: Vec<String> = expected_set
        .intersection(&actual_set)
        .map(|s| s.to_string())
        .collect();

    let false_positives: Vec<String> = actual_set
        .difference(&expected_set)
        .map(|s| s.to_string())
        .collect();

    let false_negatives: Vec<String> = expected_set
        .difference(&actual_set)
        .map(|s| s.to_string())
        .collect();

    EvalResult {
        case: case.clone(),
        actual,
        true_positives,
        false_positives,
        false_negatives,
    }
}

/// Evaluate all cases in a manifest
pub fn evaluate_manifest(
    manifest: &EvalManifest,
    base_dir: &Path,
    config: &LintConfig,
    filter: Option<&str>,
) -> Vec<EvalResult> {
    manifest
        .cases
        .iter()
        .filter(|case| {
            // Apply filter if provided
            match filter {
                Some(f) => {
                    // Include cases with empty expected (to detect false positives on clean files)
                    // or cases where any expected rule matches the filter prefix
                    case.expected.is_empty() || case.expected.iter().any(|rule| rule.starts_with(f))
                }
                None => true,
            }
        })
        .map(|case| evaluate_case(case, base_dir, config))
        .collect()
}

/// Main entry point: load manifest and evaluate
pub fn evaluate_manifest_file<P: AsRef<Path>>(
    manifest_path: P,
    config: &LintConfig,
    filter: Option<&str>,
) -> Result<(Vec<EvalResult>, EvalSummary), EvalError> {
    let manifest = EvalManifest::load(&manifest_path)?;
    let base_dir = EvalManifest::base_dir(&manifest_path);

    let results = evaluate_manifest(&manifest, &base_dir, config, filter);
    let summary = EvalSummary::from_results(&results);

    Ok((results, summary))
}

/// Output format for evaluation results
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EvalFormat {
    #[default]
    Markdown,
    Json,
    Csv,
}

impl std::str::FromStr for EvalFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(EvalFormat::Markdown),
            "json" => Ok(EvalFormat::Json),
            "csv" => Ok(EvalFormat::Csv),
            _ => Err(format!(
                "Unknown format: {}. Use markdown, json, or csv.",
                s
            )),
        }
    }
}

impl std::fmt::Display for EvalFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalFormat::Markdown => write!(f, "markdown"),
            EvalFormat::Json => write!(f, "json"),
            EvalFormat::Csv => write!(f, "csv"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_metrics_precision() {
        let mut m = RuleMetrics::new("TEST-001");
        m.tp = 8;
        m.fp = 2;
        m.fn_count = 0;

        // precision = 8 / (8 + 2) = 0.8
        assert!((m.precision() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_rule_metrics_recall() {
        let mut m = RuleMetrics::new("TEST-001");
        m.tp = 8;
        m.fp = 0;
        m.fn_count = 2;

        // recall = 8 / (8 + 2) = 0.8
        assert!((m.recall() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_rule_metrics_f1() {
        let mut m = RuleMetrics::new("TEST-001");
        m.tp = 8;
        m.fp = 2;
        m.fn_count = 2;

        // precision = 8/10 = 0.8, recall = 8/10 = 0.8
        // f1 = 2 * 0.8 * 0.8 / (0.8 + 0.8) = 1.28 / 1.6 = 0.8
        assert!((m.f1() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_rule_metrics_zero_division() {
        let m = RuleMetrics::new("TEST-001");

        // No predictions, no actual positives - should return 1.0
        assert!((m.precision() - 1.0).abs() < 0.001);
        assert!((m.recall() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rule_metrics_f1_zero() {
        let mut m = RuleMetrics::new("TEST-001");
        m.tp = 0;
        m.fp = 5;
        m.fn_count = 5;

        // precision = 0/5 = 0, recall = 0/5 = 0
        // f1 = 0
        assert!((m.f1() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_eval_result_passed() {
        let result = EvalResult {
            case: EvalCase {
                file: PathBuf::from("test.md"),
                expected: vec!["AS-001".to_string()],
                description: None,
            },
            actual: vec!["AS-001".to_string()],
            true_positives: vec!["AS-001".to_string()],
            false_positives: vec![],
            false_negatives: vec![],
        };

        assert!(result.passed());
    }

    #[test]
    fn test_eval_result_failed_fp() {
        let result = EvalResult {
            case: EvalCase {
                file: PathBuf::from("test.md"),
                expected: vec!["AS-001".to_string()],
                description: None,
            },
            actual: vec!["AS-001".to_string(), "AS-002".to_string()],
            true_positives: vec!["AS-001".to_string()],
            false_positives: vec!["AS-002".to_string()],
            false_negatives: vec![],
        };

        assert!(!result.passed());
    }

    #[test]
    fn test_eval_result_failed_fn() {
        let result = EvalResult {
            case: EvalCase {
                file: PathBuf::from("test.md"),
                expected: vec!["AS-001".to_string(), "AS-002".to_string()],
                description: None,
            },
            actual: vec!["AS-001".to_string()],
            true_positives: vec!["AS-001".to_string()],
            false_positives: vec![],
            false_negatives: vec!["AS-002".to_string()],
        };

        assert!(!result.passed());
    }

    #[test]
    fn test_eval_summary_from_results() {
        let results = vec![
            EvalResult {
                case: EvalCase {
                    file: PathBuf::from("test1.md"),
                    expected: vec!["AS-001".to_string()],
                    description: None,
                },
                actual: vec!["AS-001".to_string()],
                true_positives: vec!["AS-001".to_string()],
                false_positives: vec![],
                false_negatives: vec![],
            },
            EvalResult {
                case: EvalCase {
                    file: PathBuf::from("test2.md"),
                    expected: vec!["AS-001".to_string()],
                    description: None,
                },
                actual: vec!["AS-001".to_string(), "AS-002".to_string()],
                true_positives: vec!["AS-001".to_string()],
                false_positives: vec!["AS-002".to_string()],
                false_negatives: vec![],
            },
        ];

        let summary = EvalSummary::from_results(&results);

        assert_eq!(summary.cases_run, 2);
        assert_eq!(summary.cases_passed, 1);
        assert_eq!(summary.cases_failed, 1);

        let as_001 = summary.rules.get("AS-001").unwrap();
        assert_eq!(as_001.tp, 2);
        assert_eq!(as_001.fp, 0);
        assert_eq!(as_001.fn_count, 0);

        let as_002 = summary.rules.get("AS-002").unwrap();
        assert_eq!(as_002.tp, 0);
        assert_eq!(as_002.fp, 1);
        assert_eq!(as_002.fn_count, 0);
    }

    #[test]
    fn test_eval_manifest_parse() {
        let yaml = r#"
cases:
  - file: test1.md
    expected: [AS-001]
    description: "Test case 1"
  - file: test2.md
    expected: [AS-002, AS-003]
"#;

        let manifest: EvalManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.cases.len(), 2);
        assert_eq!(manifest.cases[0].expected, vec!["AS-001"]);
        assert_eq!(manifest.cases[1].expected, vec!["AS-002", "AS-003"]);
    }

    #[test]
    fn test_eval_summary_to_csv() {
        let results = vec![EvalResult {
            case: EvalCase {
                file: PathBuf::from("test.md"),
                expected: vec!["AS-001".to_string()],
                description: None,
            },
            actual: vec!["AS-001".to_string()],
            true_positives: vec!["AS-001".to_string()],
            false_positives: vec![],
            false_negatives: vec![],
        }];

        let summary = EvalSummary::from_results(&results);
        let csv = summary.to_csv();

        assert!(csv.contains("rule_id,tp,fp,fn,precision,recall,f1"));
        assert!(csv.contains("AS-001,1,0,0"));
        assert!(csv.contains("OVERALL"));
    }

    #[test]
    fn test_eval_summary_to_markdown() {
        let results = vec![EvalResult {
            case: EvalCase {
                file: PathBuf::from("test.md"),
                expected: vec!["AS-001".to_string()],
                description: None,
            },
            actual: vec!["AS-001".to_string()],
            true_positives: vec!["AS-001".to_string()],
            false_positives: vec![],
            false_negatives: vec![],
        }];

        let summary = EvalSummary::from_results(&results);
        let md = summary.to_markdown();

        assert!(md.contains("## Evaluation Summary"));
        assert!(md.contains("| Rule | TP | FP | FN |"));
        assert!(md.contains("| AS-001 |"));
    }

    #[test]
    fn test_eval_format_from_str() {
        assert_eq!(
            "markdown".parse::<EvalFormat>().unwrap(),
            EvalFormat::Markdown
        );
        assert_eq!("md".parse::<EvalFormat>().unwrap(), EvalFormat::Markdown);
        assert_eq!("json".parse::<EvalFormat>().unwrap(), EvalFormat::Json);
        assert_eq!("csv".parse::<EvalFormat>().unwrap(), EvalFormat::Csv);
        assert!("invalid".parse::<EvalFormat>().is_err());
    }

    #[test]
    fn test_evaluate_case_with_fixture() {
        // Use an actual fixture to test evaluation
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: deploy-prod\ndescription: Deploys to production\n---\nBody",
        )
        .unwrap();

        let case = EvalCase {
            file: PathBuf::from("SKILL.md"),
            expected: vec!["CC-SK-006".to_string()],
            description: Some("Dangerous skill name".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, temp.path(), &config);

        // CC-SK-006 should fire for dangerous deploy-prod name
        assert!(
            result.true_positives.contains(&"CC-SK-006".to_string()),
            "Expected CC-SK-006 in true_positives, got: {:?}",
            result
        );
    }

    #[test]
    fn test_evaluate_case_empty_expected() {
        // Test a valid file with no expected rules
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
        )
        .unwrap();

        let case = EvalCase {
            file: PathBuf::from("SKILL.md"),
            expected: vec![],
            description: Some("Valid skill, no rules expected".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, temp.path(), &config);

        // No errors expected - this is a valid skill
        assert!(
            result.false_negatives.is_empty(),
            "Should have no false negatives"
        );
        // true_positives should be empty since expected is empty
        assert!(result.true_positives.is_empty());
    }

    #[test]
    fn test_eval_manifest_load_file_not_found() {
        let result = EvalManifest::load("nonexistent-manifest-file.yaml");
        assert!(result.is_err());
        match result {
            Err(EvalError::Read { path, .. }) => {
                assert!(path.to_string_lossy().contains("nonexistent"));
            }
            _ => panic!("Expected EvalError::Read"),
        }
    }

    #[test]
    fn test_eval_manifest_load_large_file_rejected() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest_path = temp.path().join("large.yaml");
        let content = vec![b'x'; (crate::file_utils::DEFAULT_MAX_FILE_SIZE + 1) as usize];
        std::fs::write(&manifest_path, &content).unwrap();

        let result = EvalManifest::load(&manifest_path);
        assert!(result.is_err());
        match result {
            Err(EvalError::Read { source, .. }) => {
                assert!(matches!(source, LintError::FileTooBig { .. }));
            }
            _ => panic!("Expected EvalError::Read"),
        }
    }

    #[test]
    fn test_eval_manifest_load_invalid_yaml() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest_path = temp.path().join("invalid.yaml");
        std::fs::write(&manifest_path, "invalid: yaml: : syntax").unwrap();

        let result = EvalManifest::load(&manifest_path);
        assert!(result.is_err());
        match result {
            Err(EvalError::Parse { path, message }) => {
                assert_eq!(path, manifest_path);
                assert!(!message.is_empty());
            }
            _ => panic!("Expected EvalError::Parse"),
        }
    }

    #[test]
    fn test_evaluate_manifest_with_filter() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create two skill files
        let skill1_path = temp.path().join("skill1.md");
        std::fs::write(
            &skill1_path,
            "---\nname: deploy-prod\ndescription: Deploy\n---\nBody",
        )
        .unwrap();

        let skill2_path = temp.path().join("skill2.md");
        std::fs::write(
            &skill2_path,
            "---\nname: run-tests\ndescription: Tests\n---\nBody",
        )
        .unwrap();

        let manifest = EvalManifest {
            cases: vec![
                EvalCase {
                    file: PathBuf::from("skill1.md"),
                    expected: vec!["CC-SK-006".to_string()],
                    description: None,
                },
                EvalCase {
                    file: PathBuf::from("skill2.md"),
                    expected: vec!["AS-004".to_string()],
                    description: None,
                },
            ],
        };

        let config = LintConfig::default();

        // Filter for CC-SK rules only
        let results = evaluate_manifest(&manifest, temp.path(), &config, Some("CC-SK"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].case.file, PathBuf::from("skill1.md"));

        // Filter for AS rules only
        let results = evaluate_manifest(&manifest, temp.path(), &config, Some("AS-"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].case.file, PathBuf::from("skill2.md"));

        // No filter - all cases
        let results = evaluate_manifest(&manifest, temp.path(), &config, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_evaluate_manifest_filter_no_matches() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("skill.md");
        std::fs::write(&skill_path, "---\nname: test\ndescription: Test\n---\nBody").unwrap();

        let manifest = EvalManifest {
            cases: vec![EvalCase {
                file: PathBuf::from("skill.md"),
                expected: vec!["AS-001".to_string()],
                description: None,
            }],
        };

        let config = LintConfig::default();
        let results = evaluate_manifest(&manifest, temp.path(), &config, Some("NONEXISTENT"));
        assert!(results.is_empty());
    }

    #[test]
    fn test_evaluate_manifest_file_entry_point() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create skill file
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: deploy-prod\ndescription: Deploy\n---\nBody",
        )
        .unwrap();

        // Create manifest
        let manifest_path = temp.path().join("eval.yaml");
        std::fs::write(
            &manifest_path,
            r#"cases:
  - file: SKILL.md
    expected: [CC-SK-006]
    description: "Dangerous skill name"
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let result = evaluate_manifest_file(&manifest_path, &config, None);
        assert!(result.is_ok());

        let (results, summary) = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(summary.cases_run, 1);
    }

    #[test]
    fn test_path_traversal_absolute_path() {
        let temp = tempfile::TempDir::new().unwrap();

        // Test absolute path (not allowed in manifests)
        let case = EvalCase {
            file: PathBuf::from("/etc/passwd"),
            expected: vec!["SOME-RULE".to_string()],
            description: Some("Absolute path attempt".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, temp.path(), &config);

        // Should detect path traversal for absolute paths
        assert!(
            result.actual.contains(&"eval::path-traversal".to_string()),
            "Should reject absolute paths, got: {:?}",
            result.actual
        );
    }

    #[test]
    fn test_relative_path_escaping_base_dir_blocked() {
        // Relative paths that escape the base directory should be blocked
        // This test verifies that ../sibling paths are detected as path traversal
        let temp = tempfile::TempDir::new().unwrap();

        // Create a structure: temp/subdir as base_dir, temp/file.md as target
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file_path = temp.path().join("file.md");
        std::fs::write(&file_path, "---\nname: test\n---\nContent").unwrap();

        let case = EvalCase {
            file: PathBuf::from("../file.md"),
            expected: vec![],
            description: Some("Relative path going up one level".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, &subdir, &config);

        // Should be flagged as path traversal - escapes base_dir
        assert!(
            result.actual.contains(&"eval::path-traversal".to_string()),
            "Relative ../sibling paths that escape base_dir should be blocked, got: {:?}",
            result.actual
        );
    }

    #[test]
    fn test_relative_path_within_base_dir_allowed() {
        // Relative paths that stay within base_dir should be allowed
        let temp = tempfile::TempDir::new().unwrap();

        // Create file in base_dir
        let file_path = temp.path().join("SKILL.md");
        std::fs::write(
            &file_path,
            "---\nname: test-skill\ndescription: Test\n---\nBody",
        )
        .unwrap();

        let case = EvalCase {
            file: PathBuf::from("SKILL.md"),
            expected: vec![],
            description: Some("File in base directory".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, temp.path(), &config);

        // Should NOT be flagged as path traversal
        assert!(
            !result.actual.contains(&"eval::path-traversal".to_string()),
            "Files within base_dir should be allowed, got: {:?}",
            result.actual
        );
    }

    #[test]
    fn test_evaluate_case_validation_error() {
        // Test behavior when validate_file returns an error
        let temp = tempfile::TempDir::new().unwrap();

        // Create an invalid file that will cause validation errors
        let file_path = temp.path().join("invalid.md");
        std::fs::write(&file_path, "invalid content that won't parse").unwrap();

        let case = EvalCase {
            file: PathBuf::from("invalid.md"),
            expected: vec!["SOME-RULE".to_string()],
            description: Some("Invalid file content".to_string()),
        };

        let config = LintConfig::default();
        let result = evaluate_case(&case, temp.path(), &config);

        // Should handle the case gracefully - either no diagnostics or some actual rules
        // The expected rule won't fire, so it should be in false_negatives
        assert!(
            result.false_negatives.contains(&"SOME-RULE".to_string()),
            "Expected rule should be in false_negatives when it doesn't fire"
        );
    }

    #[test]
    fn test_eval_format_display() {
        assert_eq!(format!("{}", EvalFormat::Markdown), "markdown");
        assert_eq!(format!("{}", EvalFormat::Json), "json");
        assert_eq!(format!("{}", EvalFormat::Csv), "csv");
    }

    #[test]
    fn test_eval_summary_empty_results() {
        let results: Vec<EvalResult> = vec![];
        let summary = EvalSummary::from_results(&results);

        assert_eq!(summary.cases_run, 0);
        assert_eq!(summary.cases_passed, 0);
        assert_eq!(summary.cases_failed, 0);
        assert!(summary.rules.is_empty());
        assert!((summary.overall_precision - 1.0).abs() < 0.001);
        assert!((summary.overall_recall - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_eval_manifest_base_dir() {
        // Test base_dir extraction with nested path
        let base = EvalManifest::base_dir("some/path/to/manifest.yaml");
        assert_eq!(base, PathBuf::from("some/path/to"));

        // Test root file fallback - parent() returns "" on Windows for just filename
        let base = EvalManifest::base_dir("manifest.yaml");
        // Parent of "manifest.yaml" is either "" or "." depending on platform
        assert!(base == std::path::Path::new(".") || base == std::path::Path::new(""));
    }
}
