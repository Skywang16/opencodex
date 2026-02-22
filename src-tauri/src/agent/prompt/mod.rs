//! Prompt module - unified prompt loading and building
//!
//! All prompts are stored in md files under the `prompts/` directory:
//! - `prompts/models/` - model-family system prompts (primary behavioral layer)
//! - `prompts/agents/` - agent prompts (with frontmatter configuration)
//! - `prompts/reminders/` - runtime-injected prompts
//! - `prompts/system/` - system-level prompts (compaction, summary, etc.)
//!
//! Architecture: agent_prompt and model_profile are mutually exclusive.
//! If an agent defines a custom prompt it replaces the model profile.

mod builder;
mod loader;
pub mod model_harness;
pub mod orchestrator;

pub use builder::{PromptBuilder, SystemPromptParts};
pub use loader::{BuiltinPrompts, PromptLoader};
