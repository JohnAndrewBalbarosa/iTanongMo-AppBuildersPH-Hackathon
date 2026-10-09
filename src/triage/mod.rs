//! High-recall deterministic triage. Rules surface candidates; they never discard requests.

mod rules;

pub use rules::{RuleHit, TriageView, load_triage, run_rules_and_queue_ai};
