use crate::embedding::{normalize, EmbeddingProvider};
use crate::error::RouterError;
use crate::route::{EmbeddedExample, EmbeddedHardNegative, HardNegative, RouteExample};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub fn load_examples(path: &Path) -> Result<Vec<RouteExample>, RouterError> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut examples = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let example: RouteExample = serde_json::from_str(line).map_err(|e| {
            RouterError::Parse(format!("routes.jsonl line {}: {}", line_num + 1, e))
        })?;
        examples.push(example);
    }

    Ok(examples)
}

pub fn embed_examples(
    examples: Vec<RouteExample>,
    embedder: &dyn EmbeddingProvider,
) -> Result<Vec<EmbeddedExample>, RouterError> {
    examples
        .into_iter()
        .map(|ex| {
            let mut embedding = embedder.embed(&ex.text)?;
            normalize(&mut embedding);
            Ok(EmbeddedExample {
                example: ex,
                embedding,
            })
        })
        .collect()
}

pub fn load_hard_negatives(path: &Path) -> Result<Vec<HardNegative>, RouterError> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut hns = Vec::new();
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let hn: HardNegative = serde_json::from_str(line).map_err(|e| {
            RouterError::Parse(format!("hard_negatives.jsonl line {}: {}", line_num + 1, e))
        })?;
        hns.push(hn);
    }
    Ok(hns)
}

pub fn embed_hard_negatives(
    hns: Vec<HardNegative>,
    embedder: &dyn EmbeddingProvider,
) -> Result<Vec<EmbeddedHardNegative>, RouterError> {
    hns.into_iter()
        .map(|hn| {
            let mut embedding = embedder.embed(&hn.text)?;
            normalize(&mut embedding);
            Ok(EmbeddedHardNegative { hn, embedding })
        })
        .collect()
}

pub fn save_binary_index(
    examples: &[EmbeddedExample],
    index_dir: &Path,
) -> Result<(), RouterError> {
    std::fs::create_dir_all(index_dir)?;

    // Save embeddings as binary f32 array
    let embeddings_path = index_dir.join("embeddings.f32");
    let mut file = File::create(&embeddings_path)?;
    for ex in examples {
        for &val in &ex.embedding {
            file.write_all(&val.to_le_bytes())?;
        }
    }

    // Save examples metadata as JSON
    let examples_path = index_dir.join("examples.json");
    let examples_json: Vec<_> = examples.iter().map(|ex| &ex.example).collect();
    let json_content = serde_json::to_string_pretty(&examples_json)?;
    std::fs::write(&examples_path, json_content)?;

    // Save manifest
    let manifest_path = index_dir.join("manifest.json");
    let manifest = BinaryIndexManifest {
        version: "1.0".to_string(),
        example_count: examples.len(),
        vector_dimension: examples.first().map_or(0, |ex| ex.embedding.len()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryIndexManifest {
    pub version: String,
    pub example_count: usize,
    pub vector_dimension: usize,
    pub created_at: String,
}

pub fn load_binary_index(index_dir: &Path) -> Result<Vec<EmbeddedExample>, RouterError> {
    let embeddings_path = index_dir.join("embeddings.f32");
    let examples_path = index_dir.join("examples.json");
    let manifest_path = index_dir.join("manifest.json");

    if !embeddings_path.exists() || !examples_path.exists() || !manifest_path.exists() {
        return Err(RouterError::Config("Binary index incomplete".to_string()));
    }

    // Load manifest
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest: BinaryIndexManifest = serde_json::from_str(&manifest_content)?;

    // Load examples
    let examples_content = std::fs::read_to_string(&examples_path)?;
    let raw_examples: Vec<RouteExample> = serde_json::from_str(&examples_content)?;

    // Load embeddings
    let embeddings_data = std::fs::read(&embeddings_path)?;
    if embeddings_data.len() != raw_examples.len() * manifest.vector_dimension * 4 {
        return Err(RouterError::Config(
            "Embedding data size mismatch".to_string(),
        ));
    }

    let mut embeddings = Vec::new();
    let mut offset = 0;
    for ex in raw_examples {
        let mut embedding = Vec::new();
        for _ in 0..manifest.vector_dimension {
            let bytes = &embeddings_data[offset..offset + 4];
            embedding.push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
            offset += 4;
        }
        embeddings.push(EmbeddedExample {
            example: ex,
            embedding,
        });
    }

    Ok(embeddings)
}
