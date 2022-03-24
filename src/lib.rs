//! src/lib.rs
// make public to other binaries (main, test)
pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod idempotency;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod utils;
