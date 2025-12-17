//! Strategy optimization command

use anyhow::Result;
use colored::*;
use f1_nexus_core::*;
use f1_nexus_strategy::*;
use indicatif::ProgressBar;
use tracing::info;

pub async fn run(track: String, lap: Option<u16>, strategy_type: String) -> Result<()> {
    info!("Optimizing strategy for track: {}", track);
    println!("\n{}", "Running strategy optimization...".cyan());
    println!("Track: {}", track.yellow());
    println!("Current Lap: {}", lap.unwrap_or(1).to_string().yellow());
    println!("Strategy Type: {}", strategy_type.yellow());

    // Create circuit configuration
    let circuit = create_test_circuit(&track)?;

    // Setup optimization config
    let config = OptimizationConfig {
        total_laps: circuit.typical_race_laps,
        circuit: circuit.clone(),
        available_compounds: vec![TireCompound::C1, TireCompound::C2, TireCompound::C3],
        pit_lane_time_loss: 20.0,
        tire_change_time: 2.5,
        current_position: 1,
        competitors_ahead: vec![],
        degradation_factors: DegradationFactors {
            track_severity: circuit.characteristics.tire_severity,
            temperature_factor: 1.0,
            driving_style_factor: 1.0,
            fuel_load_factor: 1.0,
            downforce_factor: circuit.characteristics.downforce_level,
        },
        fuel_model: FuelConsumptionModel::default_model(),
        starting_fuel: 110.0,
        min_pit_stops: 1,
        max_pit_stops: 3,
    };

    // Show progress bar
    let progress = ProgressBar::new(100);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
    );

    // Run optimization
    progress.set_message("Analyzing tire strategies...");
    let strategy = tokio::task::spawn_blocking(move || {
        optimize_pit_strategy(&config)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
    .map_err(|e| anyhow::anyhow!("Optimization error: {}", e))?;

    progress.finish_with_message("Optimization complete!");

    // Display results
    println!("\n{}", "Optimal Strategy:".green().bold());

    println!("\n{}", "Pit Stops:".green());
    for (i, stop) in strategy.pit_stops.iter().enumerate() {
        println!("  {}. Lap {} → {}",
            i + 1,
            stop.lap.0,
            format_compound(stop.compound),
        );
        println!("     Reason: {:?}, Confidence: {:.1}%", stop.reason, stop.confidence * 100.0);
    }

    println!("\n{}", "Tire Strategy:".green());
    println!("  Starting Compound: {}", format_compound(strategy.starting_compound));
    println!("  Total Pit Stops: {}", strategy.pit_stops.len());

    println!("\n{}", "Fuel Strategy:".green());
    println!("  Starting Fuel: {:.1} kg", strategy.fuel_strategy.starting_fuel);
    if !strategy.fuel_strategy.fuel_saving_laps.is_empty() {
        println!("  Fuel Saving Laps: {:?}", strategy.fuel_strategy.fuel_saving_laps);
    }

    Ok(())
}

pub fn create_test_circuit(track_id: &str) -> Result<Circuit> {
    // Create sample circuits for common tracks
    let circuit = match track_id.to_lowercase().as_str() {
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
        _ => {
            // Default circuit
            Circuit {
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
            }
        }
    };

    Ok(circuit)
}

fn format_compound(compound: TireCompound) -> ColoredString {
    match compound {
        TireCompound::C5 => "C5 (Soft)".red(),
        TireCompound::C4 => "C4 (Medium-Soft)".yellow(),
        TireCompound::C3 => "C3 (Medium)".bright_yellow(),
        TireCompound::C2 => "C2 (Medium-Hard)".white(),
        TireCompound::C1 => "C1 (Hard)".bright_white(),
        TireCompound::C0 => "C0 (Super-Hard)".bright_black(),
        TireCompound::Intermediate => "Intermediate".green(),
        TireCompound::Wet => "Wet".blue(),
    }
}
