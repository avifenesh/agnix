//! Schema definitions for agent config files

pub mod agent;
pub mod claude_md;
pub mod hooks;
pub mod plugin;
pub mod skill;

pub use agent::AgentSchema;
pub use hooks::HooksSchema;
pub use plugin::PluginSchema;
pub use skill::SkillSchema;
