//! Authenticated citizen drafting through the shared server-owned AI provider.

mod provider;
mod schemas;

pub use provider::RequesterAiService;
pub use schemas::{RequesterAiDraft, RequesterAiInput, response_schema};
