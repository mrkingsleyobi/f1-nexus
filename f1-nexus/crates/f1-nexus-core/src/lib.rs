//! F1 Nexus Core - Domain types and fundamental racing logic
//!
//! This crate provides the core data structures and business logic for F1 racing,
//! including telemetry data, race state, strategy representation, and FIA regulations.

pub mod telemetry;
pub mod strategy;
pub mod race;
pub mod regulations;
pub mod track;
pub mod weather;
pub mod tire;
pub mod fuel;
pub mod types;
pub mod api;

pub use telemetry::*;
pub use strategy::*;
pub use race::*;
pub use regulations::*;
pub use track::*;
pub use weather::*;
pub use tire::*;
pub use fuel::*;
pub use types::*;
pub use api::*;

/// F1 Nexus version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of laps in a typical F1 race
pub const MAX_RACE_LAPS: u16 = 78; // Monaco 2024

/// Number of cars on the grid
pub const GRID_SIZE: u8 = 20;

/// Mandatory pit stop count (2024+ regulations)
pub const MANDATORY_PIT_STOPS: u8 = 1;

/// Telemetry sampling rate (Hz)
pub const TELEMETRY_HZ: u32 = 1000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
