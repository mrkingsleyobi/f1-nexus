//! Core type definitions for F1 Nexus

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Car identifier (1-20)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CarId(pub u8);

impl CarId {
    pub fn new(id: u8) -> Result<Self, &'static str> {
        if id >= 1 && id <= 20 {
            Ok(CarId(id))
        } else {
            Err("CarId must be between 1 and 20")
        }
    }
}

/// Session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        SessionId(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Lap number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LapNumber(pub u16);

impl fmt::Display for LapNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lap {}", self.0)
    }
}

/// Time in milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Milliseconds(pub u64);

impl Milliseconds {
    pub fn as_seconds(&self) -> f64 {
        self.0 as f64 / 1000.0
    }

    pub fn from_seconds(seconds: f64) -> Self {
        Milliseconds((seconds * 1000.0) as u64)
    }
}

/// Position on track (1st, 2nd, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position(pub u8);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

/// Track sector (1, 2, or 3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sector {
    Sector1,
    Sector2,
    Sector3,
}

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Practice1,
    Practice2,
    Practice3,
    Qualifying,
    Sprint,
    Race,
}

/// Flag status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlagStatus {
    Green,
    Yellow,
    DoubleYellow,
    Red,
    SafetyCar,
    VirtualSafetyCar,
    Blue,
    Black,
    Checkered,
}

/// Weather condition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherCondition {
    Dry,
    LightRain,
    HeavyRain,
    Cloudy,
    PartlyCloudy,
}

/// Track condition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackCondition {
    Dry,
    Damp,
    Wet,
    VeryWet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_car_id_validation() {
        assert!(CarId::new(1).is_ok());
        assert!(CarId::new(20).is_ok());
        assert!(CarId::new(0).is_err());
        assert!(CarId::new(21).is_err());
    }

    #[test]
    fn test_milliseconds_conversion() {
        let ms = Milliseconds::from_seconds(1.5);
        assert_eq!(ms.as_seconds(), 1.5);
    }
}
