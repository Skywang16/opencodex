//! Agent context orchestration primitives (stage 2).

pub mod builder;
pub mod file_tracker;
pub mod project_context;

pub use crate::agent::config::ContextBuilderConfig;
pub use builder::ContextBuilder;
pub use file_tracker::{
    FileContextTracker, FileOperationRecord, FileRecordSource, FileRecordState, TrackedFileRecord,
};
pub use project_context::{ProjectContext, ProjectContextLoader};

// get_available_rules_files has been moved to crate::workspace::rules
// ProjectContext still uses internal implementation
