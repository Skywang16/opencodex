use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// File language type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
}

impl Language {
    /// Infer language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "c" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "c++" => Some(Language::Cpp),
            "cs" => Some(Language::CSharp),
            "rb" => Some(Language::Ruby),
            "php" | "phtml" | "php3" | "php4" | "php5" | "phps" | "phar" => Some(Language::Php),
            "swift" => Some(Language::Swift),
            "kt" | "kts" => Some(Language::Kotlin),
            _ => None,
        }
    }

    /// Infer language from file path
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// Text fragment location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

impl Span {
    /// Create a new Span
    pub fn new(byte_start: usize, byte_end: usize, line_start: usize, line_end: usize) -> Self {
        Self {
            byte_start,
            byte_end,
            line_start,
            line_end,
        }
    }

    /// Validate Span validity
    pub fn validate(&self) -> crate::vector_db::core::Result<()> {
        if self.byte_start > self.byte_end {
            return Err(crate::vector_db::core::VectorDbError::InvalidSpan(format!(
                "Invalid byte range: {} > {}",
                self.byte_start, self.byte_end
            )));
        }
        if self.line_start > self.line_end {
            return Err(crate::vector_db::core::VectorDbError::InvalidSpan(format!(
                "Invalid line range: {} > {}",
                self.line_start, self.line_end
            )));
        }
        Ok(())
    }
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub hash: String,
    pub last_modified: u64,
    pub size: u64,
    pub language: Option<Language>,
}

impl FileMetadata {
    /// Create file metadata
    pub fn new(path: PathBuf, hash: String, last_modified: u64, size: u64) -> Self {
        let language = Language::from_path(&path);
        Self {
            path,
            hash,
            last_modified,
            size,
            language,
        }
    }
}

/// Vector chunk ID
pub type ChunkId = Uuid;

/// Chunk type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    Function,
    Class,
    Method,
    Struct,
    Enum,
    Generic,
}

impl std::fmt::Display for ChunkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkType::Function => write!(f, "function"),
            ChunkType::Class => write!(f, "class"),
            ChunkType::Method => write!(f, "method"),
            ChunkType::Struct => write!(f, "struct"),
            ChunkType::Enum => write!(f, "enum"),
            ChunkType::Generic => write!(f, "generic"),
        }
    }
}

/// Stride information - used to record information after splitting large chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrideInfo {
    /// Unique ID of the original chunk
    pub original_chunk_id: String,
    /// Index of current stride (starting from 0)
    pub stride_index: usize,
    /// Total number of strides
    pub total_strides: usize,
    /// Start byte offset overlapping with previous stride
    pub overlap_start: usize,
    /// End byte offset overlapping with next stride
    pub overlap_end: usize,
}

/// Chunk configuration
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum number of tokens per chunk
    pub max_tokens: usize,
    /// Number of tokens for stride overlap
    pub stride_overlap: usize,
    /// Whether to enable striding (large chunk splitting)
    pub enable_striding: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8192,     // Default uses large model limit
            stride_overlap: 1024, // 12.5% overlap
            enable_striding: true,
        }
    }
}

impl ChunkConfig {
    /// Create configuration based on model name
    pub fn for_model(model_name: Option<&str>) -> Self {
        let (max_tokens, stride_overlap) =
            crate::vector_db::chunking::TokenEstimator::get_model_chunk_config(model_name);
        Self {
            max_tokens,
            stride_overlap,
            enable_striding: true,
        }
    }
}

/// Text chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub file_path: PathBuf,
    pub span: Span,
    pub content: String,
    pub chunk_type: ChunkType,
    /// Stride information (if this chunk was split from a large chunk)
    pub stride_info: Option<StrideInfo>,
}

impl Chunk {
    /// Create a new text chunk
    pub fn new(file_path: PathBuf, span: Span, content: String, chunk_type: ChunkType) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            span,
            content,
            chunk_type,
            stride_info: None,
        }
    }

    /// Create text chunk with stride information
    pub fn with_stride(
        file_path: PathBuf,
        span: Span,
        content: String,
        chunk_type: ChunkType,
        stride_info: StrideInfo,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            span,
            content,
            chunk_type,
            stride_info: Some(stride_info),
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub span: Span,
    pub score: f32,
    pub preview: String,
    pub language: Option<Language>,
    pub chunk_type: Option<ChunkType>,
}

impl SearchResult {
    /// Create search result
    pub fn new(
        file_path: PathBuf,
        span: Span,
        score: f32,
        preview: String,
        language: Option<Language>,
        chunk_type: Option<ChunkType>,
    ) -> Self {
        Self {
            file_path,
            span,
            score,
            preview,
            language,
            chunk_type,
        }
    }
}
