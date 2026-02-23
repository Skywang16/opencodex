//! Token estimator - used to estimate the number of tokens in text

/// Simple token estimator
/// Based on empirical analysis: code ~4.2 chars/token, text ~4.8 chars/token
pub struct TokenEstimator;

impl TokenEstimator {
    /// Estimate the number of tokens in text
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let char_count = text.chars().count();

        // Detect whether content is code or natural language
        let code_indicators = Self::count_code_indicators(text);
        let total_lines = text.lines().count().max(1);
        let code_density = code_indicators as f32 / total_lines as f32;

        // Adjust ratio based on code density
        let chars_per_token = if code_density > 0.3 {
            // Code - more tokens (symbols, identifiers)
            4.2
        } else if code_density > 0.1 {
            // Mixed content
            4.4
        } else {
            // Mainly natural language
            4.8
        };

        (char_count as f32 / chars_per_token).ceil() as usize
    }

    /// Check if text exceeds token limit
    pub fn exceeds_limit(text: &str, max_tokens: usize) -> bool {
        Self::estimate_tokens(text) > max_tokens
    }

    /// Get model's chunk configuration (target_tokens, overlap_tokens)
    pub fn get_model_chunk_config(model_name: Option<&str>) -> (usize, usize) {
        let model = model_name.unwrap_or("BAAI/bge-m3");

        match model {
            // Small models - keep smaller chunks for better precision
            "BAAI/bge-small-en-v1.5" | "sentence-transformers/all-MiniLM-L6-v2" => {
                (400, 80) // 400 tokens target, 80 token overlap (~20%)
            }

            // Large context models - can use larger chunks
            "nomic-embed-text-v1" | "nomic-embed-text-v1.5" | "jina-embeddings-v2-base-code" => {
                (1024, 200) // 1024 tokens target, 200 token overlap (~20%)
            }

            // BGE variants
            "BAAI/bge-base-en-v1.5" | "BAAI/bge-large-en-v1.5" => (400, 80),

            // BGE-M3 - large context
            "BAAI/bge-m3" => (1024, 200),

            // Default to large model configuration
            _ => (1024, 200),
        }
    }

    /// Count code feature indicators
    fn count_code_indicators(text: &str) -> usize {
        let mut count = 0;

        for line in text.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            // Find code patterns
            if trimmed.contains('{') || trimmed.contains('}') {
                count += 1;
            }
            if trimmed.contains(';') && !trimmed.ends_with('.') {
                count += 1;
            }
            if trimmed.contains("fn ")
                || trimmed.contains("def ")
                || trimmed.contains("function ")
                || trimmed.contains("func ")
            {
                count += 1;
            }
            if trimmed.contains("->") || trimmed.contains("=>") || trimmed.contains("::") {
                count += 1;
            }
            if trimmed.starts_with("pub ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("public ")
            {
                count += 1;
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(TokenEstimator::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let code = r#"
fn main() {
    println!("Hello, world!");
    let x = 42;
    return x;
}
"#;
        let tokens = TokenEstimator::estimate_tokens(code);
        assert!((15..=25).contains(&tokens), "Got {tokens} tokens");
    }

    #[test]
    fn test_exceeds_limit() {
        assert!(!TokenEstimator::exceeds_limit("short text", 100));

        let long_text = "word ".repeat(200);
        assert!(TokenEstimator::exceeds_limit(&long_text, 100));
    }
}
