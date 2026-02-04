//! agnix CLI - The nginx of agent configs

mod json;
mod sarif;

use agnix_core::{
    apply_fixes,
    config::{LintConfig, TargetTool},
    diagnostics::DiagnosticLevel,
    eval::{evaluate_manifest_file, EvalFormat},
    validate_project, ValidationResult,
};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use similar::{ChangeTag, TextDiff};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Sarif,
}

/// CLI target argument enum with kebab-case names for command line ergonomics.
/// Separate from TargetTool (which uses PascalCase for config file serialization).
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TargetArg {
    #[default]
    Generic,
    #[value(name = "claude-code")]
    ClaudeCode,
    Cursor,
    Codex,
}

impl From<TargetArg> for TargetTool {
    fn from(arg: TargetArg) -> Self {
        match arg {
            TargetArg::Generic => TargetTool::Generic,
            TargetArg::ClaudeCode => TargetTool::ClaudeCode,
            TargetArg::Cursor => TargetTool::Cursor,
            TargetArg::Codex => TargetTool::Codex,
        }
    }
}

#[derive(Parser)]
#[command(name = "agnix")]
#[command(author, version, about, long_about = None)]
#[command(
    about = "The nginx of agent configs",
    long_about = "Validate agent specifications across Claude Code, Cursor, Codex, and beyond.\n\nValidates: Skills • MCP • Hooks • Memory • Plugins"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to validate (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Strict mode (treat warnings as errors)
    #[arg(short, long)]
    strict: bool,

    /// Target tool (generic, claude-code, cursor, codex)
    #[arg(short, long, value_enum, default_value_t = TargetArg::Generic)]
    target: TargetArg,

    /// Config file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Apply automatic fixes
    #[arg(long, group = "fix_mode")]
    fix: bool,

    /// Show what would be fixed without modifying files
    #[arg(long, group = "fix_mode")]
    dry_run: bool,

    /// Only apply safe (HIGH certainty) fixes (implies --fix)
    #[arg(long)]
    fix_safe: bool,

    /// Output format (text, json, or sarif)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

/// Output format for evaluation results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum EvalOutputFormat {
    #[default]
    Markdown,
    Json,
    Csv,
}

impl From<EvalOutputFormat> for EvalFormat {
    fn from(f: EvalOutputFormat) -> Self {
        match f {
            EvalOutputFormat::Markdown => EvalFormat::Markdown,
            EvalOutputFormat::Json => EvalFormat::Json,
            EvalOutputFormat::Csv => EvalFormat::Csv,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Validate agent configs
    Validate {
        /// Path to validate
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Initialize config file
    Init {
        /// Output path for config
        #[arg(default_value = ".agnix.toml")]
        output: PathBuf,
    },

    /// Evaluate rule efficacy against labeled test cases
    Eval {
        /// Path to evaluation manifest (YAML file)
        path: PathBuf,

        /// Output format (markdown, json, csv)
        #[arg(long, short, value_enum, default_value_t = EvalOutputFormat::Markdown)]
        format: EvalOutputFormat,

        /// Filter to specific rule prefix (e.g., "AS-", "MCP-")
        #[arg(long)]
        filter: Option<String>,

        /// Show detailed results for each case
        #[arg(long, short)]
        verbose: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::Validate { path }) => validate_command(path, &cli),
        Some(Commands::Init { output }) => init_command(output),
        Some(Commands::Eval {
            path,
            format,
            filter,
            verbose,
        }) => eval_command(path, *format, filter.as_deref(), *verbose),
        None => validate_command(&cli.path, &cli),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

fn validate_command(path: &Path, cli: &Cli) -> anyhow::Result<()> {
    let config_path = resolve_config_path(path, cli);
    let (mut config, config_warning) = LintConfig::load_or_default(config_path.as_ref());

    // Display config warning before validation output
    if let Some(warning) = config_warning {
        eprintln!("{} {}", "Warning:".yellow().bold(), warning);
        eprintln!();
    }
    config.target = cli.target.into();

    let should_fix = cli.fix || cli.fix_safe || cli.dry_run;
    if should_fix && !matches!(cli.format, OutputFormat::Text) {
        return Err(anyhow::anyhow!(
            "Fix flags are only supported with text output. Remove --format or use --format text."
        ));
    }

    // Resolve absolute path for consistent relative output (prefer repo root)
    let base_path = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));

    let ValidationResult {
        diagnostics,
        files_checked,
    } = validate_project(path, &config)?;

    // Handle JSON output format
    if matches!(cli.format, OutputFormat::Json) {
        let json_output = json::diagnostics_to_json(&diagnostics, &base_path, files_checked);
        let json_str = serde_json::to_string_pretty(&json_output)?;
        println!("{}", json_str);

        // Exit with error code if there are errors (use summary to avoid re-iterating)
        if json_output.summary.errors > 0 || (cli.strict && json_output.summary.warnings > 0) {
            process::exit(1);
        }
        return Ok(());
    }

    // Handle SARIF output format
    if matches!(cli.format, OutputFormat::Sarif) {
        let sarif = sarif::diagnostics_to_sarif(&diagnostics, &base_path);
        let json = serde_json::to_string_pretty(&sarif)?;
        println!("{}", json);

        // Exit with error code if there are errors
        let has_errors = diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error);
        let has_warnings = diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Warning);

        if has_errors || (cli.strict && has_warnings) {
            process::exit(1);
        }
        return Ok(());
    }

