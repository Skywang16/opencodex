use lsp_types::{
    Diagnostic, DocumentSymbol, Hover, Location, MarkedString, SymbolInformation, SymbolKind,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspServerId {
    Deno,
    Typescript,
    Vue,
    RustAnalyzer,
    Pyright,
    Gopls,
}

impl LspServerId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deno => "deno",
            Self::Typescript => "typescript",
            Self::Vue => "vue",
            Self::RustAnalyzer => "rust_analyzer",
            Self::Pyright => "pyright",
            Self::Gopls => "gopls",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspServerStatus {
    pub server_id: String,
    pub root: String,
    pub command: String,
    pub initialized: bool,
    pub connected: bool,
    pub open_documents: usize,
    pub diagnostics_files: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspLocation {
    pub path: String,
    pub range: LspRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDocumentSymbol {
    pub name: String,
    pub kind: u32,
    pub detail: Option<String>,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub children: Vec<LspDocumentSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspWorkspaceSymbol {
    pub name: String,
    pub kind: u32,
    pub container_name: Option<String>,
    pub location: Option<LspLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspHoverResult {
    pub contents: String,
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspFileDiagnostics {
    pub path: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct ResolvedServerConfig {
    pub server_id: LspServerId,
    pub root: PathBuf,
    pub command: String,
    pub args: Vec<String>,
    pub initialization_options: Option<serde_json::Value>,
}

impl From<lsp_types::Position> for LspPosition {
    fn from(value: lsp_types::Position) -> Self {
        Self {
            line: value.line,
            character: value.character,
        }
    }
}

impl From<lsp_types::Range> for LspRange {
    fn from(value: lsp_types::Range) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

impl From<Location> for LspLocation {
    fn from(value: Location) -> Self {
        let path = uri_to_path_string(&value.uri);
        Self {
            path,
            range: value.range.into(),
        }
    }
}

impl From<DocumentSymbol> for LspDocumentSymbol {
    fn from(value: DocumentSymbol) -> Self {
        Self {
            name: value.name,
            kind: symbol_kind_code(value.kind),
            detail: value.detail,
            range: value.range.into(),
            selection_range: value.selection_range.into(),
            children: value
                .children
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<SymbolInformation> for LspWorkspaceSymbol {
    fn from(value: SymbolInformation) -> Self {
        Self {
            name: value.name,
            kind: symbol_kind_code(value.kind),
            container_name: value.container_name,
            location: Some(value.location.into()),
        }
    }
}

impl From<Hover> for LspHoverResult {
    fn from(value: Hover) -> Self {
        use lsp_types::HoverContents;

        let contents = match value.contents {
            HoverContents::Scalar(marked) => marked_string_to_text(marked),
            HoverContents::Array(items) => items
                .into_iter()
                .map(marked_string_to_text)
                .collect::<Vec<_>>()
                .join("\n\n"),
            HoverContents::Markup(markup) => markup.value,
        };

        Self {
            contents,
            range: value.range.map(Into::into),
        }
    }
}

fn marked_string_to_text(marked: MarkedString) -> String {
    match marked {
        MarkedString::String(text) => text,
        MarkedString::LanguageString(block) => {
            format!("```{}\n{}\n```", block.language, block.value)
        }
    }
}

fn symbol_kind_code(kind: SymbolKind) -> u32 {
    match serde_json::to_value(kind) {
        Ok(value) => match value.as_u64() {
            Some(code) => code as u32,
            None => {
                tracing::warn!("Serialized SymbolKind was not an integer: {}", value);
                0
            }
        },
        Err(err) => {
            tracing::warn!("Failed to serialize SymbolKind: {}", err);
            0
        }
    }
}

fn uri_to_path_string(uri: &lsp_types::Uri) -> String {
    match url::Url::parse(uri.as_str()) {
        Ok(url) => match url.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => {
                tracing::warn!("LSP URI is not a valid file path: {}", uri.as_str());
                uri.as_str().to_string()
            }
        },
        Err(err) => {
            tracing::warn!("Failed to parse LSP URI '{}': {}", uri.as_str(), err);
            uri.as_str().to_string()
        }
    }
}
