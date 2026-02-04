# Evaluation Harness

The evaluation harness measures rule efficacy by comparing expected vs actual diagnostics against labeled test cases.

## Quick Start

```bash
# Run evaluation with default markdown output
agnix eval tests/eval.yaml

# Output as JSON
agnix eval tests/eval.yaml --format json

# Output as CSV
agnix eval tests/eval.yaml --format csv

# Filter to specific rule family
agnix eval tests/eval.yaml --filter "AS-"

# Show per-case details
agnix eval tests/eval.yaml --verbose
```

## Manifest Format

Evaluation manifests are YAML files with the following structure:

```yaml
cases:
  - file: path/to/test/file.md
    expected: [AS-001, AS-002]
    description: "Optional description of what this tests"

  - file: path/to/valid/file.md
    expected: []
    description: "Valid file should trigger no rules"
```

### Fields

- `file`: Path to the file to validate (relative to manifest location)
- `expected`: List of rule IDs that should fire for this file
- `description`: Optional human-readable description

## Metrics

The harness calculates standard classification metrics:

### Per-Rule Metrics

- **TP (True Positives)**: Rule fired when expected
- **FP (False Positives)**: Rule fired when not expected
- **FN (False Negatives)**: Rule did not fire when expected

### Calculated Metrics

- **Precision**: TP / (TP + FP) - How often the rule is correct when it fires
- **Recall**: TP / (TP + FN) - How often the rule catches expected issues
- **F1 Score**: 2 * precision * recall / (precision + recall) - Harmonic mean

## Output Formats

### Markdown (default)

```markdown
## Evaluation Summary

**Cases**: 10 run, 8 passed, 2 failed
**Overall**: precision=95.00%, recall=90.00%, F1=92.43%

### Per-Rule Metrics

| Rule | TP | FP | FN | Precision | Recall | F1 |
|------|----|----|----|-----------:|-------:|----:|
| AS-001 | 5 | 1 | 0 | 83.33% | 100.00% | 90.91% |
```

### JSON

```json
{
  "cases_run": 10,
  "cases_passed": 8,
  "cases_failed": 2,
  "overall_precision": 0.95,
  "overall_recall": 0.90,
  "overall_f1": 0.9243,
  "rules": {
    "AS-001": {"tp": 5, "fp": 1, "fn_count": 0}
  }
}
```

### CSV

```csv
rule_id,tp,fp,fn,precision,recall,f1
AS-001,5,1,0,0.8333,1.0000,0.9091
OVERALL,10,2,1,0.9500,0.9000,0.9243
```

## Exit Codes

- `0`: All cases passed
- `1`: One or more cases failed

## Use Cases

### Regression Testing

Add evaluation to CI to catch rule regressions:

```yaml
- name: Evaluate rule efficacy
  run: agnix eval tests/eval.yaml
```

### Rule Development

When adding a new rule:

1. Create test fixtures that should trigger the rule
2. Create test fixtures that should NOT trigger the rule
3. Add cases to the evaluation manifest
4. Run `agnix eval` to verify behavior

### Metrics Tracking

Export metrics to track rule quality over time:

```bash
agnix eval tests/eval.yaml --format csv >> metrics-history.csv
```

## Creating Test Cases

### Positive Cases (Rule Should Fire)

```yaml
- file: fixtures/invalid/missing-field.md
  expected: [AS-002]
  description: "Missing required field triggers AS-002"
```

### Negative Cases (Rule Should NOT Fire)

```yaml
- file: fixtures/valid/complete.md
  expected: []
  description: "Valid file has no errors"
```

### Multiple Rules

```yaml
- file: fixtures/invalid/multiple-issues.md
  expected: [AS-002, AS-004, XML-001]
  description: "File with multiple issues"
```

## Interpreting Results

### High Precision, Low Recall

The rule is reliable but misses some cases. Consider:
- Adding more detection patterns
- Loosening match criteria

### Low Precision, High Recall

The rule catches issues but has false positives. Consider:
- Adding more specific patterns
- Adding exclusion rules

### Low F1 Score

The rule needs improvement in both areas. Consider:
- Reviewing the rule logic
- Adding more test cases to understand edge cases