    // Text output format
    println!("{} {}", "Validating:".cyan().bold(), path.display());
    println!();

    if diagnostics.is_empty() {
        println!("{}", "No issues found".green().bold());
        return Ok(());
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .count();
    let infos = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Info)
        .count();
    let fixable = diagnostics.iter().filter(|d| d.has_fixes()).count();

    for diag in &diagnostics {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
        };

        let fixable_marker = if diag.has_fixes() {
            " [fixable]".green().to_string()
        } else {
            String::new()
        };

        println!(
            "{}:{}:{} {}: {}{}",
            diag.file.display().to_string().dimmed(),
            diag.line,
            diag.column,
            level_str,
            diag.message,
            fixable_marker
        );

        if cli.verbose {
            println!("  {} {}", "rule:".dimmed(), diag.rule.dimmed());
            if let Some(suggestion) = &diag.suggestion {
                println!("  {} {}", "help:".cyan(), suggestion);
            }
            for fix in &diag.fixes {
                let safety = if fix.safe { "safe" } else { "unsafe" };
                println!("  {} {} ({})", "fix:".green(), fix.description, safety);
            }
        }
        println!();
    }

    println!("{}", "-".repeat(60).dimmed());
    println!(
        "Found {} {}, {} {}",
        errors,
        if errors == 1 { "error" } else { "errors" },
        warnings,
        if warnings == 1 { "warning" } else { "warnings" }
    );

    if infos > 0 {
        println!("  {} info messages", infos);
    }

    if fixable > 0 {
        println!(
            "  {} {} automatically fixable",
            fixable,
            if fixable == 1 {
                "issue is"
            } else {
                "issues are"
            }
        );
    }

    // --fix-safe implies --fix
    if should_fix {
        println!();
        let mode = if cli.dry_run { "Preview" } else { "Applying" };
        let safe_mode = if cli.fix_safe { " (safe only)" } else { "" };
        println!("{} fixes{}...", mode.cyan().bold(), safe_mode);

        let results = apply_fixes(&diagnostics, cli.dry_run, cli.fix_safe)?;

        if results.is_empty() {
            println!("  No fixes to apply");
        } else {
            for result in &results {
                println!();
                println!(
                    "  {} {}",
                    if cli.dry_run { "Would fix:" } else { "Fixed:" }.green(),
                    result.path.display()
                );
                for desc in &result.applied {
                    println!("    - {}", desc);
                }

                if cli.dry_run && cli.verbose {
                    println!();
                    println!("  {}:", "Diff".yellow());
                    show_diff(&result.original, &result.fixed);
                }
            }

            println!();
            let action = if cli.dry_run { "Would fix" } else { "Fixed" };
            println!(
                "{} {} {}",
                action.green().bold(),
                results.len(),
                if results.len() == 1 { "file" } else { "files" }
            );
        }
    } else if fixable > 0 {
        println!();
        println!(
            "{} Run with {} to apply fixes",
            "hint:".cyan(),
            "--fix".bold()
        );
    }

    // Exit with error if errors remain (even after fixing) or strict mode with warnings
    if errors > 0 || (cli.strict && warnings > 0) {
        process::exit(1);
    }

    Ok(())
}

