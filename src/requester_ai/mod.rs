//! Requester-controlled AI drafting through a stateless, non-persisting credential relay.

mod provider;
mod schemas;

pub use provider::RequesterAiService;
pub use schemas::{RequesterAiDraft, RequesterAiInput};
