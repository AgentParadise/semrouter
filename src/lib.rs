pub mod config;
pub mod decision;
pub mod embedding;
pub mod error;
pub mod eval;
pub mod experiment;
pub mod route;
pub mod scoring;
pub mod storage;
pub mod testing;
pub(crate) mod time_util;

use config::RouterConfig;
use decision::{make_decision, RouteDecision};
pub use embedding::FastEmbedEmbedder;
use embedding::{normalize, EmbeddingProvider};
use error::RouterError;
use route::{EmbeddedExample, EmbeddedHardNegative};
use scoring::score_routes;
use std::path::{Path, PathBuf};
use storage::{embed_examples, embed_hard_negatives, load_examples, load_hard_negatives};

pub struct SemanticRouter {
    config: RouterConfig,
    examples: Vec<EmbeddedExample>,
    hard_negatives: Vec<EmbeddedHardNegative>,
    embedder: Box<dyn EmbeddingProvider>,
}

impl SemanticRouter {
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

    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

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
