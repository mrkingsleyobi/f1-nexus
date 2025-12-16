//! CLI command implementations

pub mod optimize;
pub mod simulate;

// Re-export create_test_circuit for shared use
pub use optimize::create_test_circuit;
