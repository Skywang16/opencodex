// Tools interface and builtins for Agent module
// Real implementation after migration

pub mod builtin;
pub mod logger;
pub mod metadata;
pub mod parallel;
pub mod registry;
pub mod r#trait;
// Re-exports for external use
pub use logger::ToolExecutionLogger;
pub use metadata::{
    BackoffStrategy, ExecutionMode, RateLimitConfig, ToolCategory, ToolMetadata, ToolPriority,
};
pub use parallel::{execute_batch, BatchToolResult, ToolCall};
pub use r#trait::{
    RunnableTool, ToolAvailabilityContext, ToolDescriptionContext, ToolResult, ToolResultContent,
    ToolResultStatus, ToolSchema,
};
pub use registry::ToolConfirmationManager;
pub use registry::{ToolExecutionStats, ToolRegistry};

// Builtin tool type re-exports
pub use builtin::{
    GlobTool, GrepTool, ListFilesTool, LspQueryTool, MultiEditTool, ReadFileTool, ReadTerminalTool,
    SemanticSearchTool, ShellTool, SyntaxDiagnosticsTool, TaskTool, TodoWriteTool, UnifiedEditTool,
    WebFetchTool, WebSearchTool, WriteFileTool,
};

use std::sync::Arc;
use tracing::error;

pub async fn create_tool_registry(
    chat_mode: &str,
    permission_rules: crate::settings::types::PermissionRules,
    agent_tool_filter: Option<crate::agent::permissions::ToolFilter>,
    confirmations: Arc<ToolConfirmationManager>,
    extra_tools: Vec<Arc<dyn RunnableTool>>,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    lsp_manager: Option<Arc<crate::lsp::LspManager>>,
    skill_manager: Option<Arc<crate::agent::skill::SkillManager>>,
) -> Arc<ToolRegistry> {
    let checker = Arc::new(crate::agent::permissions::PermissionChecker::new(
        &permission_rules,
    ));
    let agent_filter = agent_tool_filter.map(Arc::new);
    let registry = Arc::new(ToolRegistry::new(
        Some(checker),
        agent_filter,
        confirmations,
    ));
    let is_chat = chat_mode == "chat";

    let availability_ctx = ToolAvailabilityContext {
        has_vector_index: vector_search_engine.is_some(),
    };

    register_builtin_tools(
        &registry,
        is_chat,
        &availability_ctx,
        vector_search_engine,
        lsp_manager,
        skill_manager,
    )
    .await;

    for tool in extra_tools {
        let name = tool.name().to_string();
        register_tool(&registry, &name, tool, is_chat, &availability_ctx).await;
    }

    registry
}

async fn register_builtin_tools(
    registry: &ToolRegistry,
    is_chat_mode: bool,
    availability_ctx: &ToolAvailabilityContext,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    lsp_manager: Option<Arc<crate::lsp::LspManager>>,
    skill_manager: Option<Arc<crate::agent::skill::SkillManager>>,
) {
    use std::sync::Arc;

    register_tool(
        registry,
        "task",
        Arc::new(TaskTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "todowrite",
        Arc::new(TodoWriteTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "web_fetch",
        Arc::new(WebFetchTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "web_search",
        Arc::new(WebSearchTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "read_file",
        Arc::new(ReadFileTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    register_tool(
        registry,
        "write_file",
        Arc::new(WriteFileTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    register_tool(
        registry,
        "edit_file",
        Arc::new(UnifiedEditTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    register_tool(
        registry,
        "multi_edit_file",
        Arc::new(MultiEditTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    register_tool(
        registry,
        "list_files",
        Arc::new(ListFilesTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "shell",
        Arc::new(ShellTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    // Search tools
    register_tool(
        registry,
        "grep",
        Arc::new(GrepTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    register_tool(
        registry,
        "glob",
        Arc::new(GlobTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;
    if let Some(engine) = vector_search_engine {
        register_tool(
            registry,
            "semantic_search",
            Arc::new(SemanticSearchTool::new(engine)),
            is_chat_mode,
            availability_ctx,
        )
        .await;
    }
    if let Some(manager) = lsp_manager {
        register_tool(
            registry,
            "lsp_query",
            Arc::new(LspQueryTool::new(manager)),
            is_chat_mode,
            availability_ctx,
        )
        .await;
    }

    register_tool(
        registry,
        "read_terminal",
        Arc::new(ReadTerminalTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    register_tool(
        registry,
        "syntax_diagnostics",
        Arc::new(SyntaxDiagnosticsTool::new()),
        is_chat_mode,
        availability_ctx,
    )
    .await;

    // Register Skill tool
    if let Some(manager) = skill_manager {
        register_tool(
            registry,
            "skill",
            Arc::new(crate::agent::skill::SkillTool::new(manager)),
            is_chat_mode,
            availability_ctx,
        )
        .await;
    }
}

async fn register_tool(
    registry: &ToolRegistry,
    name: &str,
    tool: Arc<dyn RunnableTool>,
    is_chat_mode: bool,
    availability_ctx: &ToolAvailabilityContext,
) {
    if let Err(err) = registry
        .register(name, tool, is_chat_mode, availability_ctx)
        .await
    {
        error!("failed to register tool '{name}': {err}");
    }
}
