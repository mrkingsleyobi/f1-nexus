//! Race simulation engine with lap-by-lap prediction
//!
//! This module provides comprehensive race simulation capabilities, including:
//! - Lap-by-lap race progression modeling
//! - Tire degradation tracking and impact
//! - Fuel consumption and weight effects
//! - Pit stop execution and time loss
//! - Weather condition changes
//! - Strategy validation and warnings

use f1_nexus_core::{
    Circuit, FuelConsumptionModel, LapNumber, PitStop, RaceStrategy,
    TireCharacteristics, TireCompound, DegradationFactors,
    WeatherForecast, WeatherCondition,
};
use serde::{Deserialize, Serialize};

/// Race simulator for lap-by-lap prediction
#[derive(Debug, Clone)]
pub struct RaceSimulator {
    /// Circuit being raced on
    pub circuit: Circuit,

    /// Race strategy to simulate
    pub strategy: RaceStrategy,

    /// Fuel consumption model
    pub fuel_model: FuelConsumptionModel,

    /// Weather conditions (initial and forecasted changes)
    pub weather: WeatherConditions,
}

/// Weather conditions for simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    /// Starting weather condition
    pub initial_condition: WeatherCondition,

    /// Initial track temperature (°C)
    pub track_temperature: f32,

    /// Initial air temperature (°C)
    pub air_temperature: f32,

    /// Weather changes during race (lap number -> new condition)
    pub changes: Vec<(LapNumber, WeatherCondition, f32)>, // (lap, condition, track_temp)
}

impl WeatherConditions {
    /// Create from WeatherForecast
    pub fn from_forecast(forecast: &WeatherForecast) -> Self {
        WeatherConditions {
            initial_condition: forecast.overall_condition,
            track_temperature: forecast.track_temperature,
            air_temperature: forecast.air_temperature,
            changes: vec![],
        }
    }

    /// Get weather condition for a specific lap
    pub fn condition_at_lap(&self, lap: LapNumber) -> WeatherCondition {
        self.changes
            .iter()
            .rev()
            .find(|(change_lap, _, _)| change_lap.0 <= lap.0)
            .map(|(_, condition, _)| *condition)
            .unwrap_or(self.initial_condition)
    }

    /// Get track temperature for a specific lap
    pub fn track_temp_at_lap(&self, lap: LapNumber) -> f32 {
        self.changes
            .iter()
            .rev()
            .find(|(change_lap, _, _)| change_lap.0 <= lap.0)
            .map(|(_, _, temp)| *temp)
            .unwrap_or(self.track_temperature)
    }
}

/// Complete simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Total race time (seconds)
    pub total_time: f32,

    /// Lap times for each lap
    pub lap_times: Vec<f32>,

    /// Pit stop events that occurred
    pub pit_stops: Vec<PitStopEvent>,

    /// Tire compound history (lap number, compound)
    pub tire_history: Vec<(LapNumber, TireCompound)>,

    /// Fuel remaining at end of each lap (kg)
    pub fuel_history: Vec<f32>,

    /// Simulation warnings
    pub warnings: Vec<String>,

    /// Final position (estimated)
    pub estimated_position: Option<u8>,

    /// Average lap time
    pub average_lap_time: f32,

    /// Fastest lap time
    pub fastest_lap: f32,

    /// Slowest lap time
    pub slowest_lap: f32,
}

/// Pit stop event in simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitStopEvent {
    /// Lap number when pit stop occurred
    pub lap: LapNumber,

    /// Old tire compound
    pub old_compound: TireCompound,

    /// New tire compound
    pub new_compound: TireCompound,

    /// Pit stop duration (seconds)
    pub duration: f32,

    /// Tire age when pitted (laps)
    pub tire_age: u16,

    /// Fuel remaining when pitted (kg)
    pub fuel_remaining: f32,
}

impl RaceSimulator {
    /// Create a new race simulator
    pub fn new(
        circuit: Circuit,
        strategy: RaceStrategy,
        fuel_model: FuelConsumptionModel,
        weather: WeatherConditions,
    ) -> Self {
        RaceSimulator {
            circuit,
            strategy,
            fuel_model,
            weather,
        }
    }

