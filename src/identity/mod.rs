//! Identity boundary. Demo personas implement the same actor contract future eGov adapters will use.

mod actor;
mod authorization;
mod demo;

pub use actor::{ActorContext, ActorType, Role};
pub use authorization::{can_manage_staff, can_review_appeal, can_triage, can_write_request};
pub use demo::{Persona, list_personas, load_actor};
