//! Intelligent command line context analyzer
//!
//! Based on design principles of excellent completion systems (such as zsh, fish, carapace, etc.),
//! implements intelligent context-aware completion analysis

use std::collections::{HashMap, HashSet};

/// Completion position type
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionPosition {
    /// Command name position
    Command,
    /// Command option position (e.g., -h, --help)
    Option,
    /// Option value position (e.g., --file <filename>)
    OptionValue { option: String },
    /// Subcommand position
    Subcommand { parent: String },
    /// Command argument position
    Argument { command: String, position: usize },
    /// File path position
    FilePath,
    /// Unknown position
    Unknown,
}

/// Token structure
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn text<'a>(&self, input: &'a str) -> &'a str {
        // Safe slicing: ensure indices are within valid range
        input.get(self.start..self.end).unwrap_or("")
    }
}

/// Context analysis result (the "second layer data" of the completion engine)
#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub tokens: Vec<Token>,
    pub current_token_index: Option<usize>,
    pub current_word: String,
    pub position: CompletionPosition,
}

/// Command metadata
#[derive(Debug, Clone)]
pub struct CommandMeta {
    /// Command name
    pub name: String,
    /// Subcommand list
    pub subcommands: Vec<String>,
    /// Option list
    pub options: Vec<CommandOption>,
    /// Whether file arguments are required
    pub takes_files: bool,
    /// Argument type list
    pub arg_types: Vec<ArgType>,
}

/// Command option
#[derive(Debug, Clone)]
pub struct CommandOption {
    /// Short option (e.g., -h)
    pub short: Option<String>,
    /// Long option (e.g., --help)
    pub long: Option<String>,
    /// Whether a value is required
    pub takes_value: bool,
    /// Value type
    pub value_type: Option<ArgType>,
    /// Description
    pub description: String,
}

/// Argument type
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// Arbitrary string
    String,
    /// File path
    File,
    /// Directory path
    Directory,
    /// Number
    Number,
    /// URL
    Url,
    /// Enumeration value
    Enum(Vec<String>),
}

/// Intelligent context analyzer
pub struct ContextAnalyzer {
    /// Built-in command knowledge base
    command_db: HashMap<String, CommandMeta>,
    /// Set of options that require argument values (eliminates hardcoded special cases)
    options_taking_values: HashSet<&'static str>,
}