    /// Simulate the complete race lap-by-lap
    pub fn simulate_race(&self) -> SimulationResult {
        let total_laps = self.circuit.typical_race_laps;
        let mut lap_times = Vec::with_capacity(total_laps as usize);
        let mut pit_stop_events = Vec::new();
        let mut tire_history = vec![(LapNumber(1), self.strategy.starting_compound)];
        let mut fuel_history = Vec::with_capacity(total_laps as usize);
        let mut warnings = Vec::new();

        // Initialize state
        let mut current_fuel = self.strategy.fuel_strategy.starting_fuel;
        let mut current_compound = self.strategy.starting_compound;
        let mut tire_age = 0u16;
        let mut total_time = 0.0f32;

        // Simulate each lap
        for lap in 1..=total_laps {
            let lap_number = LapNumber(lap);

            // Check if we're pitting this lap
            let is_pit_lap = self.strategy.pit_stop_on_lap(lap_number).is_some();

            // Calculate lap time BEFORE pit stop
            tire_age += 1;
            let lap_time = self.calculate_lap_time(
                lap_number,
                current_compound,
                tire_age,
                current_fuel,
            );

            lap_times.push(lap_time);
            total_time += lap_time;

            // Update fuel consumption
            let fuel_consumed = self.fuel_model.consumption_per_lap(current_fuel);
            current_fuel -= fuel_consumed;
            fuel_history.push(current_fuel);

            // Check for fuel warnings
            if current_fuel < 5.0 {
                warnings.push(format!(
                    "Low fuel warning at lap {}: {:.2} kg remaining",
                    lap, current_fuel
                ));
            }

            let laps_remaining = total_laps - lap;
            if laps_remaining > 0 {
                let fuel_needed = self.fuel_model.fuel_needed_for_laps(
                    laps_remaining,
                    current_fuel,
                );
                if fuel_needed > current_fuel {
                    warnings.push(format!(
                        "Fuel insufficient at lap {}: need {:.2} kg, have {:.2} kg",
                        lap, fuel_needed, current_fuel
                    ));
                }
            }

            // Execute pit stop if scheduled
            if is_pit_lap {
                let pit_stop = self.strategy.pit_stop_on_lap(lap_number).unwrap();
                let old_compound = current_compound;

                // Record pit stop event
                pit_stop_events.push(PitStopEvent {
                    lap: lap_number,
                    old_compound,
                    new_compound: pit_stop.compound,
                    duration: pit_stop.pit_loss,
                    tire_age,
                    fuel_remaining: current_fuel,
                });

                // Add pit stop time
                total_time += pit_stop.pit_loss;

                // Change tires
                current_compound = pit_stop.compound;
                tire_age = 0;
                tire_history.push((lap_number, current_compound));
            }

            // Check tire degradation warnings
            let tire_chars = TireCharacteristics::for_compound(current_compound);
            if tire_age > tire_chars.typical_life {
                warnings.push(format!(
                    "Tire age exceeded typical life at lap {}: {} laps on {} (typical: {})",
                    lap, tire_age, format!("{:?}", current_compound), tire_chars.typical_life
                ));
            }

            // Check weather changes
            let current_weather = self.weather.condition_at_lap(lap_number);
            if self.is_wrong_tire_for_weather(current_compound, current_weather) {
                warnings.push(format!(
                    "Wrong tire compound at lap {}: {:?} tires in {:?} conditions",
                    lap, current_compound, current_weather
                ));
            }
        }

        // Calculate statistics
        let average_lap_time = if !lap_times.is_empty() {
            lap_times.iter().sum::<f32>() / lap_times.len() as f32
        } else {
            0.0
        };

        let fastest_lap = lap_times.iter().cloned().fold(f32::INFINITY, f32::min);
        let slowest_lap = lap_times.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Validate strategy completion
        if current_fuel < 0.0 {
            warnings.push("Strategy failed: ran out of fuel before race end".to_string());
        }

        if pit_stop_events.is_empty() {
            warnings.push("Strategy invalid: no pit stops executed (FIA regulation violation)".to_string());
        }

        SimulationResult {
            total_time,
            lap_times,
            pit_stops: pit_stop_events,
            tire_history,
            fuel_history,
            warnings,
            estimated_position: None, // Can be enhanced with competitor simulation
            average_lap_time,
            fastest_lap,
            slowest_lap,
        }
    }

