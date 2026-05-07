//! `BagOfWordsEmbedder` — a tiny deterministic 64-dim keyword-bag embedder
//! used **only** by integration tests in this crate.
//!
//! This is NOT a public API. It exists in `tests/common/` instead of `src/` so
//! it cannot leak into the published crate or be accidentally picked as a CLI
//! default. Its purpose is to validate routing math (top-K averaging,
//! hard-negative penalty, threshold/margin gating) against a known-shape vector
//! distribution — without paying the ~5s cold-start cost of real fastembed in
//! every fast unit test.
//!
//! Real benchmarks (accuracy, latency) MUST use `fastembed` per CLAUDE.md
//! testing principles. This embedder is for routing-logic isolation only.
//!
//! This is a verbatim lift of the old `MockEmbedder` from `src/embedding.rs`
//! (same vocab, same math, same FNV hash) renamed so existing test assertions
//! continue to pass unchanged.

use semrouter::embedding::EmbeddingProvider;
use semrouter::error::RouterError;

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

/// 64-dim deterministic keyword-bag embedder. Test-only.
///
/// Replaces the old `MockEmbedder` that used to live in `src/embedding.rs`.
/// Identical vocab and math — moved here so it is never part of the public API.
pub struct BagOfWordsEmbedder;

impl BagOfWordsEmbedder {
    pub fn new() -> Self {
        Self
    }

    pub fn dimension(&self) -> usize {
        DIM
    }
}

impl Default for BagOfWordsEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider for BagOfWordsEmbedder {
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

        semrouter::embedding::normalize(&mut vec);
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
