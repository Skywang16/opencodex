//! Transform layer: Anthropic format to other Provider format conversion
//!
//! ## Design Principles
//!
//! 1. **One-way conversion**: Only Anthropic → Other, no reverse
//! 2. **Centralized management**: One file per provider, easy to maintain and test
//! 3. **Stateless**: All transform functions are pure functions
//!
//! ## Correspondence
//!
//! | Module | Transform Target | Reference |
//! |------|---------|------|
//! | `openai` | OpenAI Chat Completions API | Cline: transform/openai-format.ts |
//! | `gemini` | Google Gemini API | Cline: transform/gemini-format.ts |
//! | `reasoning` | Reasoning/Thinking context | opencode-dev: transform.ts |

pub mod openai;
pub mod reasoning;
// pub mod gemini;  // TODO: Phase 2

pub use openai::*;
pub use reasoning::*;