    /// Calculate lap time considering all factors
    fn calculate_lap_time(
        &self,
        lap: LapNumber,
        compound: TireCompound,
        tire_age: u16,
        current_fuel: f32,
    ) -> f32 {
        let tire_chars = TireCharacteristics::for_compound(compound);

        // Base lap time (slightly slower than lap record for realistic race pace)
        let base_time = self.circuit.lap_record * 1.03;

        // 1. Tire degradation penalty
        let wear_ratio = tire_age as f32 / tire_chars.typical_life as f32;
        let degradation_penalty = wear_ratio.powf(1.5) * 1.5; // Non-linear degradation

        // 2. Fuel weight penalty (heavier car = slower)
        // Each kg of fuel costs ~0.03s per lap
        let fuel_penalty = (current_fuel / 110.0) * 0.35;

        // 3. Track temperature effect
        let track_temp = self.weather.track_temp_at_lap(lap);
        let temp_penalty = self.calculate_temperature_penalty(track_temp, &tire_chars);

        // 4. Weather condition penalty
        let weather = self.weather.condition_at_lap(lap);
        let weather_penalty = self.calculate_weather_penalty(weather, compound);

        // 5. Tire compound grip advantage
        let grip_bonus = (tire_chars.grip_level - 0.75) * 0.8;

        // 6. Circuit-specific tire degradation
        let track_severity = self.circuit.characteristics.tire_severity;
        let track_deg_penalty = (track_severity - 1.0) * wear_ratio * 0.5;

        // Combine all factors
        base_time + degradation_penalty + fuel_penalty + temp_penalty
            + weather_penalty + track_deg_penalty - grip_bonus
    }

    /// Calculate penalty from track temperature
    fn calculate_temperature_penalty(&self, track_temp: f32, tire_chars: &TireCharacteristics) -> f32 {
        let (min_temp, max_temp) = tire_chars.optimal_temp_range;

        if track_temp >= min_temp && track_temp <= max_temp {
            0.0 // Optimal conditions
        } else if track_temp < min_temp {
            // Too cold - tires don't work well
            let delta = min_temp - track_temp;
            delta * 0.05 // 0.05s per degree below optimal
        } else {
            // Too hot - tire degradation accelerates
            let delta = track_temp - max_temp;
            delta * 0.08 // 0.08s per degree above optimal
        }
    }

    /// Calculate penalty from weather conditions
    fn calculate_weather_penalty(&self, weather: WeatherCondition, compound: TireCompound) -> f32 {
        match (weather, compound) {
            // Correct tire choices
            (WeatherCondition::Dry, TireCompound::C0..=TireCompound::C5) => 0.0,
            (WeatherCondition::Cloudy, TireCompound::C0..=TireCompound::C5) => 0.0,
            (WeatherCondition::PartlyCloudy, TireCompound::C0..=TireCompound::C5) => 0.0,
            (WeatherCondition::LightRain, TireCompound::Intermediate) => 0.0,
            (WeatherCondition::HeavyRain, TireCompound::Wet) => 0.0,

            // Wrong tire in rain
            (WeatherCondition::LightRain, TireCompound::C0..=TireCompound::C5) => 5.0,
            (WeatherCondition::HeavyRain, TireCompound::C0..=TireCompound::C5) => 15.0,
            (WeatherCondition::HeavyRain, TireCompound::Intermediate) => 3.0,

            // Wets/Inters on dry track
            (WeatherCondition::Dry, TireCompound::Intermediate) => 2.5,
            (WeatherCondition::Dry, TireCompound::Wet) => 5.0,
            (WeatherCondition::Cloudy, TireCompound::Wet) => 4.0,

            // Other combinations
            _ => 1.0,
        }
    }

