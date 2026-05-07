use crate::error::RouterError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouterConfig {
    pub router: RouterSection,
    #[serde(default)]
    pub storage: StorageSection,
    #[serde(default)]
    pub embedding: EmbeddingSection,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct EmbeddingSection {
    /// HTTP provider name (e.g. "openai", "ollama"). Informational only.
    pub provider: Option<String>,
    /// Full URL to the /v1/embeddings endpoint. Falls back to OPENAI_BASE_URL env var.
    pub endpoint: Option<String>,
    /// Embedding model name (e.g. "text-embedding-3-small").
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouterSection {
    pub name: String,
    pub version: String,
    pub embedding_model: String,
    pub vector_dimension: usize,
    #[serde(default = "default_similarity")]
    pub similarity: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_minimum_score")]
    pub minimum_score: f32,
    #[serde(default = "default_minimum_margin")]
    pub minimum_margin: f32,
    #[serde(default = "default_fallback_route")]
    pub fallback_route: String,
    #[serde(default = "default_hard_negative_penalty")]
    pub hard_negative_penalty: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageSection {
    pub routes_file: String,
    pub hard_negatives_file: String,
    pub feedback_file: String,
    pub decision_log_file: String,
    pub index_dir: String,
}

fn default_hard_negative_penalty() -> f32 {
    0.1
}
fn default_similarity() -> String {
    "cosine".to_string()
}
fn default_top_k() -> usize {
    3
}
fn default_minimum_score() -> f32 {
    0.72
}
fn default_minimum_margin() -> f32 {
    0.06
}
fn default_fallback_route() -> String {
    "needs_review".to_string()
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            routes_file: "routes.jsonl".to_string(),
            hard_negatives_file: "hard_negatives.jsonl".to_string(),
            feedback_file: "feedback.jsonl".to_string(),
            decision_log_file: "decisions.jsonl".to_string(),
            index_dir: "index".to_string(),
        }
    }
}

impl RouterConfig {
    pub fn load(path: &Path) -> Result<Self, RouterError> {
        let content = std::fs::read_to_string(path)?;
        let config: RouterConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        RouterConfig {
            router: RouterSection {
                name: "semrouter".to_string(),
                version: "0.1.0".to_string(),
                embedding_model: "mock".to_string(),
                vector_dimension: 384,
                similarity: "cosine".to_string(),
                top_k: 3,
                minimum_score: 0.72,
                minimum_margin: 0.06,
                fallback_route: "needs_review".to_string(),
                hard_negative_penalty: 0.1,
            },
            storage: StorageSection::default(),
            embedding: EmbeddingSection::default(),
        }
    }
}
