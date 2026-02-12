//! Prompt module - unified prompt loading and building
//!
//! All prompts are stored in md files under the `prompts/` directory:
//! - `prompts/base/` - base prompt fragments (role, rules, methodology)
//! - `prompts/agents/` - complete agent prompts (with frontmatter configuration)
//! - `prompts/reminders/` - runtime-injected prompts
//! - `prompts/system/` - system-level prompts (compaction, summary, etc.)

mod builder;
mod loader;
pub mod model_harness;
pub mod orchestrator;

pub use builder::{PromptBuilder, SystemPromptParts};
pub use loader::{BuiltinPrompts, PromptLoader};
