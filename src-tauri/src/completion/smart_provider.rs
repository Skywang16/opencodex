//! Smart completion provider

use crate::completion::command_line::extract_command_key;
use crate::completion::context_analyzer::{
    ArgType, CompletionPosition, ContextAnalysis, ContextAnalyzer,
};
use crate::completion::error::CompletionProviderResult;
use crate::completion::metadata::CommandRegistry;
use crate::completion::prediction::CommandPredictor;
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::repositories::CompletionModelRepo;
use crate::storage::DatabaseManager;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SmartCompletionProvider {
    context_analyzer: Arc<ContextAnalyzer>,
    filesystem_provider: Arc<dyn CompletionProvider>,
    system_commands_provider: Arc<dyn CompletionProvider>,
    history_provider: Arc<dyn CompletionProvider>,
    context_aware_provider: Option<Arc<dyn CompletionProvider>>,
    predictor: Option<CommandPredictor>,
    database: Arc<DatabaseManager>,
}

impl SmartCompletionProvider {
    pub fn new(
        filesystem_provider: Arc<dyn CompletionProvider>,
        system_commands_provider: Arc<dyn CompletionProvider>,
        history_provider: Arc<dyn CompletionProvider>,
        database: Arc<DatabaseManager>,
    ) -> Self {
        // Get current working directory, initialize predictor
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let predictor = Some(CommandPredictor::new(current_dir));

        Self {
            context_analyzer: Arc::new(ContextAnalyzer::new()),
            filesystem_provider,
            system_commands_provider,
            history_provider,
            context_aware_provider: None,
            predictor,
            database,
        }
    }

    /// Set context-aware provider (for entity enhancement)
    pub fn with_context_aware(mut self, provider: Arc<dyn CompletionProvider>) -> Self {
        self.context_aware_provider = Some(provider);
        self
    }

