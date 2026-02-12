/// Command sequence prediction module
///
/// Provides intelligent command prediction functionality:
/// - Predict next operations based on command history
/// - Automatically extract entities from output (PID, container ID, file paths, etc.)
/// - Intelligently score based on working directory context
mod command_pairs;
mod predictor;

pub use command_pairs::{get_suggested_commands, matches_command_pattern, COMMAND_PAIRS};
pub use predictor::{CommandPredictor, PredictionResult};
