//! The two-threshold confidence gate: an absolute score floor plus a
//! top1/top2 margin check, same shape as Rasa NLU's `FallbackClassifier`
//! (`threshold` + `ambiguity_threshold`).
//!
//! `Thresholds::default()` below is calibrated from a real run against the
//! actual embedding model and the bundled exemplars, not guessed — see
//! `router::build::tests::calibration_probe_prints_real_scores`
//! (`cargo test --lib -- --ignored calibration_probe --nocapture`). A first
//! guess of 0.55/0.05 would have rejected ~70% of genuine reminders
//! requests; real reminders probes scored 0.38-0.82 against the domain
//! pool, real off-topic probes scored 0.05-0.38 — the two clusters nearly
//! touch, so 0.40 is deliberately conservative (favors sending a
//! borderline reminders request to the general chat fallback over
//! force-fitting an unrelated request into a reminders action).
//!
//! Caveat: today there's only one domain, so the "absolute floor" is
//! doing the separating work alone — margin only becomes meaningful at
//! the domain level once a second, competing domain's exemplars exist.
//! Re-run the calibration probe whenever the embedding model changes, a
//! new domain is added, or an exemplar file grows meaningfully.

use super::index::TopMatch;

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Top match cleared both thresholds — safe to act on `label`.
    Matched(String),
    /// Cleared the absolute floor, but the runner-up was too close to call.
    Ambiguous,
    /// Didn't even clear the absolute floor.
    NoMatch,
}

pub struct Thresholds {
    pub min_score: f32,
    pub min_margin: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self { min_score: 0.40, min_margin: 0.06 }
    }
}

pub fn decide(top: Option<TopMatch>, thresholds: &Thresholds) -> Decision {
    let Some(top) = top else {
        return Decision::NoMatch;
    };
    if top.score < thresholds.min_score {
        return Decision::NoMatch;
    }
    if top.margin < thresholds.min_margin {
        return Decision::Ambiguous;
    }
    Decision::Matched(top.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(label: &str, score: f32, margin: f32) -> TopMatch {
        TopMatch { label: label.to_string(), text: String::new(), score, margin }
    }

    #[test]
    fn no_match_when_pool_is_empty() {
        assert_eq!(decide(None, &Thresholds::default()), Decision::NoMatch);
    }

    #[test]
    fn no_match_below_absolute_floor() {
        let t = Thresholds::default();
        let d = decide(Some(top("set", 0.1, 0.5)), &t);
        assert_eq!(d, Decision::NoMatch);
    }

    #[test]
    fn ambiguous_when_margin_too_tight() {
        let t = Thresholds::default();
        let d = decide(Some(top("set", 0.9, 0.01)), &t);
        assert_eq!(d, Decision::Ambiguous);
    }

    #[test]
    fn matched_when_both_thresholds_clear() {
        let t = Thresholds::default();
        let d = decide(Some(top("set", 0.9, 0.3)), &t);
        assert_eq!(d, Decision::Matched("set".to_string()));
    }

    #[test]
    fn boundary_scores_are_inclusive() {
        let t = Thresholds::default();
        let d = decide(Some(top("set", t.min_score, t.min_margin)), &t);
        assert_eq!(d, Decision::Matched("set".to_string()));
    }
}
