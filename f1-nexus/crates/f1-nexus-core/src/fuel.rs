//! Fuel consumption modeling

use serde::{Deserialize, Serialize};

/// Fuel consumption constants
pub const MAX_FUEL_CAPACITY: f32 = 110.0; // kg (2024 regulations)
pub const MIN_FUEL_BUFFER: f32 = 1.0; // kg (safety margin)
pub const TYPICAL_CONSUMPTION: f32 = 1.6; // kg/lap (average circuit)

/// Fuel consumption model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelConsumptionModel {
    /// Base consumption rate (kg/lap)
    pub base_rate: f32,

    /// Track-specific multiplier
    pub track_multiplier: f32,

    /// Fuel load impact on consumption
    pub fuel_load_factor: f32,

    /// Safety car consumption rate (kg/lap)
    pub safety_car_rate: f32,
}

impl FuelConsumptionModel {
    /// Create default model
    pub fn default_model() -> Self {
        FuelConsumptionModel {
            base_rate: TYPICAL_CONSUMPTION,
            track_multiplier: 1.0,
            fuel_load_factor: 0.0005, // 0.05% increase per kg
            safety_car_rate: 0.4,      // Much lower under SC
        }
    }

    /// Calculate consumption for a lap at full race pace
    pub fn consumption_per_lap(&self, current_fuel_load: f32) -> f32 {
        let fuel_load_impact = 1.0 + (current_fuel_load * self.fuel_load_factor);
        self.base_rate * self.track_multiplier * fuel_load_impact
    }

    /// Calculate laps remaining with current fuel
    pub fn laps_remaining(&self, current_fuel: f32) -> f32 {
        if current_fuel < MIN_FUEL_BUFFER {
            return 0.0;
        }

        let usable_fuel = current_fuel - MIN_FUEL_BUFFER;
        let avg_consumption = self.consumption_per_lap(current_fuel / 2.0);

        if avg_consumption > 0.0 {
            usable_fuel / avg_consumption
        } else {
            f32::INFINITY
        }
    }

    /// Calculate fuel needed for N laps
    pub fn fuel_needed_for_laps(&self, laps: u16, starting_fuel: f32) -> f32 {
        let mut total_fuel = 0.0;
        let mut current_fuel = starting_fuel;

        for _ in 0..laps {
            let consumption = self.consumption_per_lap(current_fuel);
            total_fuel += consumption;
            current_fuel -= consumption;
        }

        total_fuel
    }

    /// Calculate fuel saving required per lap
    pub fn fuel_saving_needed(
        &self,
        current_fuel: f32,
        remaining_laps: u16,
    ) -> Option<f32> {
        let avg_consumption = self.consumption_per_lap(current_fuel / 2.0);
        let total_needed = avg_consumption * remaining_laps as f32;
        let available = current_fuel - MIN_FUEL_BUFFER;

        if total_needed > available {
            let deficit_per_lap = (total_needed - available) / remaining_laps as f32;
            Some(deficit_per_lap)
        } else {
            None // No saving needed
        }
    }
}

/// Fuel strategy recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelStrategyRecommendation {
    /// Can finish race with current fuel
    pub can_finish: bool,

    /// Laps remaining with current fuel
    pub laps_remaining: f32,

    /// Required fuel saving (kg/lap) if needed
    pub fuel_saving_required: Option<f32>,

    /// Recommended engine mode
    pub recommended_mode: EngineMode,

    /// Margin to end of race (laps)
    pub safety_margin: f32,
}

/// Engine fuel modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineMode {
    FullPower,
    Standard,
    FuelSaving,
    CriticalSaving,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_consumption() {
        let model = FuelConsumptionModel::default_model();

        // Test consumption calculation
        let consumption = model.consumption_per_lap(100.0);
        assert!(consumption > 1.5 && consumption < 2.0);

        // Test laps remaining
        let laps = model.laps_remaining(50.0);
        assert!(laps > 25.0 && laps < 35.0);

        // Test fuel needed
        let needed = model.fuel_needed_for_laps(50, 110.0);
        assert!(needed < MAX_FUEL_CAPACITY);
    }

    #[test]
    fn test_fuel_saving() {
        let model = FuelConsumptionModel::default_model();

        // Scenario: 30 laps remaining, only 40kg fuel
        let saving = model.fuel_saving_needed(40.0, 30);

        assert!(saving.is_some());
        assert!(saving.unwrap() > 0.0);
    }
}
