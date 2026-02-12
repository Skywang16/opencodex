use super::{TokenEstimator, TreeSitterChunker};
use crate::vector_db::core::{Chunk, ChunkConfig, ChunkType, Language, Result, Span, StrideInfo};
use std::path::Path;

pub struct TextChunker {
    config: ChunkConfig,
    tree_sitter_chunker: TreeSitterChunker,
}

impl TextChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            config: ChunkConfig {
                max_tokens: chunk_size,
                stride_overlap: chunk_size / 5, // 20% overlap
                enable_striding: true,
            },
            tree_sitter_chunker: TreeSitterChunker::new(chunk_size),
        }
    }

    /// Create chunker using model name
    pub fn for_model(model_name: Option<&str>) -> Self {
        let config = ChunkConfig::for_model(model_name);
        Self {
            tree_sitter_chunker: TreeSitterChunker::new(config.max_tokens),
            config,
        }
    }

    /// Create chunker with custom configuration
    pub fn with_config(config: ChunkConfig) -> Self {
        Self {
            tree_sitter_chunker: TreeSitterChunker::new(config.max_tokens),
            config,
        }
    }

    pub fn chunk(&self, content: &str, file_path: &Path) -> Result<Vec<Chunk>> {
        // Try using tree-sitter intelligent chunking
        let mut chunks = if let Some(language) = Language::from_path(file_path) {
            // Use tree-sitter for supported languages
            if matches!(
                language,
                Language::Python
                    | Language::TypeScript
                    | Language::JavaScript
                    | Language::Rust
                    | Language::Go
                    | Language::Java
                    | Language::C
                    | Language::Cpp
                    | Language::CSharp
                    | Language::Ruby
                    | Language::Php
                    | Language::Swift
            ) {
                tracing::debug!("Using tree-sitter chunking for {:?}", language);
                if let Ok(chunks) = self.tree_sitter_chunker.chunk(content, file_path, language) {
                    if !chunks.is_empty() {
                        chunks
                    } else {
                        self.chunk_generic(content, file_path)?
                    }
                } else {
                    // If tree-sitter fails, fallback to simple chunking
                    tracing::warn!("Tree-sitter failed, fallback to simple chunking");
                    self.chunk_generic(content, file_path)?
                }
            } else {
                self.chunk_generic(content, file_path)?
            }
        } else {
            self.chunk_generic(content, file_path)?
        };

        // Apply striding (split large chunks that exceed token limit)
        if self.config.enable_striding {
            chunks = self.apply_striding(chunks, file_path)?;
        }

        Ok(chunks)
    }

    /// Generic chunking (with overlap)
    fn chunk_generic(&self, content: &str, file_path: &Path) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Estimate line count based on token target
        let avg_tokens_per_line = 10.0;
        let target_lines = ((self.config.max_tokens as f32) / avg_tokens_per_line) as usize;
        let overlap_lines = ((self.config.stride_overlap as f32) / avg_tokens_per_line) as usize;

        let chunk_size = target_lines.max(5); // Minimum 5 lines
        let overlap = overlap_lines.max(1); // Minimum 1 line overlap

        // Precompute byte offsets for each line
        let mut line_byte_offsets = Vec::with_capacity(lines.len() + 1);
        line_byte_offsets.push(0);
        let mut cumulative_offset = 0;

        for line in lines.iter() {
            cumulative_offset += line.len() + 1; // +1 for newline
            line_byte_offsets.push(cumulative_offset);
        }

        let mut i = 0;
        while i < lines.len() {
            let end = (i + chunk_size).min(lines.len());
            let chunk_lines = &lines[i..end];
            let chunk_text = chunk_lines.join("\n");

            let byte_start = line_byte_offsets[i];
            let byte_end = line_byte_offsets[end];

            chunks.push(Chunk::new(
                file_path.to_path_buf(),
                Span::new(byte_start, byte_end, i + 1, end),
                chunk_text,
                ChunkType::Generic,
            ));

            // Move to next position (subtract overlap)
            i += chunk_size.saturating_sub(overlap);
            if i >= lines.len() {
                break;
            }
        }

        Ok(chunks)
    }

    /// Apply striding - split large chunks that exceed token limit
    fn apply_striding(&self, chunks: Vec<Chunk>, file_path: &Path) -> Result<Vec<Chunk>> {
        let mut result = Vec::new();

        for chunk in chunks {
            let estimated_tokens = TokenEstimator::estimate_tokens(&chunk.content);

            if estimated_tokens <= self.config.max_tokens {
                // Chunk is within limit, no need to split
                result.push(chunk);
            } else {
                // Chunk exceeds limit, apply striding
                tracing::debug!(
                    "Chunk with {} tokens exceeds limit of {}, applying striding",
                    estimated_tokens,
                    self.config.max_tokens
                );

                let strided_chunks = self.stride_large_chunk(chunk, file_path)?;
                result.extend(strided_chunks);
            }
        }

        Ok(result)
    }

    /// Split large chunk into multiple smaller chunks with overlap
    fn stride_large_chunk(&self, chunk: Chunk, file_path: &Path) -> Result<Vec<Chunk>> {
        let text = &chunk.content;

        if text.is_empty() {
            return Ok(vec![chunk]);
        }

        // Calculate stride parameters (using character count)
        let char_count = text.chars().count();
        let estimated_tokens = TokenEstimator::estimate_tokens(text);

        let chars_per_token = if estimated_tokens == 0 {
            4.5 // Default value
        } else {
            char_count as f32 / estimated_tokens as f32
        };

        let window_chars = ((self.config.max_tokens as f32 * 0.9) * chars_per_token) as usize; // 10% buffer
        let overlap_chars = (self.config.stride_overlap as f32 * chars_per_token) as usize;
        let stride_chars = window_chars.saturating_sub(overlap_chars);

        if stride_chars == 0 {
            return Ok(vec![chunk]);
        }

        // Build character to byte index mapping (handling UTF-8)
        let char_byte_indices: Vec<(usize, char)> = text.char_indices().collect();

        let mut strided_chunks = Vec::new();
        let original_chunk_id = format!("{}:{}", chunk.span.byte_start, chunk.span.byte_end);
        let mut start_char_idx = 0;
        let mut stride_index = 0;

        // Calculate total number of strides
        let total_strides = if char_count <= window_chars {
            1
        } else {
            ((char_count - overlap_chars) as f32 / stride_chars as f32).ceil() as usize
        };

        while start_char_idx < char_count {
            let end_char_idx = (start_char_idx + window_chars).min(char_count);

            // Get byte positions
            let start_byte_pos = char_byte_indices[start_char_idx].0;
            let end_byte_pos = if end_char_idx < char_count {
                char_byte_indices[end_char_idx].0
            } else {
                text.len()
            };

            let stride_text = &text[start_byte_pos..end_byte_pos];

            // Calculate overlap information
            let overlap_start = if stride_index > 0 { overlap_chars } else { 0 };
            let overlap_end = if end_char_idx < char_count {
                overlap_chars
            } else {
                0
            };

            // Calculate span
            let byte_offset_start = chunk.span.byte_start + start_byte_pos;
            let byte_offset_end = chunk.span.byte_start + end_byte_pos;

            // Estimate line numbers
            let text_before_start = &text[..start_byte_pos];
            let line_offset_start = text_before_start.lines().count().saturating_sub(1);
            let stride_lines = stride_text.lines().count();

            let stride_info = StrideInfo {
                original_chunk_id: original_chunk_id.clone(),
                stride_index,
                total_strides,
                overlap_start,
                overlap_end,
            };

            strided_chunks.push(Chunk::with_stride(
                file_path.to_path_buf(),
                Span::new(
                    byte_offset_start,
                    byte_offset_end,
                    chunk.span.line_start + line_offset_start,
                    chunk.span.line_start + line_offset_start + stride_lines.saturating_sub(1),
                ),
                stride_text.to_string(),
                chunk.chunk_type.clone(),
                stride_info,
            ));

            start_char_idx += stride_chars;
            stride_index += 1;
        }

        Ok(strided_chunks)
    }
}
