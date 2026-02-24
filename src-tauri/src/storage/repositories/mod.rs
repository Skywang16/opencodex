/*!
 * Data access module - one simple struct per table, directly using sqlx
 *
 * Design principles:
 * - No abstraction layer: directly use sqlx, no Repository trait
 * - Single responsibility: each struct corresponds to one table
 * - Borrowing preferred: use &DatabaseManager instead of Arc
 */

pub mod ai_models;
pub mod app_preferences;
pub mod audit_logs;

// ==================== Repository structs ====================
pub use ai_models::{
    AIModelConfig, AIModels, AIProvider, AuthType, ModelType, OAuthConfig, OAuthProvider,
};
pub use app_preferences::AppPreferences;
pub use audit_logs::AuditLogs;