    /// Intelligently provide completions based on context
    async fn provide_smart_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        match &analysis.position {
            CompletionPosition::Command => self.provide_command_completions(context).await,
            CompletionPosition::Option => self.provide_option_completions(context, analysis).await,
            CompletionPosition::OptionValue { option } => {
                self.provide_option_value_completions(context, analysis, option)
                    .await
            }
            CompletionPosition::Subcommand { parent } => {
                self.provide_subcommand_completions(context, parent).await
            }
            CompletionPosition::Argument { command, position } => {
                self.provide_argument_completions(context, command, *position)
                    .await
            }
            CompletionPosition::FilePath => self.provide_filepath_completions(context).await,
            CompletionPosition::Unknown => self.provide_fallback_completions(context).await,
        }
    }

    /// Provide command completions
    async fn provide_command_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // `cd` without a trailing space: still offer directory completions by inserting a leading space.
        if context.input.trim() == "cd"
            && context.cursor_position == context.input.len()
            && !context.input.chars().any(|c| c.is_whitespace())
        {
            let ctx = crate::completion::types::CompletionContext::new(
                "cd ".to_string(),
                context.cursor_position + 1,
                context.working_directory.clone(),
            );
            if let Ok(mut dir_items) = self.filesystem_provider.provide_completions(&ctx).await {
                for item in &mut dir_items {
                    item.text = format!(" {}", item.text);
                    item.score = item.score.max(90.0);
                    item.source = Some("smart".to_string());
                }
                items.extend(dir_items);
            }
        }

        // Step 1: Predict next command (learning model first, static table fallback)
        items.extend(self.predict_next_command_items(context).await);

        // Step 2: Get related commands from history
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
            items.extend(history_items);
        }

        // Step 3: Get from system commands
        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(context)
            .await
        {
            items.extend(system_items);
        }

        // Step 4: Deduplicate, normalize scores, sort
        items = self.deduplicate_and_sort(items);

        Ok(items)
    }

    /// Provide option completions
    async fn provide_option_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        if let Some(token) = analysis.tokens.first() {
            let command = token.text(&context.input);

            // Get options from command knowledge base
            if let Some(meta) = self.context_analyzer.get_command_meta(command) {
                for option in &meta.options {
                    // Add long options
                    if let Some(long) = &option.long {
                        if long.starts_with(&context.current_word) {
                            let mut item =
                                CompletionItem::new(long.clone(), CompletionType::Option)
                                    .with_description(option.description.clone())
                                    .with_source("builtin".to_string())
                                    .with_score(90.0);

                            if option.takes_value {
                                item = item.with_display_text(format!("{long} <value>"));
                            }

                            items.push(item);
                        }
                    }

                    // Add short options
                    if let Some(short) = &option.short {
                        if short.starts_with(&context.current_word) {
                            let mut item =
                                CompletionItem::new(short.clone(), CompletionType::Option)
                                    .with_description(option.description.clone())
                                    .with_source("builtin".to_string())
                                    .with_score(85.0);

                            if option.takes_value {
                                item = item.with_display_text(format!("{short} <value>"));
                            }

                            items.push(item);
                        }
                    }
                }
            }

            // Get commonly used options for this command from history
            if let Ok(history_items) = self.history_provider.provide_completions(context).await {
                for item in history_items {
                    if item.text.starts_with('-') && item.text.starts_with(&context.current_word) {
                        let score = item.score;
                        items.push(
                            item.with_source("history".to_string())
                                .with_score(score * 0.8),
                        );
                    }
                }
            }
        }

        // Sort by score (using CompletionItem's Ord implementation)
        items.sort_unstable();

        Ok(items)
    }

    /// Provide option value completions
    async fn provide_option_value_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
        option: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Get command name (for looking up metadata)
        let command = analysis
            .tokens
            .first()
            .map(|t| t.text(&context.input))
            .unwrap_or("");

        // Provide completions based on option type
        if self.is_file_option(command, option) {
            // File type option
            if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await {
                items.extend(file_items);
            }
        } else if self.is_directory_option(command, option) {
            // Directory type option
            if let Ok(dir_items) = self.filesystem_provider.provide_completions(context).await {
                let dir_items: Vec<_> = dir_items
                    .into_iter()
                    .filter(|item| item.completion_type == CompletionType::Directory.to_string())
                    .collect();
                items.extend(dir_items);
            }
        } else {
            // Find commonly used values for this option from history
            if let Ok(history_items) = self.history_provider.provide_completions(context).await {
                items.extend(history_items);
            }
        }

        Ok(items)
    }

    /// Provide subcommand completions
    async fn provide_subcommand_completions(
        &self,
        context: &CompletionContext,
        parent: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Get subcommands from command knowledge base
        if let Some(meta) = self.context_analyzer.get_command_meta(parent) {
            for subcommand in &meta.subcommands {
                if subcommand.starts_with(&context.current_word) {
                    let item = CompletionItem::new(subcommand.clone(), CompletionType::Subcommand)
                        .with_description(format!("{parent} subcommand"))
                        .with_source("builtin".to_string())
                        .with_score(95.0);
                    items.push(item);
                }
            }
        }

        // Get commonly used subcommand combinations from history
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
            for item in history_items {
                // Filter items that look like subcommands
                if !item.text.starts_with('-') && item.text.starts_with(&context.current_word) {
                    let score = item.score;
                    items.push(
                        item.with_source("history".to_string())
                            .with_score(score * 0.9),
                    );
                }
            }
        }

        // Sort by score and deduplicate
        items = self.deduplicate_and_sort(items);

        Ok(items)
    }

    /// Provide argument completions
    async fn provide_argument_completions(
        &self,
        context: &CompletionContext,
        command: &str,
        position: usize,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Provide completions based on command's argument type
        if let Some(meta) = self.context_analyzer.get_command_meta(command) {
            if let Some(arg_type) = meta.arg_types.get(position) {
                match arg_type {
                    ArgType::File | ArgType::Directory => {
                        if let Ok(file_items) =
                            self.filesystem_provider.provide_completions(context).await
                        {
                            if matches!(arg_type, ArgType::Directory) {
                                let dir_items: Vec<_> = file_items
                                    .into_iter()
                                    .filter(|item| {
                                        item.completion_type
                                            == CompletionType::Directory.to_string()
                                    })
                                    .collect();
                                items.extend(dir_items);
                            } else {
                                items.extend(file_items);
                            }
                        }
                    }
                    ArgType::Enum(values) => {
                        for value in values {
                            if value.starts_with(&context.current_word) {
                                let item =
                                    CompletionItem::new(value.clone(), CompletionType::Value)
                                        .with_source("builtin".to_string())
                                        .with_score(90.0);
                                items.push(item);
                            }
                        }
                    }
                    _ => {
                        // For other types, get from history
                        if let Ok(history_items) =
                            self.history_provider.provide_completions(context).await
                        {
                            items.extend(history_items);
                        }
                    }
                }
            }
        } else {
            // Commands without metadata, use heuristic rules
            if self.command_usually_takes_files(command) {
                if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await
                {
                    items.extend(file_items);
                }
            } else {
                // Get from history
                if let Ok(history_items) = self.history_provider.provide_completions(context).await
                {
                    items.extend(history_items);
                }
            }
        }

        // Enhance completions: add related completions from entities extracted by OutputAnalyzer
        items.extend(self.enhance_with_entities(command, context).await);

        Ok(items)
    }

    /// Enhance completions using entities extracted by OutputAnalyzer
    async fn enhance_with_entities(
        &self,
        command: &str,
        _context: &CompletionContext,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // If no context provider, skip
        if self.context_aware_provider.is_none() {
            return items;
        }

        use crate::completion::output_analyzer::OutputAnalyzer;

        let analyzer = OutputAnalyzer::global();
        let provider = analyzer.context_provider();

        match command {
            "kill" | "killall" => {
                // Add recent PIDs for kill command
                for pid in provider.get_recent_pids() {
                    let item = CompletionItem::new(pid, CompletionType::Value)
                        .with_score(85.0)
                        .with_description("Recent process ID".to_string())
                        .with_source("context".to_string());
                    items.push(item);
                }
            }
            "lsof" => {
                // Add recent ports for lsof command
                for port in provider.get_recent_ports() {
                    let item = CompletionItem::new(port, CompletionType::Value)
                        .with_score(85.0)
                        .with_description("Recently used port".to_string())
                        .with_source("context".to_string());
                    items.push(item);
                }
            }
            "cd" => {
                // Add recently accessed directories for cd command
                for path in provider.get_recent_paths() {
                    // Only add directories
                    if std::path::Path::new(&path).is_dir() {
                        let item = CompletionItem::new(path, CompletionType::Directory)
                            .with_score(80.0)
                            .with_description("Recently accessed directory".to_string())
                            .with_source("context".to_string());
                        items.push(item);
                    }
                }
            }
            _ => {}
        }

        items
    }

    /// Provide file path completions
    async fn provide_filepath_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        self.filesystem_provider.provide_completions(context).await
    }

    /// Provide fallback completions
    async fn provide_fallback_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Try all providers
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
            items.extend(history_items);
        }

        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(context)
            .await
        {
            items.extend(system_items);
        }

        if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await {
            items.extend(file_items);
        }

        // Limit count and sort
        items = self.deduplicate_and_sort(items);
        items.truncate(20); // Limit fallback completion count

        Ok(items)
    }

    async fn predict_next_command_items(&self, context: &CompletionContext) -> Vec<CompletionItem> {
        use crate::completion::output_analyzer::OutputAnalyzer;

        let Some(ref predictor) = self.predictor else {
            return Vec::new();
        };

        let analyzer = OutputAnalyzer::global();
        let (last_cmd, last_output) = {
            let provider = analyzer.context_provider();
            let Some(last) = provider.get_last_command() else {
                return Vec::new();
            };

            last
        };

        // 1) Learning model: prev_key -> next_key topK
        let mut predictions = self
            .learned_next_commands(&last_cmd, &context.current_word)
            .await
            .into_iter()
            .map(|(suggested, confidence)| {
                let mut pred = predictor.build_prediction_for_suggestion(
                    &suggested,
                    &last_cmd,
                    Some(&last_output),
                );
                pred.confidence = pred.confidence.max(confidence).min(100.0);
                pred
            })
            .collect::<Vec<_>>();

        // 2) Fallback: static workflow table
        if predictions.is_empty() {
            predictions = predictor.predict_next_commands(
                &last_cmd,
                Some(&last_output),
                &context.current_word,
            );
        }

        predictions
            .into_iter()
            .map(|p| p.to_completion_item())
            .collect()
    }

    async fn learned_next_commands(
        &self,
        last_command: &str,
        input_prefix: &str,
    ) -> Vec<(String, f64)> {
        let Some(prev_key) = extract_command_key(last_command) else {
            return Vec::new();
        };

        let repo = CompletionModelRepo::new(&self.database);
        let Ok(Some(prev_id)) = repo.find_key_id(&prev_key.key).await else {
            return Vec::new();
        };

        let Ok(rows) = repo.top_next_keys(prev_id, 20).await else {
            return Vec::new();
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        rows.into_iter()
            .filter(|(key, _, _, _)| input_prefix.is_empty() || key.starts_with(input_prefix))
            .map(|(key, count, success_count, last_used_ts)| {
                let confidence = transition_confidence(
                    now,
                    count as u64,
                    success_count as u64,
                    last_used_ts as u64,
                );
                (key, confidence)
            })
            .collect()
    }

    fn deduplicate_and_sort(&self, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        let mut seen: std::collections::HashMap<String, CompletionItem> =
            std::collections::HashMap::new();

        for mut item in items {
            item.score = item.score.clamp(0.0, 100.0);
            seen.entry(item.text.clone())
                .and_modify(|existing| {
                    if item.score > existing.score {
                        *existing = item.clone();
                    }
                })
                .or_insert(item);
        }

        let mut deduped: Vec<CompletionItem> = seen.into_values().collect();
        deduped.sort_unstable();
        deduped
    }

    /// Check if it's a file type option (using command registry)
    fn is_file_option(&self, command: &str, option: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.is_file_option(command, option)
    }

    /// Check if it's a directory type option (using command registry)
    fn is_directory_option(&self, command: &str, option: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.is_directory_option(command, option)
    }

    /// Check if command usually accepts file arguments (using command registry)
    fn command_usually_takes_files(&self, command: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.accepts_files(command)
    }
}

