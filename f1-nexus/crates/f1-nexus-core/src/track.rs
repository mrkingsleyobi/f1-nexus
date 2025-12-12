//! Track definitions and characteristics

use crate::types::Sector;
use nalgebra as na;
use serde::{Deserialize, Serialize};

/// F1 circuit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    /// Unique track identifier
    pub id: String,

    /// Full track name
    pub name: String,

    /// Country
    pub country: String,

    /// Track length (meters)
    pub length: f32,

    /// Number of turns
    pub num_turns: u8,

    /// Lap record (seconds)
    pub lap_record: f32,

    /// Track characteristics
    pub characteristics: TrackCharacteristics,

    /// Sector information
    pub sectors: Vec<SectorInfo>,

    /// DRS zones
    pub drs_zones: Vec<DrsZone>,

    /// Typical race distance (laps)
    pub typical_race_laps: u16,
}

/// Track-specific characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackCharacteristics {
    /// Tire degradation severity (1.0 = average)
    pub tire_severity: f32,

    /// Fuel consumption rate (1.0 = average)
    pub fuel_consumption: f32,

    /// Overtaking difficulty (0.0 = easy, 1.0 = very hard)
    pub overtaking_difficulty: f32,

    /// Downforce level (0.0 = low, 1.0 = high)
    pub downforce_level: f32,

    /// Average speed (km/h)
    pub average_speed: f32,

    /// Maximum speed (km/h)
    pub maximum_speed: f32,

    /// Elevation change (meters)
    pub elevation_change: f32,

    /// Weather variability (0.0 = stable, 1.0 = highly variable)
    pub weather_variability: f32,
}

/// Sector information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorInfo {
    pub sector: Sector,

    /// Sector length (meters)
    pub length: f32,

    /// Average time (seconds)
    pub average_time: f32,

    /// Sector type
    pub sector_type: SectorType,

    /// Key corners in this sector
    pub key_corners: Vec<Corner>,
}

/// Sector type characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectorType {
    Straights,     // High speed, low downforce
    Technical,     // Low-medium speed, high downforce
    HighSpeed,     // High speed corners
    MixedSpeed,    // Combination
}

/// Corner definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corner {
    pub number: u8,
    pub name: Option<String>,
    pub apex_speed: f32,      // km/h
    pub entry_speed: f32,     // km/h
    pub exit_speed: f32,      // km/h
    pub corner_type: CornerType,
    pub gear: u8,
}

/// Corner classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CornerType {
    Slow,          // <100 km/h
    Medium,        // 100-200 km/h
    Fast,          // 200-250 km/h
    VeryFast,      // >250 km/h
    Hairpin,
    Chicane,
}

/// DRS (Drag Reduction System) zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrsZone {
    pub zone_id: u8,

    /// Detection point (meters from start/finish)
    pub detection_point: f32,

    /// Activation point (meters from start/finish)
    pub activation_point: f32,

    /// End point (meters from start/finish)
    pub end_point: f32,

    /// Expected lap time gain (seconds)
    pub expected_time_gain: f32,
}

impl Circuit {
    /// Get total track length
    pub fn total_length(&self) -> f32 {
        self.length
    }

    /// Calculate race distance in kilometers
    pub fn race_distance_km(&self) -> f32 {
        (self.length * self.typical_race_laps as f32) / 1000.0
    }

    /// Get sector by type
    pub fn sector(&self, sector: Sector) -> Option<&SectorInfo> {
        self.sectors.iter().find(|s| s.sector == sector)
    }

    /// Check if circuit has DRS
    pub fn has_drs(&self) -> bool {
        !self.drs_zones.is_empty()
    }

    /// Get famous circuits (2045)
    pub fn famous_circuits() -> Vec<Circuit> {
        vec![
            Self::monaco(),
            Self::spa(),
            Self::silverstone(),
            Self::monza(),
            Self::suzuka(),
        ]
    }

    /// Monaco circuit definition
    pub fn monaco() -> Self {
        Circuit {
            id: "monaco".to_string(),
            name: "Circuit de Monaco".to_string(),
            country: "Monaco".to_string(),
            length: 3337.0,
            num_turns: 19,
            lap_record: 70.246,
            characteristics: TrackCharacteristics {
                tire_severity: 0.8,
                fuel_consumption: 0.85,
                overtaking_difficulty: 0.95,
                downforce_level: 0.95,
                average_speed: 160.0,
                maximum_speed: 290.0,
                elevation_change: 42.0,
                weather_variability: 0.3,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 78,
        }
    }

    /// Spa-Francorchamps circuit
    pub fn spa() -> Self {
        Circuit {
            id: "spa".to_string(),
            name: "Circuit de Spa-Francorchamps".to_string(),
            country: "Belgium".to_string(),
            length: 7004.0,
            num_turns: 19,
            lap_record: 103.458,
            characteristics: TrackCharacteristics {
                tire_severity: 1.2,
                fuel_consumption: 1.3,
                overtaking_difficulty: 0.4,
                downforce_level: 0.6,
                average_speed: 237.0,
                maximum_speed: 340.0,
                elevation_change: 105.0,
                weather_variability: 0.9,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 44,
        }
    }

    /// Silverstone circuit
    pub fn silverstone() -> Self {
        Circuit {
            id: "silverstone".to_string(),
            name: "Silverstone Circuit".to_string(),
            country: "United Kingdom".to_string(),
            length: 5891.0,
            num_turns: 18,
            lap_record: 86.089,
            characteristics: TrackCharacteristics {
                tire_severity: 1.1,
                fuel_consumption: 1.1,
                overtaking_difficulty: 0.5,
                downforce_level: 0.7,
                average_speed: 230.0,
                maximum_speed: 330.0,
                elevation_change: 30.0,
                weather_variability: 0.7,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 52,
        }
    }

    /// Monza circuit
    pub fn monza() -> Self {
        Circuit {
            id: "monza".to_string(),
            name: "Autodromo Nazionale di Monza".to_string(),
            country: "Italy".to_string(),
            length: 5793.0,
            num_turns: 11,
            lap_record: 81.046,
            characteristics: TrackCharacteristics {
                tire_severity: 0.9,
                fuel_consumption: 1.4,
                overtaking_difficulty: 0.3,
                downforce_level: 0.3,
                average_speed: 264.0,
                maximum_speed: 360.0,
                elevation_change: 28.0,
                weather_variability: 0.4,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 53,
        }
    }

    /// Suzuka circuit
    pub fn suzuka() -> Self {
        Circuit {
            id: "suzuka".to_string(),
            name: "Suzuka International Racing Course".to_string(),
            country: "Japan".to_string(),
            length: 5807.0,
            num_turns: 18,
            lap_record: 87.435,
            characteristics: TrackCharacteristics {
                tire_severity: 1.3,
                fuel_consumption: 1.2,
                overtaking_difficulty: 0.7,
                downforce_level: 0.8,
                average_speed: 226.0,
                maximum_speed: 315.0,
                elevation_change: 43.0,
                weather_variability: 0.6,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 53,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit() {
        let monaco = Circuit::monaco();
        assert_eq!(monaco.id, "monaco");
        assert!(monaco.race_distance_km() > 250.0);
        assert!(monaco.characteristics.overtaking_difficulty > 0.9);
    }

    #[test]
    fn test_famous_circuits() {
        let circuits = Circuit::famous_circuits();
        assert_eq!(circuits.len(), 5);
        assert!(circuits.iter().any(|c| c.id == "spa"));
    }
}
