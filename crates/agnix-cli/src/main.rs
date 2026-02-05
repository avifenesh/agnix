//! agnix CLI - The nginx of agent configs

mod json;
mod sarif;
pub mod telemetry;
mod watch;

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
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

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

    /// Watch mode - re-validate on file changes
    #[arg(short, long)]
    watch: bool,
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

/// Telemetry action for the CLI subcommand.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TelemetryAction {
    /// Show current telemetry status
    #[default]
    Status,
    /// Enable telemetry (opt-in)
    Enable,
    /// Disable telemetry
    Disable,
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

    /// Manage telemetry settings (opt-in usage analytics)
    Telemetry {
        /// Action to perform (status, enable, disable)
        #[arg(value_enum, default_value_t = TelemetryAction::Status)]
        action: TelemetryAction,
    },
}

fn main() {
    let cli = Cli::parse();

    // Initialize tracing for verbose mode (only for text output to avoid corrupting JSON/SARIF)
    if cli.verbose && matches!(cli.format, OutputFormat::Text) {
        use tracing_subscriber::{fmt, prelude::*, EnvFilter};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("agnix=debug,agnix_core=debug"));

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_level(true)
                    .with_writer(std::io::stderr),
            )
            .with(filter)
            .init();

        tracing::debug!("Verbose mode enabled");
    }

    let result = match &cli.command {
        Some(Commands::Validate { path }) => validate_command(path, &cli),
        Some(Commands::Init { output }) => init_command(output),
        Some(Commands::Eval {
            path,
            format,
            filter,
            verbose,
        }) => eval_command(path, *format, filter.as_deref(), *verbose),
        Some(Commands::Telemetry { action }) => telemetry_command(*action),
        None => validate_command(&cli.path, &cli),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

#[tracing::instrument(skip(cli), fields(path = %path.display()))]
fn validate_command(path: &Path, cli: &Cli) -> anyhow::Result<()> {
    tracing::debug!("Starting validation");

    // Watch mode validation
    if cli.watch {
        if !matches!(cli.format, OutputFormat::Text) {
            return Err(anyhow::anyhow!(
                "Watch mode is only supported with text output."
            ));
        }
        let should_fix = cli.fix || cli.fix_safe || cli.dry_run;
        if should_fix {
            return Err(anyhow::anyhow!(
                "Watch mode cannot be combined with fix flags."
            ));
        }

        let path = path.to_path_buf();
        let path_for_watch = path.clone();
        let strict = cli.strict;
        let verbose = cli.verbose;
        let target = cli.target;
        let config_override = cli.config.clone();

        return watch::watch_and_validate(&path_for_watch, move || {
            run_single_validation(&path, strict, verbose, target, config_override.as_ref())
        });
    }

    let config_path = resolve_config_path(path, cli);
    tracing::debug!(config_path = ?config_path, "Resolved config path");

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

    // Time the validation for telemetry
    let validation_start = Instant::now();

    let ValidationResult {
        diagnostics,
        files_checked,
    } = validate_project(path, &config)?;

    let validation_duration = validation_start.elapsed();

    tracing::debug!(
        files_checked = files_checked,
        diagnostics_count = diagnostics.len(),
        "Validation complete"
    );

    // Record telemetry (non-blocking, respects opt-in)
    record_telemetry_event(&diagnostics, validation_duration);

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
            if let Some(assumption) = &diag.assumption {
                println!("  {} {}", "note:".yellow(), assumption);
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

/// Run a single validation pass (for watch mode)
/// Returns true if there are errors
fn run_single_validation(
    path: &Path,
    strict: bool,
    verbose: bool,
    target: TargetArg,
    config_override: Option<&PathBuf>,
) -> anyhow::Result<bool> {
    let config_path = if let Some(c) = config_override {
        Some(c.clone())
    } else {
        resolve_config_path_simple(path)
    };

    let (mut config, config_warning) = LintConfig::load_or_default(config_path.as_ref());

    if let Some(warning) = config_warning {
        eprintln!("{} {}", "Warning:".yellow().bold(), warning);
        eprintln!();
    }
    config.target = target.into();

    let ValidationResult {
        diagnostics,
        files_checked: _,
    } = validate_project(path, &config)?;

    println!("{} {}", "Validating:".cyan().bold(), path.display());
    println!();

    if diagnostics.is_empty() {
        println!("{}", "No issues found".green().bold());
        return Ok(false);
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .count();

    for diag in &diagnostics {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
        };

        println!(
            "{}:{}:{} {}: {}",
            diag.file.display().to_string().dimmed(),
            diag.line,
            diag.column,
            level_str,
            diag.message,
        );

        if verbose {
            println!("  {} {}", "rule:".dimmed(), diag.rule.dimmed());
            if let Some(suggestion) = &diag.suggestion {
                println!("  {} {}", "help:".cyan(), suggestion);
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

    Ok(errors > 0 || (strict && warnings > 0))
}

fn resolve_config_path_simple(path: &Path) -> Option<PathBuf> {
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

/// Record telemetry event for a validation run (non-blocking, respects opt-in).
fn record_telemetry_event(diagnostics: &[agnix_core::Diagnostic], duration: std::time::Duration) {
    use agnix_core::DiagnosticLevel;

    // Count diagnostics by level
    let mut error_count = 0u32;
    let mut warning_count = 0u32;
    let mut info_count = 0u32;

    // Count rule triggers (privacy-safe: only rule IDs, not paths or messages)
    let mut rule_trigger_counts: HashMap<String, u32> = HashMap::new();

    for diag in diagnostics {
        match diag.level {
            DiagnosticLevel::Error => error_count += 1,
            DiagnosticLevel::Warning => warning_count += 1,
            DiagnosticLevel::Info => info_count += 1,
        }

        // Validate rule ID format before including (defense-in-depth)
        // This prevents any bugs in validators from leaking paths/sensitive data
        if telemetry::is_valid_rule_id(&diag.rule) {
            *rule_trigger_counts.entry(diag.rule.clone()).or_insert(0) += 1;
        }
    }

    // File type counts would require exposing file type info from agnix-core
    // For now, we don't collect file type counts to avoid any path exposure
    let file_type_counts: HashMap<String, u32> = HashMap::new();

    // Record the event (spawns background thread, checks if enabled)
    telemetry::record_validation(
        file_type_counts,
        rule_trigger_counts,
        error_count,
        warning_count,
        info_count,
        duration.as_millis() as u64,
    );
}

fn telemetry_command(action: TelemetryAction) -> anyhow::Result<()> {
    use telemetry::TelemetryConfig;

    match action {
        TelemetryAction::Status => {
            let config = TelemetryConfig::load().unwrap_or_default();
            let effective = config.is_enabled();

            println!("{}", "Telemetry Status".cyan().bold());
            println!();
            println!(
                "  {} {}",
                "Configured:".dimmed(),
                if config.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!(
                "  {} {}",
                "Effective:".dimmed(),
                if effective { "enabled" } else { "disabled" }
            );

            if config.enabled && !effective {
                println!();
                println!(
                    "  {} Telemetry is disabled due to environment (CI, DO_NOT_TRACK, etc.)",
                    "note:".yellow()
                );
            }

            if let Some(id) = &config.installation_id {
                // Show only first 8 chars for privacy
                let short_id = if id.len() > 8 { &id[..8] } else { id };
                println!("  {} {}...", "Installation ID:".dimmed(), short_id);
            }

            if let Some(ts) = &config.consent_timestamp {
                println!("  {} {}", "Consent given:".dimmed(), ts);
            }

            println!();
            println!("{}", "Privacy Guarantees".cyan().bold());
            println!("  - Opt-in only (disabled by default)");
            println!("  - No file paths or contents ever collected");
            println!("  - No user identity collected");
            println!("  - Only aggregate counts (file types, rule triggers)");
            println!("  - Respects DO_NOT_TRACK, CI environments");

            if let Ok(path) = TelemetryConfig::config_path() {
                println!();
                println!("  {} {}", "Config file:".dimmed(), path.display());
            }
        }

        TelemetryAction::Enable => {
            let mut config = TelemetryConfig::load().unwrap_or_default();

            if config.enabled {
                println!("{} Telemetry is already enabled.", "note:".cyan());
            } else {
                config.enable()?;
                println!("{} Telemetry enabled.", "OK".green().bold());
                println!();
                println!("Thank you for helping improve agnix!");
                println!();
                println!("{}", "What we collect:".cyan());
                println!("  - File type counts (e.g., 5 skills, 2 MCP configs)");
                println!("  - Rule trigger counts (e.g., AS-001: 3 times)");
                println!("  - Error/warning counts");
                println!("  - Validation duration");
                println!();
                println!("{}", "What we never collect:".cyan());
                println!("  - File paths or names");
                println!("  - File contents or code");
                println!("  - User identity");
                println!();
                println!(
                    "You can disable telemetry at any time with: {}",
                    "agnix telemetry disable".bold()
                );
            }
        }

        TelemetryAction::Disable => {
            let mut config = TelemetryConfig::load().unwrap_or_default();

            if !config.enabled {
                println!("{} Telemetry is already disabled.", "note:".cyan());
            } else {
                config.disable()?;
                println!("{} Telemetry disabled.", "OK".green().bold());
            }
        }
    }

    Ok(())
}
