//! Flat nearest-neighbor search over exemplar embeddings.
//!
//! No vector database, no tree, no sort: at the scale of a few thousand
//! short exemplar vectors, two linear passes (best label, then best score
//! among the other labels — see `top_match`) is still sub-millisecond and
//! far simpler than anything else. Vectors are stored contiguously
//! (row-major, not `Vec<Vec<f32>>`) and are expected to already be
//! L2-normalized (see `EmbeddingEngine::embed`), so cosine similarity is a
//! plain dot product.

use super::types::Exemplar;

pub struct NearestNeighborIndex {
    exemplars: Vec<Exemplar>,
    vectors: Vec<f32>,
    dim: usize,
}

/// The winning exemplar's label/text plus how far it beat the runner-up.
/// A small margin means two different labels were nearly tied — ambiguous.
pub struct TopMatch {
    pub label: String,
    pub text: String,
    pub score: f32,
    pub margin: f32,
}

impl NearestNeighborIndex {
    /// `vectors[i]` must be the embedding of `exemplars[i].text`, already normalized.
    pub fn new(exemplars: Vec<Exemplar>, vectors: Vec<Vec<f32>>) -> Self {
        assert_eq!(
            exemplars.len(),
            vectors.len(),
            "exemplars and vectors must line up 1:1"
        );
        let dim = vectors.first().map(Vec::len).unwrap_or(0);
        let mut flat = Vec::with_capacity(exemplars.len() * dim);
        for v in &vectors {
            debug_assert_eq!(v.len(), dim, "all exemplar vectors must share one dimension");
            flat.extend_from_slice(v);
        }
        Self { exemplars, vectors: flat, dim }
    }

    pub fn is_empty(&self) -> bool {
        self.exemplars.is_empty()
    }

    /// `query` must already be L2-normalized.
    ///
    /// Margin is the winner's score minus the best score among exemplars
    /// with a *different* label — not the literal runner-up vector. Pools
    /// hold many exemplars per label on purpose, so the literal 2nd-best
    /// is very often another phrasing of the same label; that closeness
    /// isn't ambiguity and must not trip the margin threshold in `decide`.
    pub fn top_match(&self, query: &[f32]) -> Option<TopMatch> {
        if self.exemplars.is_empty() {
            return None;
        }

        let scores: Vec<f32> = (0..self.exemplars.len())
            .map(|i| dot(&self.vectors[i * self.dim..(i + 1) * self.dim], query))
            .collect();

        let best_idx = (0..scores.len())
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .expect("checked non-empty above");
        let best = &self.exemplars[best_idx];
        let best_score = scores[best_idx];

        let runner_up_score = self
            .exemplars
            .iter()
            .zip(scores.iter())
            .filter(|(ex, _)| ex.label != best.label)
            .map(|(_, &s)| s)
            .fold(f32::NEG_INFINITY, f32::max);

        // No exemplar with a different label exists in this pool: nothing to be
        // ambiguous against, so treat it as maximally unambiguous.
        let margin = if runner_up_score.is_finite() {
            best_score - runner_up_score
        } else {
            best_score
        };

        Some(TopMatch {
            label: best.label.clone(),
            text: best.text.clone(),
            score: best_score,
            margin,
        })
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex(text: &str, label: &str) -> Exemplar {
        Exemplar { text: text.to_string(), label: label.to_string() }
    }

    #[test]
    fn empty_index_has_no_match() {
        let idx = NearestNeighborIndex::new(vec![], vec![]);
        assert!(idx.is_empty());
        assert!(idx.top_match(&[1.0, 0.0]).is_none());
    }

    #[test]
    fn single_exemplar_gets_full_margin() {
        let idx = NearestNeighborIndex::new(vec![ex("a", "set")], vec![vec![1.0, 0.0]]);
        let top = idx.top_match(&[1.0, 0.0]).unwrap();
        assert_eq!(top.label, "set");
        assert!((top.score - 1.0).abs() < 1e-6);
        assert!((top.margin - 1.0).abs() < 1e-6);
    }

    #[test]
    fn picks_closest_and_reports_margin() {
        let idx = NearestNeighborIndex::new(
            vec![ex("close", "set"), ex("far", "list")],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        );
        let top = idx.top_match(&[0.9, 0.1]).unwrap();
        assert_eq!(top.label, "set");
        assert!(top.margin > 0.0);
    }

    /// Two same-label exemplars that both happen to be close to the query
    /// must NOT read as ambiguous — there's no rival label to be ambiguous
    /// against. This is the bug the two-pass (best-label vs best-other-label)
    /// approach exists to avoid.
    #[test]
    fn near_tie_within_same_label_gets_full_margin() {
        let idx = NearestNeighborIndex::new(
            vec![ex("call mom", "set"), ex("call dad", "set"), ex("far away", "list")],
            vec![vec![1.0, 0.0], vec![0.99, 0.14107], vec![0.0, 1.0]],
        );
        let top = idx.top_match(&[1.0, 0.0]).unwrap();
        assert_eq!(top.label, "set");
        assert!(top.margin > 0.9, "expected a wide margin against the 'list' rival, got {}", top.margin);
    }

    #[test]
    fn near_tie_gives_small_margin() {
        let idx = NearestNeighborIndex::new(
            vec![ex("a", "set"), ex("b", "list")],
            vec![vec![1.0, 0.0], vec![0.99, 0.14107]],
        );
        let top = idx.top_match(&[1.0, 0.0]).unwrap();
        assert_eq!(top.label, "set");
        assert!(top.margin < 0.05, "expected a tight margin, got {}", top.margin);
    }
}
