//! Tire modeling and degradation physics

use serde::{Deserialize, Serialize};

pub use crate::telemetry::TireCompound;

/// Tire degradation model constants
pub const OPTIMAL_TIRE_TEMP_MIN: f32 = 90.0; // °C
pub const OPTIMAL_TIRE_TEMP_MAX: f32 = 110.0; // °C
pub const CRITICAL_TIRE_TEMP: f32 = 120.0; // °C
pub const DEGRADATION_RATE_BASE: f32 = 0.01; // wear per lap at optimal conditions

/// Tire compound characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireCharacteristics {
    pub compound: TireCompound,
    pub grip_level: f32,        // 0.0-1.0
    pub degradation_rate: f32,  // base wear per lap
    pub optimal_temp_range: (f32, f32), // °C
    pub heat_up_rate: f32,      // °C per lap
    pub cool_down_rate: f32,    // °C per lap
    pub typical_life: u16,      // laps
}

impl TireCharacteristics {
    /// Get characteristics for a compound
    pub fn for_compound(compound: TireCompound) -> Self {
        match compound {
            TireCompound::C0 => TireCharacteristics {
                compound,
                grip_level: 0.70,
                degradation_rate: 0.005,
                optimal_temp_range: (85.0, 105.0),
                heat_up_rate: 2.0,
                cool_down_rate: 1.5,
                typical_life: 40,
            },
            TireCompound::C1 => TireCharacteristics {
                compound,
                grip_level: 0.75,
                degradation_rate: 0.007,
                optimal_temp_range: (90.0, 110.0),
                heat_up_rate: 2.5,
                cool_down_rate: 1.8,
                typical_life: 35,
            },
            TireCompound::C2 => TireCharacteristics {
                compound,
                grip_level: 0.80,
                degradation_rate: 0.010,
                optimal_temp_range: (90.0, 110.0),
                heat_up_rate: 3.0,
                cool_down_rate: 2.0,
                typical_life: 30,
            },
            TireCompound::C3 => TireCharacteristics {
                compound,
                grip_level: 0.85,
                degradation_rate: 0.013,
                optimal_temp_range: (92.0, 112.0),
                heat_up_rate: 3.5,
                cool_down_rate: 2.2,
                typical_life: 25,
            },
            TireCompound::C4 => TireCharacteristics {
                compound,
                grip_level: 0.90,
                degradation_rate: 0.017,
                optimal_temp_range: (95.0, 115.0),
                heat_up_rate: 4.0,
                cool_down_rate: 2.5,
                typical_life: 20,
            },
            TireCompound::C5 => TireCharacteristics {
                compound,
                grip_level: 0.95,
                degradation_rate: 0.022,
                optimal_temp_range: (95.0, 115.0),
                heat_up_rate: 4.5,
                cool_down_rate: 2.8,
                typical_life: 15,
            },
            TireCompound::Intermediate => TireCharacteristics {
                compound,
                grip_level: 0.75,
                degradation_rate: 0.015,
                optimal_temp_range: (70.0, 90.0),
                heat_up_rate: 3.0,
                cool_down_rate: 2.0,
                typical_life: 30,
            },
            TireCompound::Wet => TireCharacteristics {
                compound,
                grip_level: 0.65,
                degradation_rate: 0.012,
                optimal_temp_range: (60.0, 80.0),
                heat_up_rate: 2.5,
                cool_down_rate: 1.5,
                typical_life: 35,
            },
        }
    }

    /// Calculate grip multiplier based on temperature
    pub fn grip_multiplier_for_temp(&self, temp: f32) -> f32 {
        let (min_temp, max_temp) = self.optimal_temp_range;

        if temp >= min_temp && temp <= max_temp {
            // In optimal range
            1.0
        } else if temp < min_temp {
            // Too cold - linear falloff
            let delta = min_temp - temp;
            (1.0 - delta * 0.02).max(0.5)
        } else {
            // Too hot - exponential falloff
            let delta = temp - max_temp;
            (1.0 - delta * 0.03).max(0.3)
        }
    }

    /// Predict remaining life (laps) based on current wear
    pub fn predict_remaining_life(&self, current_wear: f32, track_severity: f32) -> f32 {
        let remaining_wear = 1.0 - current_wear;
        let adjusted_degradation = self.degradation_rate * track_severity;

        if adjusted_degradation > 0.0 {
            remaining_wear / adjusted_degradation
        } else {
            f32::INFINITY
        }
    }
}

/// Tire degradation prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireDegradationPrediction {
    /// Current lap
    pub current_lap: u16,

    /// Current wear level (0.0-1.0)
    pub current_wear: f32,

    /// Predicted wear per lap
    pub wear_per_lap: f32,

    /// Predicted laps remaining
    pub laps_remaining: f32,

    /// Confidence interval (lower, upper)
    pub confidence_interval: (f32, f32),

    /// Factors affecting degradation
    pub factors: DegradationFactors,
}

/// Factors affecting tire degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationFactors {
    /// Track abrasiveness (1.0 = average)
    pub track_severity: f32,

    /// Temperature impact (1.0 = optimal)
    pub temperature_factor: f32,

    /// Driving style aggressiveness (1.0 = neutral)
    pub driving_style_factor: f32,

    /// Fuel load impact (1.0 = average)
    pub fuel_load_factor: f32,

    /// Downforce level (1.0 = race trim)
    pub downforce_factor: f32,
}

impl Default for DegradationFactors {
    fn default() -> Self {
        DegradationFactors {
            track_severity: 1.0,
            temperature_factor: 1.0,
            driving_style_factor: 1.0,
            fuel_load_factor: 1.0,
            downforce_factor: 1.0,
        }
    }
}

impl DegradationFactors {
    /// Calculate combined degradation multiplier
    pub fn total_multiplier(&self) -> f32 {
        self.track_severity
            * self.temperature_factor
            * self.driving_style_factor
            * self.fuel_load_factor
            * self.downforce_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tire_characteristics() {
        let c3 = TireCharacteristics::for_compound(TireCompound::C3);
        assert_eq!(c3.compound, TireCompound::C3);
        assert!(c3.grip_level > 0.8);
        assert!(c3.typical_life < 30);

        // Test grip multiplier
        assert_eq!(c3.grip_multiplier_for_temp(100.0), 1.0); // optimal
        assert!(c3.grip_multiplier_for_temp(70.0) < 1.0); // too cold
        assert!(c3.grip_multiplier_for_temp(130.0) < 1.0); // too hot
    }

    #[test]
    fn test_degradation_factors() {
        let factors = DegradationFactors {
            track_severity: 1.2,
            temperature_factor: 0.9,
            driving_style_factor: 1.1,
            fuel_load_factor: 1.0,
            downforce_factor: 1.0,
        };

        let multiplier = factors.total_multiplier();
        assert!((multiplier - 1.188).abs() < 0.01);
    }

    #[test]
    fn test_remaining_life_prediction() {
        let c3 = TireCharacteristics::for_compound(TireCompound::C3);
        let remaining = c3.predict_remaining_life(0.5, 1.0);

        // With 50% wear remaining and 0.013 degradation rate
        // Should predict ~38 laps remaining
        assert!(remaining > 35.0 && remaining < 40.0);
    }
}
