//! FIA regulations and compliance checking

use serde::{Deserialize, Serialize};

/// FIA F1 sporting regulations (2045 edition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiaRegulations {
    /// Maximum fuel capacity (kg)
    pub max_fuel: f32,

    /// Minimum number of pit stops required
    pub min_pit_stops: u8,

    /// Minimum number of different compounds to use
    pub min_compound_types: u8,

    /// Maximum tire allocation per weekend
    pub max_tire_sets: u8,

    /// Pit lane speed limit (km/h)
    pub pit_lane_speed_limit: f32,

    /// Minimum pit stop time (seconds)
    pub min_pit_stop_time: f32,

    /// DRS detection time delta (seconds)
    pub drs_detection_delta: f32,

    /// Safety car tire changing rules
    pub safety_car_tire_rules: SafetyCarTireRules,
}

impl Default for FiaRegulations {
    fn default() -> Self {
        FiaRegulations {
            max_fuel: 110.0,
            min_pit_stops: 1,
            min_compound_types: 2,
            max_tire_sets: 13,
            pit_lane_speed_limit: 80.0,
            min_pit_stop_time: 2.0,
            drs_detection_delta: 1.0,
            safety_car_tire_rules: SafetyCarTireRules::Allowed,
        }
    }
}

/// Safety car tire changing rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyCarTireRules {
    Allowed,
    NotAllowed,
    ConditionallyAllowed,
}

/// Regulation compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub is_compliant: bool,
    pub violations: Vec<RegulationViolation>,
}

/// Type of regulation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegulationViolation {
    InsufficientPitStops { actual: u8, required: u8 },
    InsufficientCompoundTypes { actual: u8, required: u8 },
    ExcessiveFuel { actual: f32, maximum: f32 },
    PitLaneSpeedViolation { actual: f32, limit: f32 },
    IllegalTireChange { reason: String },
    Other { description: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_regulations() {
        let regs = FiaRegulations::default();
        assert_eq!(regs.max_fuel, 110.0);
        assert_eq!(regs.min_pit_stops, 1);
        assert_eq!(regs.pit_lane_speed_limit, 80.0);
    }
}
