//! Schema definitions for agent config files

pub mod agent;
pub mod claude_md;
pub mod cross_platform;
pub mod hooks;
pub mod mcp;
pub mod plugin;
pub mod skill;

pub use agent::AgentSchema;
pub use hooks::HooksSchema;
pub use mcp::McpToolSchema;
pub use plugin::PluginSchema;
pub use skill::SkillSchema;
