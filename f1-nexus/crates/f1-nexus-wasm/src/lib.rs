//! F1 Nexus WASM Module
//!
//! Browser-based strategy optimization, simulation, and visualization

use wasm_bindgen::prelude::*;
use f1_nexus_core::*;
use f1_nexus_strategy::*;
use f1_nexus_strategy::simulation::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    log("F1 Nexus WASM module initialized v1.0.0-alpha.1");
}

/// F1 Nexus WASM API
#[wasm_bindgen]
pub struct F1Nexus;

#[wasm_bindgen]
impl F1Nexus {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        F1Nexus
    }

    /// Get version
    #[wasm_bindgen]
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Optimize pit stop strategy
    ///
    /// Input format (JSON):
    /// ```json
    /// {
    ///   "track": "monaco",
    ///   "total_laps": 78,
    ///   "starting_fuel": 110.0,
    ///   "position": 5,
    ///   "available_compounds": ["C1", "C2", "C3"]
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn optimize_strategy(&self, params: JsValue) -> Result<JsValue, JsValue> {
        let input: OptimizeInput = serde_wasm_bindgen::from_value(params)
            .map_err(|e| JsValue::from_str(&format!("Invalid input: {}", e)))?;

        // Create circuit
        let circuit = create_circuit(&input.track);

        // Parse tire compounds
        let available_compounds: Vec<TireCompound> = input
            .available_compounds
            .unwrap_or_else(|| vec!["C1".to_string(), "C2".to_string(), "C3".to_string()])
            .iter()
            .filter_map(|s| parse_tire_compound(s))
            .collect();

        if available_compounds.is_empty() {
            return Err(JsValue::from_str("No valid tire compounds provided"));
        }

        // Setup optimization config
        let config = OptimizationConfig {
            total_laps: input.total_laps.unwrap_or(circuit.typical_race_laps),
            circuit: circuit.clone(),
            available_compounds,
            pit_lane_time_loss: 20.0,
            tire_change_time: 2.5,
            current_position: input.position.unwrap_or(5) as u8,
            competitors_ahead: vec![],
            degradation_factors: DegradationFactors::default(),
            fuel_model: FuelConsumptionModel::default_model(),
            starting_fuel: input.starting_fuel.unwrap_or(110.0),
            min_pit_stops: 1,
            max_pit_stops: 3,
        };

        // Optimize strategy
        let strategy = optimize_pit_strategy(&config)
            .map_err(|e| JsValue::from_str(&format!("Optimization failed: {}", e)))?;

        // Convert to JSON-friendly format
        let output = OptimizeOutput {
            strategy_id: strategy.id,
            starting_compound: format!("{:?}", strategy.starting_compound),
            pit_stops: strategy.pit_stops.iter().map(|stop| PitStopOutput {
                lap: stop.lap.0,
                compound: format!("{:?}", stop.compound),
                pit_loss: stop.pit_loss,
                reason: format!("{:?}", stop.reason),
                confidence: stop.confidence,
            }).collect(),
            predicted_race_time: strategy.predicted_race_time,
            confidence: strategy.confidence,
        };

        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Simulate race with given strategy
    ///
    /// Input format (JSON):
    /// ```json
    /// {
    ///   "track": "spa",
    ///   "num_simulations": 100,
    ///   "pit_stops": [{"lap": 22, "compound": "C2"}]
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn simulate_race(&self, params: JsValue) -> Result<JsValue, JsValue> {
        let input: SimulateInput = serde_wasm_bindgen::from_value(params)
            .map_err(|e| JsValue::from_str(&format!("Invalid input: {}", e)))?;

        // Create circuit
        let circuit = create_circuit(&input.track);

        // Parse pit stops
        let pit_stops: Vec<PitStop> = input.pit_stops.iter().map(|stop| {
            let compound = parse_tire_compound(&stop.compound).unwrap_or(TireCompound::C3);
            PitStop {
                lap: LapNumber(stop.lap),
                compound,
                pit_loss: 22.0,
                reason: PitStopReason::Mandatory,
                confidence: 0.85,
            }
        }).collect();

        // Create strategy
        let strategy = RaceStrategy {
            id: format!("wasm-sim-{}", chrono::Utc::now().timestamp()),
            starting_compound: parse_tire_compound(&input.starting_compound.unwrap_or("C3".to_string()))
                .unwrap_or(TireCompound::C3),
            pit_stops,
            fuel_strategy: FuelStrategy {
                starting_fuel: 110.0,
                fuel_saving_per_lap: 0.0,
                fuel_saving_laps: vec![],
                minimum_buffer: 3.0,
            },
            ers_plan: ErsDeploymentPlan {
                default_mode: f1_nexus_core::strategy::ErsMode::Medium,
                lap_overrides: BTreeMap::new(),
                overtake_laps: vec![],
            },
            expected_lap_times: BTreeMap::new(),
            predicted_race_time: 0.0,
            confidence: 0.8,
            metadata: StrategyMetadata {
                generated_at: chrono::Utc::now(),
                num_simulations: input.num_simulations.unwrap_or(100),
                contributing_agents: vec!["wasm".to_string()],
                version_hash: None,
                parent_strategy_id: None,
            },
        };

        // Create weather
        let weather = WeatherConditions {
            initial_condition: WeatherCondition::Dry,
            track_temperature: 30.0,
            air_temperature: 25.0,
            changes: vec![],
        };

        // Create simulator
        let simulator = RaceSimulator::new(
            circuit,
            strategy,
            FuelConsumptionModel::default_model(),
            weather,
        );

        // Run simulations
        let num_sims = input.num_simulations.unwrap_or(100);
        let mut total_time = 0.0_f32;
        let mut min_time = f32::INFINITY;
        let mut max_time = 0.0_f32;

        for _ in 0..num_sims {
            let result = simulator.simulate_race();
            total_time += result.total_time;
            min_time = min_time.min(result.total_time);
            max_time = max_time.max(result.total_time);
        }

        let mean_time = total_time / num_sims as f32;

        // Run one detailed simulation
        let sample = simulator.simulate_race();

        let output = SimulateOutput {
            num_simulations: num_sims,
            mean_race_time: mean_time,
            min_race_time: min_time,
            max_race_time: max_time,
            total_laps: sample.lap_times.len() as u16,
            pit_stops: sample.pit_stops.len() as u8,
            final_fuel_kg: sample.fuel_history.last().copied().unwrap_or(0.0),
            fastest_lap: sample.lap_times.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
        };

        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Predict tire life
    ///
    /// Input format (JSON):
    /// ```json
    /// {
    ///   "compound": "C3",
    ///   "age_laps": 15,
    ///   "track_temp": 32.0,
    ///   "track_severity": 1.2
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn predict_tire_life(&self, params: JsValue) -> Result<JsValue, JsValue> {
        let input: TireLifeInput = serde_wasm_bindgen::from_value(params)
            .map_err(|e| JsValue::from_str(&format!("Invalid input: {}", e)))?;

        let compound = parse_tire_compound(&input.compound)
            .ok_or_else(|| JsValue::from_str("Invalid tire compound"))?;

        let tire_chars = f1_nexus_core::tire::TireCharacteristics::for_compound(compound);

        let track_severity = input.track_severity.unwrap_or(1.0);
        let current_wear = input.age_laps as f32 * tire_chars.degradation_rate * track_severity;
        let current_wear = current_wear.min(1.0);

        let grip_multiplier = tire_chars.grip_multiplier_for_temp(input.track_temp.unwrap_or(100.0));
        let remaining_laps = tire_chars.predict_remaining_life(current_wear, track_severity);

        let output = TireLifeOutput {
            compound: input.compound,
            current_age_laps: input.age_laps,
            current_wear_percent: current_wear * 100.0,
            typical_life_laps: tire_chars.typical_life,
            estimated_remaining_laps: remaining_laps.min(tire_chars.typical_life as f32),
            grip_multiplier,
            recommended_pit_soon: current_wear > 0.7 || remaining_laps < 5.0,
        };

        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get list of supported circuits
    #[wasm_bindgen]
    pub fn get_circuits(&self) -> JsValue {
        let circuits = vec![
            "monaco", "spa", "silverstone", "monza", "suzuka",
            "interlagos", "austin", "barcelona", "austria", "hungary",
        ];
        serde_wasm_bindgen::to_value(&circuits).unwrap()
    }

    /// Get list of tire compounds
    #[wasm_bindgen]
    pub fn get_tire_compounds(&self) -> JsValue {
        let compounds = vec!["C0", "C1", "C2", "C3", "C4", "C5", "Intermediate", "Wet"];
        serde_wasm_bindgen::to_value(&compounds).unwrap()
    }
}

impl Default for F1Nexus {
    fn default() -> Self {
        Self::new()
    }
}

// Input/Output types

#[derive(Deserialize)]
struct OptimizeInput {
    track: String,
    total_laps: Option<u16>,
    starting_fuel: Option<f32>,
    position: Option<u16>,
    available_compounds: Option<Vec<String>>,
}

#[derive(Serialize)]
struct OptimizeOutput {
    strategy_id: String,
    starting_compound: String,
    pit_stops: Vec<PitStopOutput>,
    predicted_race_time: f32,
    confidence: f32,
}

#[derive(Serialize)]
struct PitStopOutput {
    lap: u16,
    compound: String,
    pit_loss: f32,
    reason: String,
    confidence: f32,
}

#[derive(Deserialize)]
struct SimulateInput {
    track: String,
    num_simulations: Option<u64>,
    starting_compound: Option<String>,
    pit_stops: Vec<PitStopInput>,
}

#[derive(Deserialize)]
struct PitStopInput {
    lap: u16,
    compound: String,
}

#[derive(Serialize)]
struct SimulateOutput {
    num_simulations: u64,
    mean_race_time: f32,
    min_race_time: f32,
    max_race_time: f32,
    total_laps: u16,
    pit_stops: u8,
    final_fuel_kg: f32,
    fastest_lap: f32,
}

#[derive(Deserialize)]
struct TireLifeInput {
    compound: String,
    age_laps: u16,
    track_temp: Option<f32>,
    track_severity: Option<f32>,
}

#[derive(Serialize)]
struct TireLifeOutput {
    compound: String,
    current_age_laps: u16,
    current_wear_percent: f32,
    typical_life_laps: u16,
    estimated_remaining_laps: f32,
    grip_multiplier: f32,
    recommended_pit_soon: bool,
}

// Helper functions

fn parse_tire_compound(s: &str) -> Option<TireCompound> {
    match s.to_uppercase().as_str() {
        "C0" => Some(TireCompound::C0),
        "C1" => Some(TireCompound::C1),
        "C2" => Some(TireCompound::C2),
        "C3" => Some(TireCompound::C3),
        "C4" => Some(TireCompound::C4),
        "C5" => Some(TireCompound::C5),
        "INTERMEDIATE" | "INT" => Some(TireCompound::Intermediate),
        "WET" => Some(TireCompound::Wet),
        _ => None,
    }
}

fn create_circuit(track_id: &str) -> Circuit {
    match track_id.to_lowercase().as_str() {
        "monaco" => Circuit {
            id: "monaco".to_string(),
            name: "Circuit de Monaco".to_string(),
            country: "Monaco".to_string(),
            length: 3337.0,
            num_turns: 19,
            lap_record: 70.0,
            characteristics: TrackCharacteristics {
                tire_severity: 1.2,
                fuel_consumption: 0.9,
                overtaking_difficulty: 0.95,
                downforce_level: 0.9,
                average_speed: 160.0,
                maximum_speed: 290.0,
                elevation_change: 42.0,
                weather_variability: 0.3,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 78,
        },
        "spa" | "spa-francorchamps" => Circuit {
            id: "spa".to_string(),
            name: "Circuit de Spa-Francorchamps".to_string(),
            country: "Belgium".to_string(),
            length: 7004.0,
            num_turns: 19,
            lap_record: 103.0,
            characteristics: TrackCharacteristics {
                tire_severity: 0.85,
                fuel_consumption: 1.3,
                overtaking_difficulty: 0.6,
                downforce_level: 0.65,
                average_speed: 230.0,
                maximum_speed: 340.0,
                elevation_change: 105.0,
                weather_variability: 0.8,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 44,
        },
        "silverstone" => Circuit {
            id: "silverstone".to_string(),
            name: "Silverstone Circuit".to_string(),
            country: "United Kingdom".to_string(),
            length: 5891.0,
            num_turns: 18,
            lap_record: 85.0,
            characteristics: TrackCharacteristics {
                tire_severity: 1.1,
                fuel_consumption: 1.1,
                overtaking_difficulty: 0.65,
                downforce_level: 0.75,
                average_speed: 240.0,
                maximum_speed: 320.0,
                elevation_change: 18.0,
                weather_variability: 0.7,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 52,
        },
        _ => Circuit {
            id: track_id.to_string(),
            name: format!("Circuit {}", track_id),
            country: "Unknown".to_string(),
            length: 5000.0,
            num_turns: 16,
            lap_record: 90.0,
            characteristics: TrackCharacteristics {
                tire_severity: 1.0,
                fuel_consumption: 1.0,
                overtaking_difficulty: 0.7,
                downforce_level: 0.7,
                average_speed: 210.0,
                maximum_speed: 310.0,
                elevation_change: 20.0,
                weather_variability: 0.5,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 60,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_version() {
        let nexus = F1Nexus::new();
        let version = nexus.version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_parse_tire_compound() {
        assert_eq!(parse_tire_compound("C3"), Some(TireCompound::C3));
        assert_eq!(parse_tire_compound("c3"), Some(TireCompound::C3));
        assert_eq!(parse_tire_compound("intermediate"), Some(TireCompound::Intermediate));
        assert_eq!(parse_tire_compound("invalid"), None);
    }

    #[test]
    fn test_create_circuit() {
        let monaco = create_circuit("monaco");
        assert_eq!(monaco.id, "monaco");
        assert_eq!(monaco.typical_race_laps, 78);

        let spa = create_circuit("spa");
        assert_eq!(spa.id, "spa");
        assert_eq!(spa.typical_race_laps, 44);
    }
}
