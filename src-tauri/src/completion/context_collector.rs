//! Context collector

use crate::completion::error::{ContextCollectorError, ContextCollectorResult};
use crate::completion::types::{
    CommandExecutionContext, CommandOutput, EntityType, OutputDataType, OutputEntity,
    ParsedOutputData,
};
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ContextCollector {
    parsers: HashMap<String, Box<dyn OutputParser + Send + Sync>>,

    contexts: Arc<Mutex<VecDeque<CommandExecutionContext>>>,

    max_contexts: usize,
}

pub trait OutputParser {
    fn name(&self) -> &'static str;

    fn can_parse(&self, command: &str) -> bool;

    fn parse(&self, command: &str, output: &str) -> ContextCollectorResult<ParsedOutputData>;

    fn priority(&self) -> i32 {
        0
    }
}

impl ContextCollector {
    /// Create new context collector
    pub fn new(max_contexts: usize) -> Self {
        let mut collector = Self {
            parsers: HashMap::new(),
            contexts: Arc::new(Mutex::new(VecDeque::new())),
            max_contexts,
        };

        // Register default parsers
        collector.register_default_parsers();
        collector
    }

    /// Register default parsers
    fn register_default_parsers(&mut self) {
        self.register_parser("lsof", Box::new(LsofParser::new()));
        self.register_parser("ps", Box::new(PsParser::new()));
        self.register_parser("netstat", Box::new(NetstatParser::new()));
        self.register_parser("ls", Box::new(LsParser::new()));
        self.register_parser("git", Box::new(GitParser::new()));
        self.register_parser("top", Box::new(TopParser::new()));
        self.register_parser("htop", Box::new(HtopParser::new()));
    }

    /// Register output parser
    pub fn register_parser(&mut self, command: &str, parser: Box<dyn OutputParser + Send + Sync>) {
        self.parsers.insert(command.to_string(), parser);
    }

    /// Parse command output
    fn parse_output(
        &self,
        command: &str,
        output: &str,
    ) -> ContextCollectorResult<ParsedOutputData> {
        let mut best: Option<&(dyn OutputParser + Send + Sync)> = None;
        let mut best_priority = i32::MIN;

        for parser in self.parsers.values() {
            if !parser.can_parse(command) {
                continue;
            }
            let priority = parser.priority();
            if priority > best_priority {
                best_priority = priority;
                best = Some(parser.as_ref());
            }
        }

        match best {
            Some(parser) => parser.parse(command, output),
            None => Ok(ParsedOutputData::new(OutputDataType::Unknown)),
        }
    }

    /// Add context to storage
    fn add_context(&self, context: CommandExecutionContext) -> ContextCollectorResult<()> {
        let mut contexts = self.lock_contexts()?;
        contexts.push_back(context);
        while contexts.len() > self.max_contexts {
            contexts.pop_front();
        }
        Ok(())
    }

    fn lock_contexts(
        &self,
    ) -> ContextCollectorResult<MutexGuard<'_, VecDeque<CommandExecutionContext>>> {
        self.contexts
            .lock()
            .map_err(|_| ContextCollectorError::MutexPoisoned {
                resource: "contexts",
            })
    }
}

impl Default for ContextCollector {
    fn default() -> Self {
        Self::new(1000) // Default save 1000 contexts
    }
}
