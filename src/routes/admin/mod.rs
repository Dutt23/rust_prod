mod dashboard;
mod logout;
mod newsletter;
mod password;

pub use dashboard::{admin_dashboard, e500};
pub use logout::*;
pub use newsletter::*;
pub use password::*;
