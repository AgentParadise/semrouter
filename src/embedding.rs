use crate::error::RouterError;

// ── FastEmbedEmbedder ─────────────────────────────────────────────────────────

/// Local ONNX embedder backed by `fastembed` (all-MiniLM-L6-v2, 384-dim).
#[cfg(feature = "fastembed")]
pub struct FastEmbedEmbedder {
    model: fastembed::TextEmbedding,
}

#[cfg(feature = "fastembed")]
impl FastEmbedEmbedder {
    /// Initialize the fastembed model, downloading the ONNX weights if needed.
    pub fn new() -> Result<Self, RouterError> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )
        .map_err(|e| RouterError::Embedding(format!("fastembed init failed: {e}")))?;
        Ok(Self { model })
    }
}

#[cfg(feature = "fastembed")]
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

// ── EmbeddingProvider trait ───────────────────────────────────────────────────

/// Trait for types that can embed text into a fixed-dimensional float vector.
pub trait EmbeddingProvider {
    /// Embed a single text string and return a float vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError>;
    /// Return the fixed dimension of the vectors produced by this embedder.
    fn dimension(&self) -> usize;
}

// ── Math helpers ──────────────────────────────────────────────────────────────

/// Normalize a vector in-place to unit length; no-op if the norm is near zero.
pub fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Compute the dot product of two pre-normalized vectors (equivalent to cosine similarity).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Embedding dimension mismatch");
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

