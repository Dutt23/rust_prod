//! src/authentication/mod.rs
mod authentication;
mod middleware;
pub use authentication::{change_password, validate_credentials, AuthError, Credentials};
pub use middleware::reject_anonymous_users;
