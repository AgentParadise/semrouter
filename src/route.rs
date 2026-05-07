use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouteExample {
    pub id: String,
    pub route: String,
    pub text: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub risk: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct EmbeddedExample {
    pub example: RouteExample,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HardNegative {
    pub id: String,
    pub route: String,
    pub text: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddedHardNegative {
    pub hn: HardNegative,
    pub embedding: Vec<f32>,
}
