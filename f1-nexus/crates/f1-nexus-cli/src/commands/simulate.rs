//! Race simulation command

use anyhow::Result;
use chrono;
use colored::*;
use f1_nexus_core::strategy::ErsMode;
use f1_nexus_core::*;
use f1_nexus_strategy::simulation::*;
use indicatif::ProgressBar;
use tracing::info;

pub async fn run(track: String, num_sims: u64) -> Result<()> {
    info!("Running race simulation for {}", track);
    println!("\n{}", "Running race simulation...".cyan());
    println!("Simulations: {}", num_sims.to_string().yellow());
    println!("Track: {}", track.yellow());

    // Create circuit
    let circuit = super::optimize::create_test_circuit(&track)?;

    // Create a basic race strategy
    let strategy = RaceStrategy {
        id: format!("sim-{}-{}", track, chrono::Utc::now().timestamp()),
        starting_compound: TireCompound::C3,
        pit_stops: vec![
            PitStop {
                lap: LapNumber(25),
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
            default_mode: ErsMode::Medium,
            lap_overrides: std::collections::BTreeMap::new(),
            overtake_laps: vec![],
        },
        expected_lap_times: std::collections::BTreeMap::new(),
        predicted_race_time: 0.0, // Will be updated by simulation
        confidence: 0.8,
        metadata: StrategyMetadata {
            generated_at: chrono::Utc::now(),
            num_simulations: 1,
            contributing_agents: vec!["cli-simulator".to_string()],
            version_hash: None,
            parent_strategy_id: None,
        },
    };

    // Create weather conditions
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
    let progress = ProgressBar::new(num_sims);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/blue} {pos}/{len}")
            .unwrap()
    );

    let mut total_time = 0.0;
    let mut lap_times = vec![];

    for i in 0..num_sims {
        progress.inc(1);

        // Run single simulation
        let result = simulator.simulate_race();

        // Collect statistics
        total_time += result.total_time;
        if i < 100 {
            // Only keep first 100 for detailed stats
            lap_times.extend(result.lap_times.clone());
        }
    }

    progress.finish();

    // Calculate statistics
    let mean_time = total_time / num_sims as f32;
    let fastest_lap = lap_times.iter().fold(f32::INFINITY, |a, b| a.min(*b));
    let slowest_lap = lap_times.iter().fold(0.0f32, |a, b| a.max(*b));

    // Display results
    println!("\n{}", "Simulation Results:".green().bold());
    println!("  Simulations Run: {}", num_sims);
    println!("  Mean Race Time: {:.1}s ({:.0} min)", mean_time, mean_time / 60.0);

    if !lap_times.is_empty() {
        let avg_lap = lap_times.iter().sum::<f32>() / lap_times.len() as f32;
        println!("\n{}", "Lap Time Statistics:".green());
        println!("  Average Lap Time: {:.3}s", avg_lap);
        println!("  Fastest Lap: {:.3}s", fastest_lap);
        println!("  Slowest Lap: {:.3}s", slowest_lap);
        println!("  Lap Time Range: {:.3}s", slowest_lap - fastest_lap);
    }

    // Run single detailed simulation
    println!("\n{}", "Sample Race Breakdown:".green());
    let sample = simulator.simulate_race();

    println!("  Total Laps: {}", sample.lap_times.len());
    println!("  Total Race Time: {:.1}s", sample.total_time);
    println!("  Pit Stops: {}", sample.pit_stops.len());

    println!("\n{}", "Fuel Management:".green());
    if let Some(&final_fuel) = sample.fuel_history.last() {
        println!("  Final Fuel: {:.1} kg", final_fuel);
        println!("  Fuel Used: {:.1} kg", 110.0 - final_fuel);
        println!("  Avg Consumption: {:.3} kg/lap",
            (110.0 - final_fuel) / sample.lap_times.len() as f32);
    }

    Ok(())
}

// Re-export for use in simulate command
pub use super::optimize::create_test_circuit;
