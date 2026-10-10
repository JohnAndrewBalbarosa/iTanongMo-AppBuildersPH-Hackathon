//! Isolated AI boundary. Model output is advisory, schema-validated, and never mutates FOI state.

mod jobs;
mod openai_compatible;
mod provider;
mod schemas;

pub use jobs::{AiDraftRow, AiJobOutcome, process_next_job, queue_government_draft};
pub use openai_compatible::OpenAiCompatibleProvider;
pub use provider::AiProvider;
pub use schemas::{DraftSuggestion, TriageSuggestion};
