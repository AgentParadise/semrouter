use serde::{Deserialize, Serialize};

/// Risk classification of a route; preserved for future policy modules.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Low risk (default).
    #[default]
    Low,
    /// Medium risk.
    Medium,
    /// High risk.
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
        }
    }
}

/// A labeled routing example loaded from `routes.jsonl`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouteExample {
    /// Unique identifier for this example.
    pub id: String,
    /// The route this example belongs to.
    pub route: String,
    /// The example text used for embedding.
    pub text: String,
    /// Optional tags for filtering or grouping examples.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Risk classification of the route.
    #[serde(default)]
    pub risk: RiskLevel,
}

/// A `RouteExample` paired with its pre-computed embedding vector.
#[derive(Debug, Clone)]
pub struct EmbeddedExample {
    /// The original route example.
    pub example: RouteExample,
    /// The normalized embedding vector for this example.
    pub embedding: Vec<f32>,
}

/// A hard-negative example: text that should *not* match a given route.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HardNegative {
    /// Unique identifier for this hard negative.
    pub id: String,
    /// The route that this example should not match.
    pub route: String,
    /// The text of the hard negative.
    pub text: String,
    /// Human-readable explanation of why this is a hard negative for the route.
    #[serde(default)]
    pub reason: String,
}

/// A `HardNegative` paired with its pre-computed embedding vector.
#[derive(Debug, Clone)]
pub struct EmbeddedHardNegative {
    /// The original hard-negative record.
    pub hn: HardNegative,
    /// The normalized embedding vector for this hard negative.
    pub embedding: Vec<f32>,
}
