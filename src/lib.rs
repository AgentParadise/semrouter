#![warn(missing_docs)]
//! semrouter — file-based semantic router for agent/model/workflow dispatch.
//!
//! Routes input text to a labeled route by comparing embeddings against a
//! curated set of examples. Zero default dependencies beyond serde, serde_json,
//! toml, and thiserror. Local embeddings via `fastembed` (default-on feature)
//! or bring-your-own via the [`embedding::EmbeddingProvider`] trait.
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(feature = "fastembed")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use semrouter::{SemanticRouter, config::RouterConfig, embedding::FastEmbedEmbedder};
//! use std::path::Path;
//!
//! let config = RouterConfig::load(Path::new("router.toml"))?;
//! let embedder = Box::new(FastEmbedEmbedder::new()?);
//! let router = SemanticRouter::load(config, Path::new("routes.jsonl"), embedder)?;
//! let decision = router.route("what time is it")?;
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "fastembed"))]
//! # fn main() {}
//! ```
//!
//! See the [`testing`] module for contract-testing route corpora.

/// Router configuration loaded from `router.toml`.
pub mod config;
/// Routing decision types and the decision-making logic.
pub mod decision;
/// Embedding provider trait, math helpers, and built-in embedder implementations.
pub mod embedding;
/// Error types used across the crate.
pub mod error;
/// Evaluation framework: eval cases, per-route metrics, and latency stats.
pub mod eval;
/// Experiment result type for persisting eval runs to disk.
pub mod experiment;
/// Core route data types: examples, hard negatives, and their embedded forms.
pub mod route;
/// Route scoring: cosine similarity aggregation and hard-negative penalties.
pub mod scoring;
/// JSONL loading, embedding, and binary-index persistence for route corpora.
pub mod storage;
/// Contract-testing harness for downstream consumers of semrouter.
pub mod testing;
pub(crate) mod time_util;

use config::RouterConfig;
use decision::{make_decision, RouteDecision};
#[cfg(feature = "fastembed")]
pub use embedding::FastEmbedEmbedder;
use embedding::{normalize, EmbeddingProvider};
use error::RouterError;
use route::{EmbeddedExample, EmbeddedHardNegative};
use scoring::score_routes;
use std::path::{Path, PathBuf};
use storage::{embed_examples, embed_hard_negatives, load_examples, load_hard_negatives};

/// The main router: loads a corpus of labeled examples and dispatches input text
/// to the best-matching route using embedding-based similarity.
pub struct SemanticRouter {
    config: RouterConfig,
    examples: Vec<EmbeddedExample>,
    hard_negatives: Vec<EmbeddedHardNegative>,
    embedder: Box<dyn EmbeddingProvider>,
}

impl SemanticRouter {
    /// Load a router from a config, an examples JSONL file, and an embedder.
    pub fn load(
        config: RouterConfig,
        examples_path: &Path,
        embedder: Box<dyn EmbeddingProvider>,
    ) -> Result<Self, RouterError> {
        let raw = load_examples(examples_path)?;
        if raw.is_empty() {
            return Err(RouterError::NoExamples);
        }
        let examples = embed_examples(raw, embedder.as_ref())?;

        let hn_path = PathBuf::from(&config.storage.hard_negatives_file);
        let raw_hns = load_hard_negatives(&hn_path)?;
        let hard_negatives = embed_hard_negatives(raw_hns, embedder.as_ref())?;

        Ok(Self {
            config,
            examples,
            hard_negatives,
            embedder,
        })
    }

    /// Embed input text and return a routing decision against the loaded corpus.
    pub fn route(&self, input: &str) -> Result<RouteDecision, RouterError> {
        let mut input_embedding = self.embedder.embed(input)?;
        normalize(&mut input_embedding);

        let candidates = score_routes(
            &input_embedding,
            &self.examples,
            self.config.router.top_k,
            &self.hard_negatives,
            self.config.router.hard_negative_penalty,
        );

        let decision = make_decision(input, candidates, &self.config);

        Ok(decision)
    }

    /// Return the total number of embedded examples loaded into the corpus.
    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

    /// Return a sorted, deduplicated list of route names present in the corpus.
    pub fn route_names(&self) -> Vec<String> {
        let mut routes: Vec<String> = self
            .examples
            .iter()
            .map(|e| e.example.route.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        routes.sort();
        routes
    }
}
