use super::*;

/// Builder for constructing a [`LintConfig`] with validation.
///
/// Uses the `&mut Self` return pattern (consistent with [`ValidatorRegistryBuilder`])
/// for chaining setter calls, with a terminal `build()` that validates and returns
/// `Result<LintConfig, ConfigError>`.
///
/// **Note:** `build()` and `build_unchecked()` drain the builder's state.
/// A second call will produce a default config. Create a new builder if needed.
///
/// # Examples
///
/// ```rust
/// use agnix_core::config::{LintConfig, SeverityLevel};
///
/// let config = LintConfig::builder()
///     .severity(SeverityLevel::Error)
///     .build()
///     .expect("valid config");
/// assert_eq!(config.severity(), SeverityLevel::Error);
/// ```
pub struct LintConfigBuilder {
    severity: Option<SeverityLevel>,
    rules: Option<RuleConfig>,
    exclude: Option<Vec<String>>,
    target: Option<TargetTool>,
    tools: Option<Vec<String>>,
    mcp_protocol_version: Option<Option<String>>,
    tool_versions: Option<ToolVersions>,
    spec_revisions: Option<SpecRevisions>,
    files: Option<FilesConfig>,
    locale: Option<Option<String>>,
    max_files_to_validate: Option<Option<usize>>,
    // Runtime
    root_dir: Option<PathBuf>,
    import_cache: Option<crate::parsers::ImportCache>,
    fs: Option<Arc<dyn FileSystem>>,
    disabled_rules: Vec<String>,
    disabled_validators: Vec<String>,
}

impl LintConfigBuilder {
    fn append_and_dedup(target: &mut Vec<String>, source: &mut Vec<String>) {
        if source.is_empty() {
            return;
        }

        target.append(source);
        let mut seen = std::collections::HashSet::new();
        target.retain(|item| seen.insert(item.clone()));
    }

    /// Create a new builder with all fields unset (defaults will be applied at build time).
    ///
    /// Prefer [`LintConfig::builder()`] over calling this directly.
    fn new() -> Self {
        Self {
            severity: None,
            rules: None,
            exclude: None,
            target: None,
            tools: None,
            mcp_protocol_version: None,
            tool_versions: None,
            spec_revisions: None,
            files: None,
            locale: None,
            max_files_to_validate: None,
            root_dir: None,
            import_cache: None,
            fs: None,
            disabled_rules: Vec::new(),
            disabled_validators: Vec::new(),
        }
    }

    /// Set the severity level threshold.
    pub fn severity(&mut self, severity: SeverityLevel) -> &mut Self {
        self.severity = Some(severity);
        self
    }

    /// Set the rules configuration.
    pub fn rules(&mut self, rules: RuleConfig) -> &mut Self {
        self.rules = Some(rules);
        self
    }

    /// Set the exclude patterns.
    pub fn exclude(&mut self, exclude: Vec<String>) -> &mut Self {
        self.exclude = Some(exclude);
        self
    }

    /// Set the target tool.
    pub fn target(&mut self, target: TargetTool) -> &mut Self {
        self.target = Some(target);
        self
    }

    /// Set the tools list.
    pub fn tools(&mut self, tools: Vec<String>) -> &mut Self {
        self.tools = Some(tools);
        self
    }

    /// Set the MCP protocol version (deprecated field).
    pub fn mcp_protocol_version(&mut self, version: Option<String>) -> &mut Self {
        self.mcp_protocol_version = Some(version);
        self
    }

    /// Set the tool versions configuration.
    pub fn tool_versions(&mut self, versions: ToolVersions) -> &mut Self {
        self.tool_versions = Some(versions);
        self
    }

    /// Set the spec revisions configuration.
    pub fn spec_revisions(&mut self, revisions: SpecRevisions) -> &mut Self {
        self.spec_revisions = Some(revisions);
        self
    }

    /// Set the files configuration.
    pub fn files(&mut self, files: FilesConfig) -> &mut Self {
        self.files = Some(files);
        self
    }

    /// Set the locale.
    pub fn locale(&mut self, locale: Option<String>) -> &mut Self {
        self.locale = Some(locale);
        self
    }

    /// Set the maximum number of files to validate.
    pub fn max_files_to_validate(&mut self, max: Option<usize>) -> &mut Self {
        self.max_files_to_validate = Some(max);
        self
    }

    /// Set the runtime validation root directory.
    pub fn root_dir(&mut self, root_dir: PathBuf) -> &mut Self {
        self.root_dir = Some(root_dir);
        self
    }

