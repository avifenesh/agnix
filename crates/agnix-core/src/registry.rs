//! Validator registry and factory functions.

use std::collections::{HashMap, HashSet};

use crate::file_types::FileType;
use crate::rules::Validator;

/// Factory function type that creates validator instances.
pub type ValidatorFactory = fn() -> Box<dyn Validator>;

/// A provider of validator factories.
///
/// Implement this trait to supply validators from an external source (e.g., a
/// plugin or a secondary rule set). The built-in validators are packaged as
/// a `BuiltinProvider` (internal to the crate).
///
/// # Example
///
/// ```
/// use agnix_core::{FileType, ValidatorFactory, ValidatorProvider, ValidatorRegistry};
///
/// struct MyProvider;
///
/// impl ValidatorProvider for MyProvider {
///     fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
///         // Return custom validators here
///         vec![]
///     }
/// }
///
/// let registry = ValidatorRegistry::builder()
///     .with_defaults()
///     .with_provider(&MyProvider)
///     .build();
/// ```
pub trait ValidatorProvider: Send + Sync {
    /// Human-readable name for this provider.
    ///
    /// Defaults to the unqualified struct name (e.g., `"BuiltinProvider"`).
    fn name(&self) -> &str {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }

    /// Return the validator factories supplied by this provider.
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)>;
}

/// The built-in validator provider shipping with agnix-core.
///
/// Contains all 19 validators across all supported file types. Used internally
/// by [`ValidatorRegistry::with_defaults`] and
/// [`ValidatorRegistryBuilder::with_defaults`].
pub(crate) struct BuiltinProvider;

impl ValidatorProvider for BuiltinProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        DEFAULTS.to_vec()
    }
}

/// Registry that maps [`FileType`] values to validator factories.
///
/// This is the extension point for the validation engine. A
/// `ValidatorRegistry` owns a set of [`ValidatorFactory`] functions for each
/// supported [`FileType`], and constructs concrete [`Validator`] instances on
/// demand.
///
/// Most callers should use [`ValidatorRegistry::with_defaults`] to obtain a
/// registry pre-populated with all built-in validators. For advanced use cases
/// (custom providers, disabling validators), use [`ValidatorRegistry::builder`].
pub struct ValidatorRegistry {
    validators: HashMap<FileType, Vec<ValidatorFactory>>,
    validator_names: HashMap<FileType, Vec<String>>,
    disabled_validators: HashSet<String>,
}

