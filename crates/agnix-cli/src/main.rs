//! agnix CLI - The nginx of agent configs

use agnix_core::{
    apply_fixes,
    config::{LintConfig, TargetTool},
    diagnostics::DiagnosticLevel,
    validate_project,
};
use clap::{Parser, Subcommand};
use colored::*;
use similar::{ChangeTag, TextDiff};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

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
    #[arg(short, long, default_value = "generic")]
    target: String,

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
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::Validate { path }) => validate_command(path, &cli),
        Some(Commands::Init { output }) => init_command(output),
        None => validate_command(&cli.path, &cli),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

fn validate_command(path: &Path, cli: &Cli) -> anyhow::Result<()> {
    let config_path = resolve_config_path(path, cli);
    let mut config = LintConfig::load_or_default(config_path.as_ref());
    config.target = match cli.target.as_str() {
        "claude-code" => TargetTool::ClaudeCode,
        "cursor" => TargetTool::Cursor,
        "codex" => TargetTool::Codex,
        _ => TargetTool::Generic,
    };

    println!("{} {}", "Validating:".cyan().bold(), path.display());
    println!();

    let diagnostics = validate_project(path, &config)?;

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
    let should_fix = cli.fix || cli.fix_safe || cli.dry_run;

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

    println!(
        "{} Created config file: {}",
        "✓".green().bold(),
        output.display()
    );

    Ok(())
}
