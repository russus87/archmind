//! Retrieval semantico locale via embeddings ONNX (feature `embeddings`).
//!
//! Riordina i candidati BM25 per similarità semantica con la domanda, usando
//! un piccolo modello locale (BGE-small) eseguito in ONNX da `fastembed`. Il
//! modello viene scaricato una volta e poi gira offline. Se qualcosa va storto
//! (modello non disponibile), si degrada ai primi `top` candidati lessicali.

use crate::index::Passage;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Riordina `passages` per rilevanza semantica rispetto a `query`, tenendone `top`.
pub fn rerank(query: &str, mut passages: Vec<Passage>, top: usize) -> Vec<Passage> {
    if passages.len() <= 1 {
        passages.truncate(top);
        return passages;
    }

    let mut model = match TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGESmallENV15)) {
        Ok(m) => m,
        Err(_) => {
            passages.truncate(top);
            return passages;
        }
    };

    // Primo documento = query, poi gli snippet dei candidati.
    let mut docs: Vec<String> = Vec::with_capacity(passages.len() + 1);
    docs.push(query.to_string());
    docs.extend(passages.iter().map(|p| p.snippet.clone()));

    let embeddings = match model.embed(docs, None) {
        Ok(e) => e,
        Err(_) => {
            passages.truncate(top);
            return passages;
        }
    };

    let q = &embeddings[0];
    let mut scored: Vec<(f32, Passage)> = passages
        .into_iter()
        .enumerate()
        .map(|(i, p)| (cosine(q, &embeddings[i + 1]), p))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top).map(|(_, p)| p).collect()
}

/// Similarità coseno tra due vettori.
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}