impl ContextAnalyzer {
    /// Create new context analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            command_db: HashMap::new(),
            options_taking_values: Self::build_options_set(),
        };
        analyzer.load_builtin_commands();
        analyzer
    }

    /// Build set of options that require argument values (initialize once to avoid repeated matching)
    fn build_options_set() -> HashSet<&'static str> {
        let mut set = HashSet::new();
        // Long options
        set.insert("--file");
        set.insert("--output");
        set.insert("--input");
        set.insert("--config");
        set.insert("--directory");
        set.insert("--format");
        set.insert("--type");
        set.insert("--name");
        set.insert("--path");
        set.insert("--url");
        // Short options
        set.insert("-f");
        set.insert("-o");
        set.insert("-i");
        set.insert("-c");
        set.insert("-d");
        set.insert("-t");
        set.insert("-n");
        set.insert("-p");
        set
    }

    /// Analyze command line context
    pub fn analyze(&self, input: &str, cursor_pos: usize) -> ContextAnalysis {
        let tokens = self.tokenize(input);
        let current_token_index = self.find_current_token_index(&tokens, cursor_pos);

        let position = self.determine_position(input, &tokens, current_token_index, cursor_pos);
        let current_word = self.extract_current_word(input, cursor_pos);

        ContextAnalysis {
            tokens,
            current_token_index,
            current_word,
            position,
        }
    }

    /// Tokenize
    fn tokenize(&self, input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut start_pos = 0;

        for (i, ch) in input.char_indices() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(Token {
                            start: start_pos,
                            end: i,
                        });
                        current.clear();
                    }
                    in_quotes = true;
                    quote_char = ch;
                    start_pos = i;
                }
                ch if ch == quote_char && in_quotes => {
                    current.push(ch);
                    tokens.push(Token {
                        start: start_pos,
                        end: i + 1,
                    });
                    current.clear();
                    in_quotes = false;
                    start_pos = i + 1;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(Token {
                            start: start_pos,
                            end: i,
                        });
                        current.clear();
                    }
                    start_pos = i + 1;
                    while start_pos < input.len()
                        && input.chars().nth(start_pos).unwrap().is_whitespace()
                    {
                        start_pos += 1;
                    }
                }
                ch => {
                    if current.is_empty() && !ch.is_whitespace() {
                        start_pos = i;
                    }
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(Token {
                start: start_pos,
                end: input.len(),
            });
        }

        tokens
    }

    /// Find current token index
    fn find_current_token_index(&self, tokens: &[Token], cursor_pos: usize) -> Option<usize> {
        for (i, token) in tokens.iter().enumerate() {
            if cursor_pos >= token.start && cursor_pos <= token.end {
                return Some(i);
            }
        }

        if let Some(last_token) = tokens.last() {
            if cursor_pos > last_token.end {
                return Some(tokens.len());
            }
        }

        None
    }

    /// Determine completion position type
    fn determine_position(
        &self,
        input: &str,
        tokens: &[Token],
        current_index: Option<usize>,
        cursor_pos: usize,
    ) -> CompletionPosition {
        if tokens.is_empty() {
            return CompletionPosition::Command;
        }

        let current_index = current_index.unwrap_or(tokens.len());

        let command_name = tokens[0].text(input);

        // When there's only one token (command name), distinguish:
        // - Cursor still inside command word: complete command name
        // - Cursor after command word and whitespace entered: enter first argument position
        if tokens.len() == 1 && current_index == 1 {
            if let Some(cmd_token) = tokens.first() {
                if cursor_pos > cmd_token.end {
                    // Safe slicing: ensure indices are within valid range
                    let end_pos = cursor_pos.min(input.len());
                    if let Some(tail) = input.get(cmd_token.end..end_pos) {
                        if tail.chars().any(|c| c.is_whitespace()) {
                            return CompletionPosition::Argument {
                                command: command_name.to_string(),
                                position: 0,
                            };
                        }
                    }
                }
            }
            return CompletionPosition::Command;
        }

        if current_index == 0 {
            return CompletionPosition::Command;
        }

        if let Some(cmd_meta) = self.command_db.get(command_name) {
            return self.analyze_with_metadata(input, tokens, current_index, cmd_meta);
        }

        // Analyze based on heuristic rules
        self.analyze_heuristic(input, tokens, current_index)
    }

    /// Analyze based on command metadata
    fn analyze_with_metadata(
        &self,
        input: &str,
        tokens: &[Token],
        current_index: usize,
        meta: &CommandMeta,
    ) -> CompletionPosition {
        if current_index >= tokens.len() {
            // At the last position, check previous token
            if let Some(prev_token) = tokens.get(current_index - 1) {
                let prev_text = prev_token.text(input);
                for option in &meta.options {
                    if option.takes_value {
                        if let Some(long) = &option.long {
                            if prev_text == long {
                                return CompletionPosition::OptionValue {
                                    option: prev_text.to_string(),
                                };
                            }
                        }
                        if let Some(short) = &option.short {
                            if prev_text == short {
                                return CompletionPosition::OptionValue {
                                    option: prev_text.to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }

        let current_token = tokens.get(current_index);

        if let Some(token) = current_token {
            if token.text(input).starts_with('-') {
                return CompletionPosition::Option;
            }
        }

        if !meta.subcommands.is_empty() {
            let non_option_args: Vec<&Token> = tokens
                .get(1..)
                .unwrap_or(&[])
                .iter()
                .filter(|t| !t.text(input).starts_with('-'))
                .collect();

            if non_option_args.is_empty() {
                return CompletionPosition::Subcommand {
                    parent: meta.name.clone(),
                };
            }
        }

        // Default to argument position
        let arg_position = tokens
            .get(1..current_index)
            .unwrap_or(&[])
            .iter()
            .filter(|t| !t.text(input).starts_with('-'))
            .count();

        CompletionPosition::Argument {
            command: meta.name.clone(),
            position: arg_position,
        }
    }

    /// Analyze based on heuristic rules
    fn analyze_heuristic(
        &self,
        input: &str,
        tokens: &[Token],
        current_index: usize,
    ) -> CompletionPosition {
        let current_token = tokens.get(current_index);

        if let Some(token) = current_token {
            if token.text(input).starts_with('-') {
                return CompletionPosition::Option;
            }
        }

        if current_index > 0 {
            if let Some(prev_token) = tokens.get(current_index - 1) {
                let prev_text = prev_token.text(input);
                if self.is_option_that_takes_value(prev_text) {
                    return CompletionPosition::OptionValue {
                        option: prev_text.to_string(),
                    };
                }
            }
        }

        if let Some(token) = current_token {
            if self.looks_like_path(token.text(input)) {
                return CompletionPosition::FilePath;
            }
        }

        // Default to argument
        let command_name = tokens[0].text(input);
        let arg_position = tokens
            .get(1..current_index)
            .unwrap_or(&[])
            .iter()
            .filter(|t| !t.text(input).starts_with('-'))
            .count();

        CompletionPosition::Argument {
            command: command_name.to_string(),
            position: arg_position,
        }
    }

    /// Extract current word
    fn extract_current_word(&self, input: &str, cursor_pos: usize) -> String {
        if input.is_empty() || cursor_pos == 0 {
            return String::new();
        }

        let chars: Vec<char> = input.chars().collect();
        let mut start = cursor_pos.min(chars.len());
        let mut end = cursor_pos.min(chars.len());

        // Search forward for word start
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }

        // Search backward for word end
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }

        // Safe slicing
        if start <= end && end <= chars.len() {
            chars[start..end].iter().collect()
        } else {
            String::new()
        }
    }

    /// Check if option requires value (O(1) lookup)
    fn is_option_that_takes_value(&self, option: &str) -> bool {
        self.options_taking_values.contains(option)
    }

    /// Check if looks like a path
    fn looks_like_path(&self, text: &str) -> bool {
        text.contains('/') || text.contains('\\') || text.starts_with('.') || text.starts_with('~')
    }

    /// Load built-in command knowledge base
    fn load_builtin_commands(&mut self) {
        // Git commands
        self.command_db.insert(
            "git".to_string(),
            CommandMeta {
                name: "git".to_string(),
                subcommands: vec![
                    "add".to_string(),
                    "commit".to_string(),
                    "push".to_string(),
                    "pull".to_string(),
                    "status".to_string(),
                    "branch".to_string(),
                    "checkout".to_string(),
                    "merge".to_string(),
                    "log".to_string(),
                    "diff".to_string(),
                    "clone".to_string(),
                    "init".to_string(),
                    "fetch".to_string(),
                    "reset".to_string(),
                    "rebase".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--help".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Show help information".to_string(),
                    },
                    CommandOption {
                        short: None,
                        long: Some("--version".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Show version information".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::String],
            },
        );

        // Docker commands
        self.command_db.insert(
            "docker".to_string(),
            CommandMeta {
                name: "docker".to_string(),
                subcommands: vec![
                    "run".to_string(),
                    "build".to_string(),
                    "pull".to_string(),
                    "push".to_string(),
                    "ps".to_string(),
                    "images".to_string(),
                    "stop".to_string(),
                    "start".to_string(),
                    "restart".to_string(),
                    "rm".to_string(),
                    "rmi".to_string(),
                    "exec".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--help".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Show help information".to_string(),
                    },
                    CommandOption {
                        short: Some("-f".to_string()),
                        long: Some("--file".to_string()),
                        takes_value: true,
                        value_type: Some(ArgType::File),
                        description: "Specify Dockerfile".to_string(),
                    },
                ],
                takes_files: false,
                arg_types: vec![ArgType::String],
            },
        );

        // NPM commands
        self.command_db.insert(
            "npm".to_string(),
            CommandMeta {
                name: "npm".to_string(),
                subcommands: vec![
                    "install".to_string(),
                    "run".to_string(),
                    "start".to_string(),
                    "test".to_string(),
                    "build".to_string(),
                    "publish".to_string(),
                    "init".to_string(),
                    "update".to_string(),
                    "uninstall".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-g".to_string()),
                        long: Some("--global".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Global installation".to_string(),
                    },
                    CommandOption {
                        short: Some("-D".to_string()),
                        long: Some("--save-dev".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Save as development dependency".to_string(),
                    },
                ],
                takes_files: false,
                arg_types: vec![ArgType::String],
            },
        );

        // Add more common commands...
        self.add_ls_command();
        self.add_cd_command();
        self.add_mkdir_command();
    }

    /// Add ls command
    fn add_ls_command(&mut self) {
        self.command_db.insert(
            "ls".to_string(),
            CommandMeta {
                name: "ls".to_string(),
                subcommands: vec![],
                options: vec![
                    CommandOption {
                        short: Some("-l".to_string()),
                        long: Some("--long".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Long format display".to_string(),
                    },
                    CommandOption {
                        short: Some("-a".to_string()),
                        long: Some("--all".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Show all files".to_string(),
                    },
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--human-readable".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Human-readable format".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::Directory, ArgType::File],
            },
        );
    }

    /// Add cd command
    fn add_cd_command(&mut self) {
        self.command_db.insert(
            "cd".to_string(),
            CommandMeta {
                name: "cd".to_string(),
                subcommands: vec![],
                options: vec![],
                takes_files: true,
                arg_types: vec![ArgType::Directory],
            },
        );
    }

    /// Add mkdir command
    fn add_mkdir_command(&mut self) {
        self.command_db.insert(
            "mkdir".to_string(),
            CommandMeta {
                name: "mkdir".to_string(),
                subcommands: vec![],
                options: vec![
                    CommandOption {
                        short: Some("-p".to_string()),
                        long: Some("--parents".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "Create parent directories".to_string(),
                    },
                    CommandOption {
                        short: Some("-m".to_string()),
                        long: Some("--mode".to_string()),
                        takes_value: true,
                        value_type: Some(ArgType::String),
                        description: "Set permission mode".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::Directory],
            },
        );
    }

    /// Get command metadata
    pub fn get_command_meta(&self, command: &str) -> Option<&CommandMeta> {
        self.command_db.get(command)
    }
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CompletionPosition, ContextAnalyzer};

    #[test]
    fn cd_trailing_space_enters_argument_position() {
        let analyzer = ContextAnalyzer::new();
        let analysis = analyzer.analyze("cd ", 3);

        assert_eq!(
            analysis.position,
            CompletionPosition::Argument {
                command: "cd".to_string(),
                position: 0
            }
        );
        assert_eq!(analysis.current_word, "");
        assert_eq!(analysis.tokens.len(), 1);
        assert_eq!(analysis.tokens[0].text("cd "), "cd");
    }

    #[test]
    fn cd_first_argument_word_is_detected() {
        let analyzer = ContextAnalyzer::new();
        let analysis = analyzer.analyze("cd d", 4);

        assert_eq!(
            analysis.position,
            CompletionPosition::Argument {
                command: "cd".to_string(),
                position: 0
            }
        );
        assert_eq!(analysis.current_word, "d");
        assert_eq!(analysis.tokens.len(), 2);
        assert_eq!(analysis.tokens[0].text("cd d"), "cd");
        assert_eq!(analysis.tokens[1].text("cd d"), "d");
    }
}
