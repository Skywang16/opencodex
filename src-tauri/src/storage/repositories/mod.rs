/*!
 * Data access module - one simple struct per table, directly using sqlx
 *
 * Design principles:
 * - No abstraction layer: directly use sqlx, no Repository trait
 * - Single responsibility: each struct corresponds to one table
 * - Borrowing preferred: use &DatabaseManager instead of Arc
 */

pub mod ai_features;
pub mod ai_models;
pub mod app_preferences;
pub mod audit_logs;
pub mod completion_model;

// ==================== Repository structs ====================
pub use ai_features::AIFeatures;
pub use ai_models::{
    AIModelConfig, AIModels, AIProvider, AuthType, ModelType, OAuthConfig, OAuthProvider,
};
pub use app_preferences::AppPreferences;
pub use audit_logs::AuditLogs;
pub use completion_model::CompletionModelRepo;

// ==================== Common query parameters ====================

/// Pagination parameters
#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

impl Pagination {
    pub fn new(limit: i64, offset: i64) -> Self {
        Self { limit, offset }
    }

    pub fn page(page: i64, size: i64) -> Self {
        Self {
            limit: size,
            offset: (page - 1) * size,
        }
    }
}

/// Ordering parameters
#[derive(Debug, Clone)]
pub struct Ordering {
    pub field: String,
    pub desc: bool,
}

impl Ordering {
    pub fn asc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            desc: false,
        }
    }

    pub fn desc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            desc: true,
        }
    }
}
