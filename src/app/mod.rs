//! Application composition root.

pub mod routes;
pub mod startup;
pub mod state;

pub use routes::router;
pub use state::{AppConfig, AppState};
