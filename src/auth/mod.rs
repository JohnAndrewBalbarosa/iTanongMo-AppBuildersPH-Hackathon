//! Seeded demonstration identities with password validation and server-side sessions.

mod handlers;
mod limiter;
mod model;
mod password;
mod seed;
pub mod session;

use crate::error::AppResult;

pub use handlers::{routes, session_json};
pub use seed::seed_demo_auth;
pub use session::AuthenticatedSession;

#[derive(Clone)]
pub struct AuthService {
    pub(crate) limiter: limiter::LoginLimiter,
    pub(crate) dummy_password_hash: String,
}

impl AuthService {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            limiter: limiter::LoginLimiter::default(),
            dummy_password_hash: password::hash_password("itanongmo-invalid-account")?,
        })
    }
}
