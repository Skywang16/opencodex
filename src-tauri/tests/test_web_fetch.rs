#[cfg(test)]
mod web_fetch_tests {
    use serde_json::json;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use terminal_lib::agent::config::TaskExecutionConfig;
    use terminal_lib::agent::core::context::{
        TaskContext, TaskContextDeps, TaskContextInit, TaskExecutionRequest, TaskExecutionResponse,
        TaskExecutionRunner,
    };
    use terminal_lib::agent::error::{TaskExecutorError, TaskExecutorResult};
    use terminal_lib::agent::persistence::AgentPersistence;
    use terminal_lib::agent::tools::builtin::WebFetchTool;
    use terminal_lib::agent::tools::{RunnableTool, ToolRegistry};
    use terminal_lib::agent::workspace_changes::WorkspaceChangeJournal;
    use terminal_lib::storage::{DatabaseManager, DatabaseOptions, StoragePathsBuilder};

    struct NoopTaskExecutionRunner;

    #[async_trait::async_trait]
    impl TaskExecutionRunner for NoopTaskExecutionRunner {
        async fn run_task_execution(
            &self,
            _parent: &TaskContext,
            _request: TaskExecutionRequest,
        ) -> TaskExecutorResult<TaskExecutionResponse> {
            Err(TaskExecutorError::InternalError(
                "NoopTaskExecutionRunner does not execute child tasks".to_string(),
            ))
        }
    }

    async fn create_test_task_context(root: &Path) -> TaskContext {
        let storage_root = root.join("storage");
        std::fs::create_dir_all(&storage_root).expect("failed to create test storage root");

        let paths = StoragePathsBuilder::new()
            .app_dir(storage_root)
            .build()
            .expect("failed to build storage paths");
        paths
            .ensure_directories()
            .expect("failed to create storage directories");

        let options = DatabaseOptions {
            encryption: false,
            ..DatabaseOptions::default()
        };

        let database = Arc::new(
            DatabaseManager::new(paths, options)
                .await
                .expect("failed to create test database"),
        );
        database
            .initialize()
            .await
            .expect("failed to initialize test database");

        let cwd = std::env::current_dir()
            .expect("failed to resolve current dir")
            .to_string_lossy()
            .to_string();

        TaskContext::new(TaskContextInit {
            task_id: "web-fetch-test".to_string(),
            session_id: 1,
            run_id: 1,
            node_id: 1,
            user_prompt: "test prompt".to_string(),
            agent_type: "chat".to_string(),
            config: TaskExecutionConfig::default(),
            workspace_path: cwd,
            updates_run_status: true,
            emit_task_events: false,
            progress_channel: None,
            deps: TaskContextDeps {
                tool_registry: Arc::new(ToolRegistry::default()),
                repositories: Arc::clone(&database),
                agent_persistence: Arc::new(AgentPersistence::new(Arc::clone(&database))),
                checkpoint_service: None,
                workspace_changes: Arc::new(WorkspaceChangeJournal::new()),
                task_execution_runner: Arc::new(NoopTaskExecutionRunner),
            },
        })
        .await
        .expect("failed to create test task context")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_web_fetch_timeout() {
        let tool = WebFetchTool::new();
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let context = create_test_task_context(temp_dir.path()).await;

        println!("Starting WebFetch test...");
        let start = std::time::Instant::now();

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            tool.run(&context, json!({"url": "http://127.0.0.1"})),
        )
        .await;

        println!("Test completed in {:?}", start.elapsed());

        match result {
            Ok(Ok(tool_result)) => {
                println!("Success: {:?}", tool_result.status);
            }
            Ok(Err(e)) => {
                println!("Tool error: {e:?}");
            }
            Err(_) => {
                panic!("WebFetch timed out after 10 seconds!");
            }
        }
    }
}
