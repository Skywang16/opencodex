use crate::utils::{EmptyData, TauriApiResult};
use crate::vector_db::commands::VectorDbState;
use crate::{api_error, api_success};
use std::path::PathBuf;
use tauri::State;
use tracing::{error, warn};

#[tauri::command]
pub async fn get_index_status(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<crate::vector_db::storage::IndexStatus> {
    let workspace_path = PathBuf::from(&path);

    if !workspace_path.join(".opencodex").join("index").exists() {
        return Ok(api_success!(crate::vector_db::storage::IndexStatus {
            total_files: 0,
            total_chunks: 0,
            embedding_model: String::new(),
            vector_dimension: 0,
            size_bytes: 0,
        }));
    }

    let config = state.search_engine.config().clone();
    match crate::vector_db::storage::IndexManager::new(&workspace_path, config) {
        Ok(manager) => Ok(api_success!(manager.get_status_with_size_bytes())),
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to get index status");
            Ok(api_error!("vector_db.status_failed"))
        }
    }
}

#[tauri::command]
pub async fn delete_workspace_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let root = PathBuf::from(&path);
    let index_dir = root.join(".opencodex").join("index");

    state.search_engine.invalidate_workspace_index(&root);

    if index_dir.exists() {
        let dir = index_dir.clone();
        match tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                warn!(error = %e, path = %path, "Failed to delete index");
                return Ok(api_error!("vector_db.delete_failed"));
            }
            Err(e) => {
                error!("Failed to join delete index task: {}", e);
                return Ok(api_error!("vector_db.delete_failed"));
            }
        }
    }
    Ok(api_success!())
}
