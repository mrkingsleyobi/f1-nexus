//! MCP tool implementations

use anyhow::Result;
use f1_nexus_core::*;
use f1_nexus_strategy::*;
use f1_nexus_strategy::simulation::*;
use serde_json::{json, Value};
use tracing::{info, warn};

/// Handle optimize_strategy tool call
pub fn handle_optimize_strategy(params: Value) -> Result<Value> {
    info!("MCP tool: optimize_strategy called");

    // Extract parameters
    let _current_lap = params["current_lap"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: current_lap"))?
        as u16;

    let _tire_age = params["tire_age"].as_u64().unwrap_or(0) as u16;
    let fuel_remaining = params["fuel_remaining"].as_f64().unwrap_or(110.0) as f32;
    let position = params["position"].as_u64().unwrap_or(1) as u8;
    let track_id = params["track_id"].as_str().unwrap_or("default");

    // Create a default circuit (in production, this would come from a database)
    let circuit = create_default_circuit(track_id);

    // Setup optimization configuration
    let config = OptimizationConfig {
        total_laps: circuit.typical_race_laps,
        circuit: circuit.clone(),
        available_compounds: vec![TireCompound::C1, TireCompound::C2, TireCompound::C3],
        pit_lane_time_loss: 20.0,
        tire_change_time: 2.5,
        current_position: position,
        competitors_ahead: vec![],
        degradation_factors: DegradationFactors {
            track_severity: circuit.characteristics.tire_severity,
            temperature_factor: 1.0,
            driving_style_factor: 1.0,
            fuel_load_factor: 1.0,
            downforce_factor: circuit.characteristics.downforce_level,
        },
        fuel_model: FuelConsumptionModel::default_model(),
        starting_fuel: fuel_remaining,
        min_pit_stops: 1,
        max_pit_stops: 3,
    };

    // Run optimization
    let strategy = optimize_pit_strategy(&config)
        .map_err(|e| anyhow::anyhow!("Optimization failed: {}", e))?;

    // Convert strategy to JSON
    Ok(json!({
        "success": true,
        "strategy": {
            "id": strategy.id,
            "starting_compound": format!("{:?}", strategy.starting_compound),
            "pit_stops": strategy.pit_stops.iter().map(|stop| {
                json!({
                    "lap": stop.lap.0,
                    "compound": format!("{:?}", stop.compound),
                    "pit_loss": stop.pit_loss,
                    "reason": format!("{:?}", stop.reason),
                    "confidence": stop.confidence,
                })
            }).collect::<Vec<_>>(),
            "predicted_race_time": strategy.predicted_race_time,
            "confidence": strategy.confidence,
        }
    }))
}

/// Handle predict_tire_life tool call
pub fn handle_predict_tire_life(params: Value) -> Result<Value> {
    info!("MCP tool: predict_tire_life called");

    // Extract parameters
    let compound_str = params["compound"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: compound"))?;

    let age_laps = params["age_laps"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: age_laps"))?
        as u16;

    let track_temp = params["track_temp"].as_f64().unwrap_or(100.0) as f32;
    let track_severity = params["track_severity"].as_f64().unwrap_or(1.0) as f32;

    // Parse compound
    let compound = match compound_str.to_uppercase().as_str() {
        "C0" => TireCompound::C0,
        "C1" => TireCompound::C1,
        "C2" => TireCompound::C2,
        "C3" => TireCompound::C3,
        "C4" => TireCompound::C4,
        "C5" => TireCompound::C5,
        "INTERMEDIATE" | "INT" => TireCompound::Intermediate,
        "WET" => TireCompound::Wet,
        _ => return Err(anyhow::anyhow!("Invalid tire compound: {}", compound_str)),
    };

    // Get tire characteristics
    use f1_nexus_core::tire::TireCharacteristics;
    let tire_chars = TireCharacteristics::for_compound(compound);

    // Calculate current wear based on age
    let current_wear = age_laps as f32 * tire_chars.degradation_rate * track_severity;
    let current_wear = current_wear.min(1.0); // Cap at 100%

    // Calculate grip multiplier based on temperature
    let grip_multiplier = tire_chars.grip_multiplier_for_temp(track_temp);

    // Predict remaining life
    let remaining_laps = tire_chars.predict_remaining_life(current_wear, track_severity);

    // Calculate degradation impact on lap time (simple estimate)
    // Assume 1% wear = ~0.05s lap time increase
    let lap_time_delta = current_wear * 5.0;

    Ok(json!({
        "success": true,
        "prediction": {
            "compound": compound_str,
            "current_age_laps": age_laps,
            "current_wear_percent": current_wear * 100.0,
            "typical_life_laps": tire_chars.typical_life,
            "estimated_remaining_laps": remaining_laps.min(tire_chars.typical_life as f32),
            "grip_multiplier": grip_multiplier,
            "lap_time_delta_seconds": lap_time_delta,
            "track_temperature": track_temp,
            "optimal_temp_range": tire_chars.optimal_temp_range,
            "recommended_pit_soon": current_wear > 0.7 || remaining_laps < 5.0,
        }
    }))
}

/// Handle simulate_race tool call
pub fn handle_simulate_race(params: Value) -> Result<Value> {
    info!("MCP tool: simulate_race called");

    let num_simulations = params["num_simulations"].as_u64().unwrap_or(100) as u64;
    let track_id = params["track_id"].as_str().unwrap_or("default");

    // Create circuit
    let circuit = create_default_circuit(track_id);

    // Create a basic strategy for simulation
    let strategy = RaceStrategy {
        id: format!("sim-{}", chrono::Utc::now().timestamp()),
        starting_compound: TireCompound::C3,
        pit_stops: vec![
            PitStop {
                lap: LapNumber(circuit.typical_race_laps / 2),
                compound: TireCompound::C2,
                pit_loss: 22.0,
                reason: PitStopReason::Mandatory,
                confidence: 0.85,
            },
        ],
        fuel_strategy: FuelStrategy {
            starting_fuel: 110.0,
            fuel_saving_per_lap: 0.0,
            fuel_saving_laps: vec![],
            minimum_buffer: 3.0,
        },
        ers_plan: ErsDeploymentPlan {
            default_mode: f1_nexus_core::strategy::ErsMode::Medium,
            lap_overrides: std::collections::BTreeMap::new(),
            overtake_laps: vec![],
        },
        expected_lap_times: std::collections::BTreeMap::new(),
        predicted_race_time: 0.0,
        confidence: 0.8,
        metadata: StrategyMetadata {
            generated_at: chrono::Utc::now(),
            num_simulations: num_simulations,
            contributing_agents: vec!["mcp-server".to_string()],
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
        circuit.clone(),
        strategy,
        FuelConsumptionModel::default_model(),
        weather,
    );

    // Run simulations
    let mut total_time = 0.0_f32;
    let mut min_time = f32::INFINITY;
    let mut max_time = 0.0_f32;

    for _ in 0..num_simulations {
        let result = simulator.simulate_race();
        total_time += result.total_time;
        min_time = min_time.min(result.total_time);
        max_time = max_time.max(result.total_time);
    }

    let mean_time = total_time / num_simulations as f32;

    // Run one detailed simulation for breakdown
    let sample = simulator.simulate_race();

    Ok(json!({
        "success": true,
        "simulation": {
            "num_simulations": num_simulations,
            "mean_race_time_seconds": mean_time,
            "min_race_time_seconds": min_time,
            "max_race_time_seconds": max_time,
            "mean_race_time_formatted": format_time(mean_time),
            "sample_breakdown": {
                "total_laps": sample.lap_times.len(),
                "pit_stops": sample.pit_stops.len(),
                "final_fuel_kg": sample.fuel_history.last().copied().unwrap_or(0.0),
                "fastest_lap_seconds": sample.lap_times.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
            }
        }
    }))
}

/// Handle query_historical tool call
pub fn handle_query_historical(params: Value) -> Result<Value> {
    info!("MCP tool: query_historical called");

    let track_id = params["track_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: track_id"))?;

    let weather = params["weather"].as_str().unwrap_or("dry");
    let top_k = params["top_k"].as_u64().unwrap_or(5) as usize;

    warn!("Historical data query not yet implemented - returning mock data");

    // TODO: Implement vector database integration
    // For now, return mock historical data
    Ok(json!({
        "success": true,
        "note": "Historical data integration pending - showing mock data",
        "query": {
            "track_id": track_id,
            "weather": weather,
            "top_k": top_k,
        },
        "results": [
            {
                "race_id": "2023_monaco_gp",
                "year": 2023,
                "winner_strategy": "2-stop (C3→C2→C1)",
                "winning_time_seconds": 5643.2,
                "similarity_score": 0.95,
            },
            {
                "race_id": "2022_monaco_gp",
                "year": 2022,
                "winner_strategy": "1-stop (C4→C3)",
                "winning_time_seconds": 5712.8,
                "similarity_score": 0.89,
            },
        ]
    }))
}

/// Handle get_agent_consensus tool call
pub fn handle_get_agent_consensus(params: Value) -> Result<Value> {
    info!("MCP tool: get_agent_consensus called");

    let question = params["question"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: question"))?;

    let timeout_ms = params["timeout_ms"].as_u64().unwrap_or(5000);

    warn!("Multi-agent consensus not yet implemented - returning single agent response");

    // TODO: Implement multi-agent system integration
    // For now, return single agent response
    Ok(json!({
        "success": true,
        "note": "Multi-agent system pending - showing single agent response",
        "question": question,
        "timeout_ms": timeout_ms,
        "consensus": {
            "agreement_level": 0.85,
            "num_agents": 1,
            "recommendation": "Based on current conditions, recommend 2-stop strategy",
            "reasoning": "Single agent analysis suggests optimal pit window at laps 20 and 40",
            "confidence": 0.80,
        }
    }))
}

/// Helper: Create default circuit based on track ID
fn create_default_circuit(track_id: &str) -> Circuit {
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
        "spa" => Circuit {
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

/// Helper: Format time as MM:SS.mmm
fn format_time(seconds: f32) -> String {
    let minutes = (seconds / 60.0) as u32;
    let secs = seconds % 60.0;
    format!("{}:{:06.3}", minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_optimize_strategy_handler() {
        let params = json!({
            "current_lap": 20,
            "tire_age": 15,
            "fuel_remaining": 80.0,
            "position": 3,
            "track_id": "monaco"
        });

        let result = handle_optimize_strategy(params);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert!(response["strategy"]["pit_stops"].is_array());
    }

    #[test]
    fn test_predict_tire_life_handler() {
        let params = json!({
            "compound": "C3",
            "age_laps": 15,
            "track_temp": 105.0
        });

        let result = handle_predict_tire_life(params);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert!(response["prediction"]["estimated_remaining_laps"].is_number());
        assert!(response["prediction"]["current_wear_percent"].is_number());
    }

    #[test]
    fn test_simulate_race_handler() {
        let params = json!({
            "num_simulations": 10,
            "track_id": "spa"
        });

        let result = handle_simulate_race(params);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert!(response["simulation"]["mean_race_time_seconds"].is_number());
    }
}
