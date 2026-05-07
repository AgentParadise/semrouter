use crate::error::RouterError;
use serde::{Deserialize, Serialize};

// ── FastEmbedEmbedder ─────────────────────────────────────────────────────────

pub struct FastEmbedEmbedder {
    model: fastembed::TextEmbedding,
}

impl FastEmbedEmbedder {
    pub fn new() -> Result<Self, RouterError> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )
        .map_err(|e| RouterError::Embedding(format!("fastembed init failed: {e}")))?;
        Ok(Self { model })
    }
}

impl EmbeddingProvider for FastEmbedEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        let embeddings = self
            .model
            .embed(vec![text], None)
            .map_err(|e| RouterError::Embedding(format!("fastembed embed failed: {e}")))?;
        let mut v = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| RouterError::Embedding("Empty fastembed result".to_string()))?;
        normalize(&mut v);
        Ok(v)
    }

    fn dimension(&self) -> usize {
        384
    }
}

// ── HttpEmbedder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HttpEmbedder {
    pub endpoint: String,
    pub model: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: usize,
}

impl HttpEmbedder {
    pub fn new(endpoint: String, model: String) -> Result<Self, RouterError> {
        Ok(Self {
            endpoint,
            model,
            client: reqwest::Client::new(),
        })
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RouterError::Embedding(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(RouterError::Embedding(format!(
                "HTTP {status} from embedding service: {body}"
            )));
        }

        let resp: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| RouterError::Embedding(format!("JSON parse failed: {e}")))?;

        resp.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| RouterError::Embedding("Empty embedding response".to_string()))
    }

    pub fn dimension(&self) -> usize {
        1536
    }
}

// ── EmbeddingProvider trait ───────────────────────────────────────────────────

pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError>;
    fn dimension(&self) -> usize;
}

impl EmbeddingProvider for HttpEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(HttpEmbedder::embed(self, text))
    }

    fn dimension(&self) -> usize {
        HttpEmbedder::dimension(self)
    }
}

// ── MockEmbedder ──────────────────────────────────────────────────────────────

/// Maps keywords to fixed embedding dimensions so different semantic domains
/// occupy non-overlapping dimension ranges, enabling reliable cosine similarity.
const VOCAB: &[(&str, usize)] = &[
    // coding (dims 0-15)
    ("debug", 0),
    ("code", 1),
    ("error", 2),
    ("function", 3),
    ("test", 4),
    ("implement", 5),
    ("fix", 6),
    ("rust", 7),
    ("python", 8),
    ("javascript", 9),
    ("refactor", 10),
    ("compile", 11),
    ("variable", 12),
    ("class", 13),
    ("syntax", 14),
    ("module", 15),
    // second brain (dims 16-31)
    ("save", 16),
    ("brain", 17),
    ("note", 18),
    ("knowledge", 19),
    ("capture", 20),
    ("store", 21),
    ("idea", 22),
    ("memory", 23),
    ("archive", 24),
    ("organize", 25),
    ("file", 26),
    ("link", 27),
    ("thought", 28),
    ("insight", 29),
    ("category", 30),
    ("tag", 31),
    // research (dims 32-47)
    ("research", 32),
    ("find", 33),
    ("look", 34),
    ("search", 35),
    ("information", 36),
    ("learn", 37),
    ("understand", 38),
    ("explain", 39),
    ("study", 40),
    ("read", 41),
    ("paper", 42),
    ("article", 43),
    ("data", 44),
    ("source", 45),
    ("evidence", 46),
    ("review", 47),
    // model/task routing (dims 48-63)
    ("complex", 48),
    ("simple", 49),
    ("reasoning", 50),
    ("quick", 51),
    ("expensive", 52),
    ("cheap", 53),
    ("fast", 54),
    ("slow", 55),
    ("creative", 56),
    ("analytical", 57),
    ("generate", 58),
    ("summarize", 59),
    ("analyze", 60),
    ("strategy", 61),
    ("plan", 62),
    ("decide", 63),
];

const DIM: usize = 64;

pub struct MockEmbedder;

impl Default for MockEmbedder {
    fn default() -> Self {
        MockEmbedder
    }
}

impl MockEmbedder {
    pub fn new() -> Self {
        MockEmbedder
    }
}

impl EmbeddingProvider for MockEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let mut vec = vec![0.0f32; DIM];

        for word in &words {
            let word = word.trim_matches(|c: char| !c.is_alphanumeric());
            for &(keyword, dim) in VOCAB {
                if word == keyword || word.starts_with(keyword) {
                    vec[dim] += 1.0;
                }
            }
        }

        // Small per-text noise so identical-keyword texts can still be distinguished
        let hash = fnv_hash(text);
        for (i, slot) in vec.iter_mut().enumerate() {
            let noise = ((hash
                .wrapping_add(i as u64)
                .wrapping_mul(6364136223846793005))
                >> 33) as f32
                / (u32::MAX as f32)
                * 0.05;
            *slot += noise;
        }

        normalize(&mut vec);
        Ok(vec)
    }

    fn dimension(&self) -> usize {
        DIM
    }
}

fn fnv_hash(s: &str) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(1099511628211);
        hash ^= byte as u64;
    }
    hash
}

// ── Math helpers ──────────────────────────────────────────────────────────────

pub fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Embedding dimension mismatch");
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_embedder_is_deterministic() {
        let e = MockEmbedder::new();
        let v1 = e.embed("Help me debug this Python error").unwrap();
        let v2 = e.embed("Help me debug this Python error").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn mock_embedder_produces_normalized_vector() {
        let e = MockEmbedder::new();
        let v = e.embed("Save this idea to my second brain").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "Expected unit vector, got norm={norm}"
        );
    }

    #[test]
    fn coding_texts_are_more_similar_to_each_other() {
        let e = MockEmbedder::new();
        let coding1 = e.embed("Help me debug this Python error").unwrap();
        let coding2 = e.embed("Fix this Rust compile error in my code").unwrap();
        let brain = e.embed("Save this idea to my second brain").unwrap();

        let sim_same = cosine_similarity(&coding1, &coding2);
        let sim_diff = cosine_similarity(&coding1, &brain);
        assert!(
            sim_same > sim_diff,
            "same-domain sim={sim_same} should > cross-domain sim={sim_diff}"
        );
    }
}