fn resolve_config_path(path: &Path, cli: &Cli) -> Option<PathBuf> {
    if let Some(config) = &cli.config {
        return Some(config.clone());
    }

    let mut candidates = Vec::new();
    if path.is_dir() {
        candidates.push(path.to_path_buf());
    } else if let Some(parent) = path.parent() {
        candidates.push(parent.to_path_buf());
    }

    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }

    for dir in candidates {
        let candidate = dir.join(".agnix.toml");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn show_diff(original: &str, fixed: &str) {
    let diff = TextDiff::from_lines(original, fixed);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => print!("    {} {}", "-".red(), change.to_string().red()),
            ChangeTag::Insert => print!("    {} {}", "+".green(), change.to_string().green()),
            ChangeTag::Equal => {}
        }
    }
}

fn init_command(output: &PathBuf) -> anyhow::Result<()> {
    let default_config = LintConfig::default();
    let toml_content = toml::to_string_pretty(&default_config)?;

    std::fs::write(output, toml_content)?;

    println!("{} {}", "Created:".green().bold(), output.display());

    Ok(())
}

fn eval_command(
    path: &Path,
    format: EvalOutputFormat,
    filter: Option<&str>,
    verbose: bool,
) -> anyhow::Result<()> {
    let config = LintConfig::default();

    println!("{} {}", "Evaluating:".cyan().bold(), path.display());
    if let Some(f) = filter {
        println!("  {} {}", "filter:".dimmed(), f);
    }
    println!();

    let (results, summary) = evaluate_manifest_file(path, &config, filter)?;

    // Show verbose per-case results if requested
    if verbose {
        println!("{}", "Per-Case Results".cyan().bold());
        println!("{}", "=".repeat(60).dimmed());

        for result in &results {
            let status = if result.passed() {
                "PASS".green().bold()
            } else {
                "FAIL".red().bold()
            };

            println!("[{}] {}", status, result.case.file.display());

            if let Some(desc) = &result.case.description {
                println!("     {}", desc.dimmed());
            }

            if !result.passed() {
                if !result.false_positives.is_empty() {
                    println!(
                        "     {} {:?}",
                        "unexpected:".yellow(),
                        result.false_positives
                    );
                }
                if !result.false_negatives.is_empty() {
                    println!("     {} {:?}", "missing:".red(), result.false_negatives);
                }
            }
            println!();
        }

        println!("{}", "=".repeat(60).dimmed());
        println!();
    }

    // Output summary in requested format
    let eval_format: EvalFormat = format.into();
    match eval_format {
        EvalFormat::Json => {
            let json = summary.to_json()?;
            println!("{}", json);
        }
        EvalFormat::Csv => {
            let csv = summary.to_csv();
            println!("{}", csv);
        }
        EvalFormat::Markdown => {
            let md = summary.to_markdown();
            println!("{}", md);
        }
    }

    // Print final status
    println!();
    if summary.cases_failed == 0 {
        println!(
            "{} All {} cases passed",
            "SUCCESS".green().bold(),
            summary.cases_run
        );
    } else {
        println!(
            "{} {}/{} cases failed",
            "FAILED".red().bold(),
            summary.cases_failed,
            summary.cases_run
        );
        process::exit(1);
    }

    Ok(())
}