    /// Check if tire compound is inappropriate for weather
    fn is_wrong_tire_for_weather(&self, compound: TireCompound, weather: WeatherCondition) -> bool {
        matches!(
            (weather, compound),
            (WeatherCondition::LightRain, TireCompound::C0..=TireCompound::C5) |
            (WeatherCondition::HeavyRain, TireCompound::C0..=TireCompound::C5) |
            (WeatherCondition::HeavyRain, TireCompound::Intermediate)
        )
    }
}

/// Helper function to create a simple race simulator
pub fn create_simulator(
    circuit: Circuit,
    strategy: RaceStrategy,
    degradation_factors: DegradationFactors,
) -> RaceSimulator {
    let mut fuel_model = FuelConsumptionModel::default_model();
    fuel_model.track_multiplier = circuit.characteristics.fuel_consumption;

    let weather = WeatherConditions {
        initial_condition: WeatherCondition::Dry,
        track_temperature: 30.0,
        air_temperature: 25.0,
        changes: vec![],
    };

    RaceSimulator::new(circuit, strategy, fuel_model, weather)
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::{
        Circuit, PitStop, PitStopReason, RaceStrategy, FuelStrategy,
        ErsDeploymentPlan, StrategyMetadata,
    };
    use f1_nexus_core::strategy::ErsMode;
    use std::collections::BTreeMap;

    fn create_test_strategy() -> RaceStrategy {
        RaceStrategy {
            id: uuid::Uuid::new_v4().to_string(),
            starting_compound: TireCompound::C3,
            pit_stops: vec![
                PitStop {
                    lap: LapNumber(20),
                    compound: TireCompound::C4,
                    pit_loss: 22.0,
                    reason: PitStopReason::Mandatory,
                    confidence: 0.9,
                },
                PitStop {
                    lap: LapNumber(40),
                    compound: TireCompound::C5,
                    pit_loss: 21.5,
                    reason: PitStopReason::TireDegradation,
                    confidence: 0.85,
                },
            ],
            fuel_strategy: FuelStrategy {
                starting_fuel: 110.0,
                fuel_saving_per_lap: 0.0,
                fuel_saving_laps: vec![],
                minimum_buffer: 1.5,
            },
            ers_plan: ErsDeploymentPlan {
                default_mode: ErsMode::Medium,
                lap_overrides: BTreeMap::new(),
                overtake_laps: vec![],
            },
            expected_lap_times: BTreeMap::new(),
            predicted_race_time: 0.0, // Will be calculated
            confidence: 0.85,
            metadata: StrategyMetadata {
                generated_at: chrono::Utc::now(),
                num_simulations: 1,
                contributing_agents: vec!["test-simulator".to_string()],
                version_hash: None,
                parent_strategy_id: None,
            },
        }
    }

    #[test]
    fn test_race_simulation_basic() {
        let circuit = Circuit::monaco();
        let strategy = create_test_strategy();
        let fuel_model = FuelConsumptionModel::default_model();
        let weather = WeatherConditions {
            initial_condition: WeatherCondition::Dry,
            track_temperature: 30.0,
            air_temperature: 25.0,
            changes: vec![],
        };

        let simulator = RaceSimulator::new(circuit, strategy, fuel_model, weather);
        let result = simulator.simulate_race();

        // Basic validations
        assert_eq!(result.lap_times.len(), 78); // Monaco has 78 laps
        assert_eq!(result.pit_stops.len(), 2); // Two pit stops
        assert_eq!(result.fuel_history.len(), 78);

        // Should have positive total time
        assert!(result.total_time > 0.0);

        // Should have reasonable race time (between 1.5 and 2.5 hours)
        assert!(result.total_time > 5400.0); // > 1.5 hours
        assert!(result.total_time < 9000.0); // < 2.5 hours

        // Average lap time should be reasonable
        assert!(result.average_lap_time > 70.0); // Monaco lap record is ~70s
        assert!(result.average_lap_time < 90.0);

        // Fastest lap should be faster than slowest
        assert!(result.fastest_lap < result.slowest_lap);

        // Tire history should track compound changes
        assert_eq!(result.tire_history.len(), 3); // Starting + 2 pit stops
        assert_eq!(result.tire_history[0].1, TireCompound::C3);
        assert_eq!(result.tire_history[1].1, TireCompound::C4);
        assert_eq!(result.tire_history[2].1, TireCompound::C5);
    }

    #[test]
    fn test_pit_stop_events() {
        let circuit = Circuit::monaco();
        let strategy = create_test_strategy();
        let simulator = create_simulator(circuit, strategy, DegradationFactors::default());

        let result = simulator.simulate_race();

        // Check first pit stop
        assert_eq!(result.pit_stops[0].lap.0, 20);
        assert_eq!(result.pit_stops[0].old_compound, TireCompound::C3);
        assert_eq!(result.pit_stops[0].new_compound, TireCompound::C4);
        assert_eq!(result.pit_stops[0].tire_age, 20); // 20 laps on first stint

        // Check second pit stop
        assert_eq!(result.pit_stops[1].lap.0, 40);
        assert_eq!(result.pit_stops[1].old_compound, TireCompound::C4);
        assert_eq!(result.pit_stops[1].new_compound, TireCompound::C5);
        assert_eq!(result.pit_stops[1].tire_age, 20); // 20 laps on second stint

        // Fuel should decrease at each pit stop
        assert!(result.pit_stops[0].fuel_remaining > result.pit_stops[1].fuel_remaining);
    }

    #[test]
    fn test_fuel_consumption() {
        let circuit = Circuit::silverstone();
        let strategy = create_test_strategy();
        let simulator = create_simulator(circuit, strategy, DegradationFactors::default());

        let result = simulator.simulate_race();

        // Fuel should decrease monotonically
        for i in 1..result.fuel_history.len() {
            assert!(result.fuel_history[i] < result.fuel_history[i - 1]);
        }

        // Should have positive fuel at end (or warning)
        let final_fuel = result.fuel_history.last().unwrap();
        if *final_fuel < 0.0 {
            assert!(result.warnings.iter().any(|w| w.contains("fuel")));
        }
    }

    #[test]
    fn test_tire_degradation_effect() {
        let circuit = Circuit::monaco();
        let strategy = create_test_strategy();
        let simulator = create_simulator(circuit, strategy, DegradationFactors::default());

        let result = simulator.simulate_race();

        // Lap times in first stint should generally increase (tire deg)
        // Compare lap 5 to lap 19 (before first pit on lap 20)
        let lap_5_time = result.lap_times[4];
        let lap_19_time = result.lap_times[18];
        assert!(lap_19_time > lap_5_time); // Should be slower due to tire wear

        // After pit stop, lap time should improve (fresh tires)
        let lap_20_time = result.lap_times[19]; // Still on old tires
        let lap_21_time = result.lap_times[20]; // New tires
        assert!(lap_21_time < lap_20_time); // Should be faster with fresh tires
    }

    #[test]
    fn test_weather_changes() {
        let circuit = Circuit::spa(); // Known for variable weather
        let mut strategy = create_test_strategy();

        // Add wet tire stop for weather change
        strategy.pit_stops.push(PitStop {
            lap: LapNumber(15),
            compound: TireCompound::Intermediate,
            pit_loss: 24.0,
            reason: PitStopReason::WeatherChange,
            confidence: 0.95,
        });

        let fuel_model = FuelConsumptionModel::default_model();
        let weather = WeatherConditions {
            initial_condition: WeatherCondition::Dry,
            track_temperature: 25.0,
            air_temperature: 20.0,
            changes: vec![
                (LapNumber(15), WeatherCondition::LightRain, 18.0),
                (LapNumber(30), WeatherCondition::Dry, 22.0),
            ],
        };

        let simulator = RaceSimulator::new(circuit, strategy, fuel_model, weather);
        let result = simulator.simulate_race();

        // Should have warnings about wrong tires if timing is off
        // Or no warnings if tire changes align with weather
        assert!(result.total_time > 0.0);
        assert!(result.pit_stops.len() >= 2);
    }

    #[test]
    fn test_insufficient_fuel_warning() {
        let circuit = Circuit::monaco();
        let mut strategy = create_test_strategy();

        // Set insufficient starting fuel
        strategy.fuel_strategy.starting_fuel = 80.0; // Too little for Monaco

        let simulator = create_simulator(circuit, strategy, DegradationFactors::default());
        let result = simulator.simulate_race();

        // Should have fuel warnings
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("fuel") || w.contains("Fuel")));
    }

    #[test]
    fn test_tire_age_warning() {
        let circuit = Circuit::silverstone();
        let mut strategy = create_test_strategy();

        // Remove pit stops to force tire age warning
        strategy.pit_stops = vec![
            PitStop {
                lap: LapNumber(10),
                compound: TireCompound::C5, // Soft tire, short life
                pit_loss: 22.0,
                reason: PitStopReason::Mandatory,
                confidence: 0.9,
            },
        ];

        let simulator = create_simulator(circuit, strategy, DegradationFactors::default());
        let result = simulator.simulate_race();

        // Should warn about tire age exceeding typical life
        assert!(result.warnings.iter().any(|w| w.contains("Tire age exceeded")));
    }

    #[test]
    fn test_temperature_effects() {
        let circuit = Circuit::monza();
        let strategy = create_test_strategy();
        let fuel_model = FuelConsumptionModel::default_model();

        // Test cold conditions
        let cold_weather = WeatherConditions {
            initial_condition: WeatherCondition::Cloudy,
            track_temperature: 15.0, // Cold
            air_temperature: 12.0,
            changes: vec![],
        };

        let cold_sim = RaceSimulator::new(
            circuit.clone(),
            strategy.clone(),
            fuel_model.clone(),
            cold_weather,
        );
        let cold_result = cold_sim.simulate_race();

        // Test hot conditions
        let hot_weather = WeatherConditions {
            initial_condition: WeatherCondition::Dry,
            track_temperature: 50.0, // Very hot
            air_temperature: 35.0,
            changes: vec![],
        };

        let hot_sim = RaceSimulator::new(circuit, strategy, fuel_model, hot_weather);
        let hot_result = hot_sim.simulate_race();

        // Both should complete but with different lap times
        assert!(cold_result.total_time > 0.0);
        assert!(hot_result.total_time > 0.0);

        // Extreme temperatures should result in slower times
        // (either too cold for grip or too hot for tire life)
        assert!(cold_result.average_lap_time != hot_result.average_lap_time);
    }

    #[test]
    fn test_create_simulator_helper() {
        let circuit = Circuit::suzuka();
        let strategy = create_test_strategy();
        let degradation = DegradationFactors::default();

        let simulator = create_simulator(circuit, strategy, degradation);
        let result = simulator.simulate_race();

        assert!(result.total_time > 0.0);
        assert!(!result.lap_times.is_empty());
    }

    #[test]
    fn test_weather_conditions_from_forecast() {
        let forecast = WeatherForecast {
            overall_condition: WeatherCondition::Dry,
            air_temperature: 22.0,
            track_temperature: 28.0,
            humidity: 0.6,
            wind_speed: 10.0,
            wind_direction: 180.0,
            rain_probability: 0.2,
            rainfall_intensity: 0.0,
            sector_conditions: vec![],
            predictions: vec![],
        };

        let weather = WeatherConditions::from_forecast(&forecast);

        assert_eq!(weather.initial_condition, WeatherCondition::Dry);
        assert_eq!(weather.track_temperature, 28.0);
        assert_eq!(weather.air_temperature, 22.0);
        assert!(weather.changes.is_empty());
    }
}