fn transition_confidence(now_ts: u64, count: u64, success_count: u64, last_used_ts: u64) -> f64 {
    if count == 0 {
        return 0.0;
    }

    let success_rate = (success_count as f64 / count as f64).clamp(0.0, 1.0);
    let seconds_ago = now_ts.saturating_sub(last_used_ts);
    let recency = match seconds_ago {
        0..=3600 => 1.0,
        3601..=86400 => 0.8,
        86401..=604800 => 0.6,
        604801..=2592000 => 0.4,
        _ => 0.2,
    };

    let count_factor = ((count as f64).ln_1p() / 4.0).min(1.0); // ln(54)~4
    (count_factor * 60.0 + recency * 25.0 + success_rate * 15.0).min(100.0)
}

#[async_trait]
impl CompletionProvider for SmartCompletionProvider {
    fn name(&self) -> &'static str {
        "smart"
    }

    fn should_provide(&self, _context: &crate::completion::types::CompletionContext) -> bool {
        true // Smart provider can always provide completions
    }

    async fn provide_completions(
        &self,
        context: &crate::completion::types::CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let analysis = self
            .context_analyzer
            .analyze(&context.input, context.cursor_position);
        self.provide_smart_completions(context, &analysis).await
    }

    fn priority(&self) -> i32 {
        100 // Highest priority
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