    /// Set the shared import cache.
    pub fn import_cache(&mut self, cache: crate::parsers::ImportCache) -> &mut Self {
        self.import_cache = Some(cache);
        self
    }

    /// Set the filesystem abstraction.
    pub fn fs(&mut self, fs: Arc<dyn FileSystem>) -> &mut Self {
        self.fs = Some(fs);
        self
    }

    /// Add a rule ID to the disabled rules list.
    pub fn disable_rule(&mut self, rule_id: impl Into<String>) -> &mut Self {
        self.disabled_rules.push(rule_id.into());
        self
    }

    /// Add a validator name to the disabled validators list.
    pub fn disable_validator(&mut self, name: impl Into<String>) -> &mut Self {
        self.disabled_validators.push(name.into());
        self
    }

    /// Build the `LintConfig`, applying defaults for unset fields and
    /// running validation.
    ///
    /// Returns `Err(ConfigError)` if:
    /// - A glob pattern (in exclude or files config) has invalid syntax
    /// - A glob pattern attempts path traversal (`../`)
    /// - Configuration validation produces warnings (promoted to errors)
    pub fn build(&mut self) -> Result<LintConfig, ConfigError> {
        let config = self.build_inner();

        // Validate all glob pattern lists: exclude + files config
        let pattern_lists: &[(&str, &[String])] = &[
            ("exclude", &config.exclude),
            ("files.include_as_memory", &config.files.include_as_memory),
            ("files.include_as_generic", &config.files.include_as_generic),
            ("files.exclude", &config.files.exclude),
        ];
        for &(field, patterns) in pattern_lists {
            for pattern in patterns {
                let normalized = pattern.replace('\\', "/");
                if let Err(e) = glob::Pattern::new(&normalized) {
                    return Err(ConfigError::InvalidGlobPattern {
                        pattern: pattern.clone(),
                        error: format!("{} (in {})", e, field),
                    });
                }
                if has_path_traversal(&normalized) {
                    return Err(ConfigError::PathTraversal {
                        pattern: pattern.clone(),
                    });
                }
            }
        }

        // Run full config validation
        let warnings = config.validate();
        if !warnings.is_empty() {
            return Err(ConfigError::ValidationFailed(warnings));
        }

        Ok(config)
    }

    /// Build the `LintConfig` without running any validation.
    ///
    /// This is primarily intended for tests that need to construct configs
    /// with intentionally invalid data.
    pub fn build_unchecked(&mut self) -> LintConfig {
        self.build_inner()
    }

    /// Internal: construct the LintConfig from builder state, applying defaults.
    fn build_inner(&mut self) -> LintConfig {
        let defaults = LintConfig::default();

        let mut rules = self.rules.take().unwrap_or(defaults.rules);

        // Apply convenience disabled_rules/disabled_validators.
        Self::append_and_dedup(&mut rules.disabled_rules, &mut self.disabled_rules);
        Self::append_and_dedup(
            &mut rules.disabled_validators,
            &mut self.disabled_validators,
        );

        let mut config = LintConfig {
            severity: self.severity.take().unwrap_or(defaults.severity),
            rules,
            exclude: self.exclude.take().unwrap_or(defaults.exclude),
            target: self.target.take().unwrap_or(defaults.target),
            tools: self.tools.take().unwrap_or(defaults.tools),
            mcp_protocol_version: self
                .mcp_protocol_version
                .take()
                .unwrap_or(defaults.mcp_protocol_version),
            tool_versions: self.tool_versions.take().unwrap_or(defaults.tool_versions),
            spec_revisions: self
                .spec_revisions
                .take()
                .unwrap_or(defaults.spec_revisions),
            files: self.files.take().unwrap_or(defaults.files),
            locale: self.locale.take().unwrap_or(defaults.locale),
            max_files_to_validate: self
                .max_files_to_validate
                .take()
                .unwrap_or(defaults.max_files_to_validate),
            runtime: RuntimeContext::default(),
        };

        // Apply runtime state
        if let Some(root_dir) = self.root_dir.take() {
            config.runtime.root_dir = Some(root_dir);
        }
        if let Some(cache) = self.import_cache.take() {
            config.runtime.import_cache = Some(cache);
        }
        if let Some(fs) = self.fs.take() {
            config.runtime.fs = fs;
        }

        config
    }
}

impl Default for LintConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LintConfig {
    /// Create a new [`LintConfigBuilder`] for constructing a `LintConfig`.
    pub fn builder() -> LintConfigBuilder {
        LintConfigBuilder::new()
    }
}
