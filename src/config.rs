use crate::error::RouterError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level router configuration, deserialized from `router.toml`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouterConfig {
    /// Core routing parameters (thresholds, model name, top-k, etc.).
    pub router: RouterSection,
    /// File paths for the route corpus, hard negatives, logs, and index.
    #[serde(default)]
    pub storage: StorageSection,
    /// Optional HTTP-embedder configuration (endpoint, model, provider name).
    #[serde(default)]
    pub embedding: EmbeddingSection,
}

/// Configuration for an optional HTTP embedding backend.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct EmbeddingSection {
    /// HTTP provider name (e.g. "openai", "ollama"). Informational only.
    pub provider: Option<String>,
    /// Full URL to the /v1/embeddings endpoint. Falls back to OPENAI_BASE_URL env var.
    pub endpoint: Option<String>,
    /// Embedding model name (e.g. "text-embedding-3-small").
    pub model: Option<String>,
}

/// Core routing knobs: thresholds, model name, similarity metric, and penalty.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouterSection {
    /// Human-readable name for this router instance.
    pub name: String,
    /// Semantic version of the router configuration.
    pub version: String,
    /// Embedding model identifier (e.g. `"fastembed/all-MiniLM-L6-v2"`).
    pub embedding_model: String,
    /// Expected dimensionality of the embedding vectors.
    pub vector_dimension: usize,
    /// Similarity metric to use; currently only `"cosine"` is supported.
    #[serde(default = "default_similarity")]
    pub similarity: String,
    /// Number of top examples per route used to compute the aggregate score.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Minimum similarity score for a route to be accepted.
    #[serde(default = "default_minimum_score")]
    pub minimum_score: f32,
    /// Minimum score margin between first and second candidate for acceptance.
    #[serde(default = "default_minimum_margin")]
    pub minimum_margin: f32,
    /// Route name returned when no route clears the thresholds.
    #[serde(default = "default_fallback_route")]
    pub fallback_route: String,
    /// Score penalty applied when a query is similar to a hard negative.
    #[serde(default = "default_hard_negative_penalty")]
    pub hard_negative_penalty: f32,
}

/// File paths used by the router for persistence and logging.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageSection {
    /// Path to the JSONL file containing route examples.
    pub routes_file: String,
    /// Path to the JSONL file containing hard-negative examples.
    pub hard_negatives_file: String,
    /// Path to the JSONL file where user feedback is appended.
    pub feedback_file: String,
    /// Path to the JSONL file where routing decisions are logged.
    pub decision_log_file: String,
    /// Directory for the binary embedding index.
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
    /// Load and parse a `router.toml` file from the given path.
    pub fn load(path: &Path) -> Result<Self, RouterError> {
        let content = std::fs::read_to_string(path)?;
        let config: RouterConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Return a sensible default config suitable for testing with `MockEmbedder`.
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
