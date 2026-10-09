//! iTanongMo application library.
//!
//! Module map:
//!   app            startup, shared state, HTTP routes, background jobs
//!   identity       demo actors and authorization boundary for future eGov identity
//!   organization   agency directory, hierarchy, and request routing
//!   requests       FOI intake, conversations, deadlines, and legal state machine
//!   board          government Kanban projections
//!   triage         high-recall deterministic rules and human review
//!   ai             provider-neutral AI jobs and private Modal adapter
//!   notifications  in-app notification feed and future delivery boundary
//!   audit          immutable, redacted activity records

pub mod ai;
pub mod app;
pub mod audit;
pub mod auth;
pub mod board;
pub mod error;
pub mod identity;
pub mod notifications;
pub mod organization;
pub mod requester_ai;
pub mod requests;
pub mod triage;