impl ValidatorRegistry {
    /// Create an empty registry with no registered validators.
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            validator_names: HashMap::new(),
            disabled_validators: HashSet::new(),
        }
    }

    /// Create a registry pre-populated with built-in validators.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Create a [`ValidatorRegistryBuilder`] for ergonomic construction.
    ///
    /// # Example
    ///
    /// ```
    /// use agnix_core::ValidatorRegistry;
    ///
    /// let registry = ValidatorRegistry::builder()
    ///     .with_defaults()
    ///     .without_validator("XmlValidator")
    ///     .build();
    /// ```
    pub fn builder() -> ValidatorRegistryBuilder {
        ValidatorRegistryBuilder::new()
    }

    /// Register a validator factory for a given file type.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) {
        // Cache the validator name once at registration time so disabled
        // validators can be filtered before factory instantiation.
        let validator_name = factory().name().to_string();
        self.validators.entry(file_type).or_default().push(factory);
        self.validator_names
            .entry(file_type)
            .or_default()
            .push(validator_name);
    }

    /// Return the total number of registered validator factories across all file types.
    pub fn total_factory_count(&self) -> usize {
        self.validators.values().map(|v| v.len()).sum()
    }

    /// Build a fresh validator instance list for the given file type.
    ///
    /// Validators whose [`name()`](Validator::name) appears in the
    /// `disabled_validators` set are excluded from the returned list.
    /// When no validators are disabled, the filter is skipped entirely.
    pub fn validators_for(&self, file_type: FileType) -> Vec<Box<dyn Validator>> {
        let factories = match self.validators.get(&file_type) {
            Some(f) => f,
            None => return Vec::new(),
        };

        if self.disabled_validators.is_empty() {
            return factories.iter().map(|factory| factory()).collect();
        }

        let names = match self.validator_names.get(&file_type) {
            Some(names) => names,
            None => return factories.iter().map(|factory| factory()).collect(),
        };

        factories
            .iter()
            .zip(names.iter())
            .filter(|(_, name)| !self.disabled_validators.contains(name.as_str()))
            .map(|(factory, _)| factory())
            .collect()
    }

    /// Disable a validator by name at runtime.
    ///
    /// The name must match the value returned by [`Validator::name()`]
    /// (e.g., `"XmlValidator"`). Disabled validators are excluded from
    /// [`validators_for()`](ValidatorRegistry::validators_for) results.
    pub fn disable_validator(&mut self, name: impl Into<String>) {
        self.disabled_validators.insert(name.into());
    }

    /// Return the number of validator names currently disabled.
    pub fn disabled_validator_count(&self) -> usize {
        self.disabled_validators.len()
    }

    fn register_defaults(&mut self) {
        for &(file_type, factory) in DEFAULTS {
            self.register(file_type, factory);
        }
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Builder for constructing a [`ValidatorRegistry`] with fine-grained control.
///
/// Supports adding built-in validators, custom [`ValidatorProvider`]
/// implementations, individual factories, and disabling validators by name.
///
/// # Example
///
/// ```
/// use agnix_core::ValidatorRegistry;
///
/// let registry = ValidatorRegistry::builder()
///     .with_defaults()
///     .without_validator("PromptValidator")
///     .without_validator("XmlValidator")
///     .build();
///
/// // The built registry excludes PromptValidator and XmlValidator
/// assert!(registry.disabled_validator_count() > 0);
/// ```
pub struct ValidatorRegistryBuilder {
    entries: Vec<(FileType, ValidatorFactory)>,
    disabled_validators: HashSet<String>,
}

impl ValidatorRegistryBuilder {
    /// Create a new empty builder.
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            disabled_validators: HashSet::new(),
        }
    }

    /// Add all built-in validators (equivalent to [`ValidatorRegistry::with_defaults`]).
    ///
    /// This method is additive: calling it multiple times will register
    /// duplicate factories. For most use cases, call it once.
    pub fn with_defaults(&mut self) -> &mut Self {
        self.with_provider(&BuiltinProvider)
    }

    /// Add all validators from a [`ValidatorProvider`].
    pub fn with_provider(&mut self, provider: &dyn ValidatorProvider) -> &mut Self {
        self.entries.extend(provider.validators());
        self
    }

    /// Register a single validator factory for a file type.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) -> &mut Self {
        self.entries.push((file_type, factory));
        self
    }

    /// Mark a validator name as disabled (excluded from the built registry).
    ///
    /// The name must match the value returned by [`Validator::name()`]
    /// (e.g., `"XmlValidator"`).
    pub fn without_validator(&mut self, name: &str) -> &mut Self {
        self.disabled_validators.insert(name.to_string());
        self
    }

    /// Produce a [`ValidatorRegistry`] from this builder.
    ///
    /// Drains the builder's disabled set via [`std::mem::take`], so calling
    /// `build()` a second time produces a registry with no disabled validators.
    /// This is intentional: reuse a builder by calling configuration methods
    /// again before a subsequent `build()`.
    pub fn build(&mut self) -> ValidatorRegistry {
        let mut registry = ValidatorRegistry {
            validators: HashMap::new(),
            validator_names: HashMap::new(),
            disabled_validators: std::mem::take(&mut self.disabled_validators),
        };
        for &(file_type, factory) in &self.entries {
            registry.register(file_type, factory);
        }
        registry
    }
}

// ============================================================================
// Built-in defaults
// ============================================================================

