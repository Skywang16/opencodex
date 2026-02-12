use crate::vector_db::core::{Result, SearchResult};
use std::collections::HashMap;

/// Hybrid search engine
/// Combines semantic search and keyword matching, uses Reciprocal Rank Fusion (RRF) algorithm to merge results
pub struct HybridSearchEngine;

impl HybridSearchEngine {
    /// Execute Hybrid search
    ///
    /// # Parameters
    /// - `query`: Query string
    /// - `semantic_results`: Semantic search results
    /// - `keyword_results`: Keyword search results
    /// - `semantic_weight`: Semantic search weight (0.0-1.0)
    /// - `keyword_weight`: Keyword search weight (0.0-1.0)
    /// - `k`: RRF constant, typically 60
    pub fn hybrid_search(
        _query: &str,
        semantic_results: Vec<SearchResult>,
        keyword_results: Vec<SearchResult>,
        semantic_weight: f32,
        keyword_weight: f32,
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        // Use RRF (Reciprocal Rank Fusion) algorithm to merge results
        let total_capacity = semantic_results.len() + keyword_results.len();
        let mut scores: HashMap<String, f32> = HashMap::with_capacity(total_capacity);
        let mut results_map: HashMap<String, SearchResult> = HashMap::with_capacity(total_capacity);

        // Process semantic search results
        for (rank, result) in semantic_results.iter().enumerate() {
            let key = result.unique_key();
            let rrf_score = semantic_weight / (k as f32 + (rank + 1) as f32);
            *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
            results_map.insert(key, result.clone());
        }

        // Process keyword search results
        for (rank, result) in keyword_results.iter().enumerate() {
            let key = result.unique_key();
            let rrf_score = keyword_weight / (k as f32 + (rank + 1) as f32);
            *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
            results_map.insert(key, result.clone());
        }

        // Sort by merged score
        let mut final_results: Vec<_> = scores
            .into_iter()
            .filter_map(|(key, score)| {
                results_map.get(&key).map(|result| {
                    let mut result = result.clone();
                    result.score = score; // Update to merged score
                    result
                })
            })
            .collect();

        final_results.sort_by(|a, b| b.score.total_cmp(&a.score));

        Ok(final_results)
    }

    /// Execute simple keyword search
    /// Search for content containing query keywords in all chunks
    pub fn keyword_search(query: &str, all_results: &[SearchResult]) -> Result<Vec<SearchResult>> {
        // Tokenize query
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();

        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_results = Vec::with_capacity(all_results.len());

        for result in all_results {
            let content_lower = result.preview.to_lowercase();
            let mut match_count = 0;
            let mut total_positions = 0;

            // Count matches for each keyword
            for keyword in &keywords {
                if keyword.len() < 2 {
                    continue; // Skip words that are too short
                }

                let matches: Vec<_> = content_lower.match_indices(keyword).collect();
                match_count += matches.len();

                // Record match positions to calculate relevance
                for (pos, _) in matches {
                    total_positions += pos;
                }
            }

            if match_count > 0 {
                // Calculate keyword match score
                // Consider: match count, matched keyword count, match position (earlier is better)
                let keyword_coverage = match_count as f32 / keywords.len() as f32;
                let position_score = if total_positions > 0 {
                    1.0 / (1.0 + (total_positions as f32 / content_lower.len() as f32))
                } else {
                    1.0
                };

                let score = keyword_coverage * 0.7 + position_score * 0.3;

                let mut result = result.clone();
                result.score = score;
                scored_results.push(result);
            }
        }

        // Sort by score
        scored_results.sort_by(|a, b| b.score.total_cmp(&a.score));

        Ok(scored_results)
    }
}

impl SearchResult {
    /// Generate unique key for result deduplication
    fn unique_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.file_path.display(),
            self.span.line_start,
            self.span.line_end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::core::{ChunkType, Span};
    use std::path::PathBuf;

    fn create_test_result(
        file: &str,
        line_start: usize,
        line_end: usize,
        preview: &str,
        score: f32,
    ) -> SearchResult {
        SearchResult {
            file_path: PathBuf::from(file),
            span: Span::new(0, 100, line_start, line_end),
            score,
            preview: preview.to_string(),
            language: None,
            chunk_type: Some(ChunkType::Function),
        }
    }

    #[test]
    fn test_hybrid_search() {
        let semantic_results = vec![
            create_test_result("file1.rs", 1, 10, "semantic match", 0.9),
            create_test_result("file2.rs", 5, 15, "another semantic", 0.8),
        ];

        let keyword_results = vec![
            create_test_result("file1.rs", 1, 10, "keyword match", 0.7),
            create_test_result("file3.rs", 20, 30, "keyword only", 0.6),
        ];

        let results = HybridSearchEngine::hybrid_search(
            "test query",
            semantic_results,
            keyword_results,
            0.7,
            0.3,
            60,
        )
        .unwrap();

        assert!(!results.is_empty());
        // file1.rs should be ranked first because it appears in both semantic and keyword results
        assert_eq!(results[0].file_path, PathBuf::from("file1.rs"));
    }

    #[test]
    fn test_keyword_search() {
        let all_results = vec![
            create_test_result("file1.rs", 1, 10, "This is a test function", 0.0),
            create_test_result("file2.rs", 5, 15, "Another test here", 0.0),
            create_test_result("file3.rs", 20, 30, "No match", 0.0),
        ];

        let results = HybridSearchEngine::keyword_search("test", &all_results).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].score > 0.0);
    }
}
