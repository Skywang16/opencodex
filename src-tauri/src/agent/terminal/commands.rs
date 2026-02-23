use crate::agent::terminal::AgentTerminal;
use crate::agent::terminal::AgentTerminalManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tracing::warn;

#[tauri::command]
pub async fn agent_terminal_list(session_id: Option<i64>) -> TauriApiResult<Vec<AgentTerminal>> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    Ok(api_success!(manager.list_terminals(session_id)))
}

#[tauri::command]
pub async fn agent_terminal_abort(terminal_id: String) -> TauriApiResult<EmptyData> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    match manager.abort_terminal(&terminal_id) {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            warn!("Failed to abort agent terminal {}: {}", terminal_id, e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_terminal_remove(terminal_id: String) -> TauriApiResult<EmptyData> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    match manager.remove_terminal(&terminal_id) {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            warn!("Failed to remove agent terminal {}: {}", terminal_id, e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}