const DEFAULTS: &[(FileType, ValidatorFactory)] = &[
    (FileType::Skill, skill_validator),
    (FileType::Skill, per_client_skill_validator),
    (FileType::Skill, xml_validator),
    (FileType::Skill, imports_validator),
    (FileType::ClaudeMd, claude_md_validator),
    (FileType::ClaudeMd, cross_platform_validator),
    (FileType::ClaudeMd, agents_md_validator),
    (FileType::ClaudeMd, xml_validator),
    (FileType::ClaudeMd, imports_validator),
    (FileType::ClaudeMd, prompt_validator),
    (FileType::Agent, agent_validator),
    (FileType::Agent, xml_validator),
    (FileType::Hooks, hooks_validator),
    (FileType::Plugin, plugin_validator),
    (FileType::Mcp, mcp_validator),
    (FileType::Copilot, copilot_validator),
    (FileType::Copilot, xml_validator),
    (FileType::CopilotScoped, copilot_validator),
    (FileType::CopilotScoped, xml_validator),
    (FileType::CopilotAgent, copilot_validator),
    (FileType::CopilotAgent, xml_validator),
    (FileType::CopilotPrompt, copilot_validator),
    (FileType::CopilotPrompt, xml_validator),
    (FileType::CopilotHooks, copilot_validator),
    (FileType::ClaudeRule, claude_rules_validator),
    (FileType::CursorRule, cursor_validator),
    (FileType::CursorRule, prompt_validator),
    (FileType::CursorRule, claude_md_validator),
    (FileType::CursorHooks, cursor_validator),
    (FileType::CursorAgent, cursor_validator),
    (FileType::CursorEnvironment, cursor_validator),
    (FileType::CursorRulesLegacy, cursor_validator),
    (FileType::CursorRulesLegacy, prompt_validator),
    (FileType::CursorRulesLegacy, claude_md_validator),
    (FileType::ClineRules, cline_validator),
    (FileType::ClineRulesFolder, cline_validator),
    (FileType::OpenCodeConfig, opencode_validator),
    (FileType::GeminiMd, gemini_md_validator),
    (FileType::GeminiMd, prompt_validator),
    (FileType::GeminiMd, xml_validator),
    (FileType::GeminiMd, imports_validator),
    (FileType::GeminiMd, cross_platform_validator),
    (FileType::GeminiSettings, gemini_settings_validator),
    (FileType::GeminiExtension, gemini_extension_validator),
    (FileType::GeminiIgnore, gemini_ignore_validator),
    (FileType::CodexConfig, codex_validator),
    // CodexValidator on ClaudeMd catches AGENTS.override.md files (CDX-003).
    // The validator early-returns for all other ClaudeMd filenames.
    (FileType::ClaudeMd, codex_validator),
    (FileType::RooRules, roo_validator),
    (FileType::RooModes, roo_validator),
    (FileType::RooIgnore, roo_validator),
    (FileType::RooModeRules, roo_validator),
    (FileType::RooMcp, roo_validator),
    (FileType::WindsurfRule, windsurf_validator),
    (FileType::WindsurfWorkflow, windsurf_validator),
    (FileType::WindsurfRulesLegacy, windsurf_validator),
    (FileType::GenericMarkdown, cross_platform_validator),
    (FileType::GenericMarkdown, xml_validator),
    (FileType::GenericMarkdown, imports_validator),
];

// ============================================================================
// Factory functions
// ============================================================================

fn skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::skill::SkillValidator)
}

fn per_client_skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::per_client_skill::PerClientSkillValidator)
}

fn claude_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_md::ClaudeMdValidator)
}

fn agents_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agents_md::AgentsMdValidator)
}

fn agent_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agent::AgentValidator)
}

fn hooks_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::hooks::HooksValidator)
}

fn plugin_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::plugin::PluginValidator)
}

fn mcp_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::mcp::McpValidator)
}

fn xml_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::xml::XmlValidator)
}

fn imports_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::imports::ImportsValidator)
}

fn cross_platform_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cross_platform::CrossPlatformValidator)
}

fn prompt_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::prompt::PromptValidator)
}

fn copilot_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::copilot::CopilotValidator)
}

fn claude_rules_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_rules::ClaudeRulesValidator)
}

fn cursor_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cursor::CursorValidator)
}

fn cline_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cline::ClineValidator)
}

fn opencode_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::opencode::OpenCodeValidator)
}

fn gemini_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_md::GeminiMdValidator)
}

fn gemini_settings_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_settings::GeminiSettingsValidator)
}

fn gemini_extension_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_extension::GeminiExtensionValidator)
}

fn gemini_ignore_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_ignore::GeminiIgnoreValidator)
}

fn codex_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::codex::CodexValidator)
}

fn roo_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::roo::RooCodeValidator)
}

fn windsurf_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::windsurf::WindsurfValidator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ---- BuiltinProvider tests ----

    #[test]
    fn builtin_provider_returns_expected_count() {
        let provider = BuiltinProvider;
        let entries = provider.validators();
        assert_eq!(
            entries.len(),
            DEFAULTS.len(),
            "BuiltinProvider should return the same number of entries as DEFAULTS"
        );
    }

    #[test]
    fn builtin_provider_name() {
        let provider = BuiltinProvider;
        assert_eq!(provider.name(), "BuiltinProvider");
    }

    // ---- Builder tests ----

    #[test]
    fn builder_with_defaults_matches_with_defaults() {
        let via_builder = ValidatorRegistry::builder().with_defaults().build();
        let via_direct = ValidatorRegistry::with_defaults();

        assert_eq!(
            via_builder.total_factory_count(),
            via_direct.total_factory_count(),
            "Builder with_defaults should produce the same factory count as with_defaults()"
        );
    }

    #[test]
    fn builder_empty_produces_empty_registry() {
        let registry = ValidatorRegistry::builder().build();
        assert_eq!(registry.total_factory_count(), 0);
    }

    #[test]
    fn builder_register_adds_single_factory() {
        let registry = ValidatorRegistry::builder()
            .register(FileType::Skill, skill_validator)
            .build();

        assert_eq!(registry.total_factory_count(), 1);
        let validators = registry.validators_for(FileType::Skill);
        assert_eq!(validators.len(), 1);
        assert_eq!(validators[0].name(), "SkillValidator");
    }

    #[test]
    fn builder_without_validator_disables() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("XmlValidator")
            .build();

        // XmlValidator should be excluded from Skill validators
        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(
            !names.contains(&"XmlValidator"),
            "XmlValidator should be disabled, got: {:?}",
            names
        );

        // But SkillValidator should still be present
        assert!(
            names.contains(&"SkillValidator"),
            "SkillValidator should still be present, got: {:?}",
            names
        );
    }

    // ---- Custom provider tests ----

    struct TestProvider;
    impl ValidatorProvider for TestProvider {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Skill, skill_validator)]
        }
    }

    #[test]
    fn custom_provider_adds_validators() {
        let registry = ValidatorRegistry::builder()
            .with_provider(&TestProvider)
            .build();

        assert_eq!(registry.total_factory_count(), 1);
        let validators = registry.validators_for(FileType::Skill);
        assert_eq!(validators.len(), 1);
    }

    #[test]
    fn custom_provider_name() {
        let provider = TestProvider;
        assert_eq!(provider.name(), "TestProvider");
    }

    // ---- disable_validator() direct mutation tests ----

    #[test]
    fn disable_validator_filters_from_results() {
        let mut registry = ValidatorRegistry::with_defaults();
        assert_eq!(registry.disabled_validator_count(), 0);

        registry.disable_validator("XmlValidator");
        assert_eq!(registry.disabled_validator_count(), 1);

        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(!names.contains(&"XmlValidator"));
    }

    struct CountingValidator;

    impl Validator for CountingValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "CountingValidator"
        }
    }

    static COUNTING_VALIDATOR_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

    fn counting_validator_factory() -> Box<dyn Validator> {
        COUNTING_VALIDATOR_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        Box::new(CountingValidator)
    }

    #[test]
    fn validators_for_filters_disabled_before_instantiation() {
        COUNTING_VALIDATOR_CONSTRUCTED.store(0, Ordering::SeqCst);

        let registry = ValidatorRegistry::builder()
            .register(FileType::Skill, counting_validator_factory)
            .without_validator("CountingValidator")
            .build();

        // One construction happens during registration for name caching.
        assert_eq!(COUNTING_VALIDATOR_CONSTRUCTED.load(Ordering::SeqCst), 1);

        // Disabled validators should be filtered before factory() is called.
        let validators = registry.validators_for(FileType::Skill);
        assert!(validators.is_empty());
        assert_eq!(COUNTING_VALIDATOR_CONSTRUCTED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disable_nonexistent_validator_is_harmless() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator("NonExistentValidator");
        assert_eq!(registry.disabled_validator_count(), 1);

        // Should still work normally
        let count_before = ValidatorRegistry::with_defaults().total_factory_count();
        assert_eq!(registry.total_factory_count(), count_before);
    }

    // ---- validators_for filtering ----

    #[test]
    fn validators_for_returns_all_when_none_disabled() {
        let registry = ValidatorRegistry::with_defaults();
        let skill_validators = registry.validators_for(FileType::Skill);
        // Skill has: SkillValidator, PerClientSkillValidator, XmlValidator, ImportsValidator
        assert_eq!(skill_validators.len(), 4);
    }

    #[test]
    fn validators_for_unknown_file_type_returns_empty() {
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::Unknown);
        assert!(validators.is_empty());
    }

    // ---- Multiple disabled validators ----

    #[test]
    fn builder_multiple_without_validators() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("XmlValidator")
            .without_validator("PromptValidator")
            .build();

        assert_eq!(registry.disabled_validator_count(), 2);

        let skill_names: Vec<&str> = registry
            .validators_for(FileType::Skill)
            .iter()
            .map(|v| v.name())
            .collect();
        assert!(!skill_names.contains(&"XmlValidator"));

        let claude_names: Vec<&str> = registry
            .validators_for(FileType::ClaudeMd)
            .iter()
            .map(|v| v.name())
            .collect();
        assert!(!claude_names.contains(&"PromptValidator"));
        assert!(!claude_names.contains(&"XmlValidator"));
    }

    #[test]
    fn disable_all_validators_for_file_type() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("SkillValidator")
            .without_validator("PerClientSkillValidator")
            .without_validator("XmlValidator")
            .without_validator("ImportsValidator")
            .build();

        assert!(
            registry.validators_for(FileType::Skill).is_empty(),
            "All Skill validators disabled, should return empty"
        );
    }

    #[test]
    fn disable_same_validator_twice_is_idempotent() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator("XmlValidator");
        registry.disable_validator("XmlValidator");
        assert_eq!(registry.disabled_validator_count(), 1);
    }

    // ---- Multiple providers ----

    struct ProviderA;
    impl ValidatorProvider for ProviderA {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Skill, skill_validator)]
        }
    }

    struct ProviderB;
    impl ValidatorProvider for ProviderB {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Agent, agent_validator)]
        }
    }

    #[test]
    fn builder_multiple_providers() {
        let registry = ValidatorRegistry::builder()
            .with_provider(&ProviderA)
            .with_provider(&ProviderB)
            .build();

        assert!(!registry.validators_for(FileType::Skill).is_empty());
        assert!(!registry.validators_for(FileType::Agent).is_empty());
        assert_eq!(registry.total_factory_count(), 2);
    }

    // ---- Backward compatibility ----

    #[test]
    fn with_defaults_returns_expected_factories() {
        let registry = ValidatorRegistry::with_defaults();
        assert_eq!(
            registry.total_factory_count(),
            DEFAULTS.len(),
            "with_defaults() should register exactly as many factories as DEFAULTS"
        );
    }

    #[test]
    fn default_trait_matches_with_defaults() {
        let via_default = ValidatorRegistry::default();
        let via_explicit = ValidatorRegistry::with_defaults();
        assert_eq!(
            via_default.total_factory_count(),
            via_explicit.total_factory_count()
        );
    }
}
