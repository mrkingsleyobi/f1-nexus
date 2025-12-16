//! F1 Nexus Strategy - Pit Stop Optimization and Race Strategy
//!
//! This crate provides advanced pit stop strategy optimization using dynamic programming,
//! tire degradation modeling, fuel consumption analysis, and competitor strategy simulation.

use f1_nexus_core::{
    Circuit, FuelConsumptionModel, LapNumber, PitStop, PitStopReason, RaceStrategy,
    StintNumber, TireCharacteristics, TireCompound, DegradationFactors,
    FuelStrategy, ErsDeploymentPlan, StrategyMetadata,
};
use f1_nexus_core::strategy::ErsMode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Pit stop optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Total race laps
    pub total_laps: u16,

    /// Circuit being raced on
    pub circuit: Circuit,

    /// Available tire compounds for the race
    pub available_compounds: Vec<TireCompound>,

    /// Pit lane time loss (seconds)
    pub pit_lane_time_loss: f32,

    /// Tire change time (seconds)
    pub tire_change_time: f32,

    /// Current track position
    pub current_position: u8,

    /// Number of competitors ahead
    pub competitors_ahead: Vec<CompetitorState>,

    /// Degradation factors for this race
    pub degradation_factors: DegradationFactors,

    /// Fuel consumption model
    pub fuel_model: FuelConsumptionModel,

    /// Starting fuel load (kg)
    pub starting_fuel: f32,

    /// Minimum number of pit stops required (regulations)
    pub min_pit_stops: u8,

    /// Maximum number of pit stops to consider
    pub max_pit_stops: u8,
}

/// Competitor state for undercut/overcut analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorState {
    pub position: u8,
    pub current_lap: u16,
    pub current_compound: TireCompound,
    pub tire_age: u16,
    pub estimated_pit_lap: Option<u16>,
    pub gap_seconds: f32,
}

/// Dynamic programming state for optimization
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DPState {
    /// Best time to this state
    best_time: f32,

    /// Pit stops taken to reach this state
    pit_stops: Vec<PitStop>,

    /// Number of pit stops
    num_stops: u8,

    /// Last compound used
    last_compound: TireCompound,
}

/// Pit window constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitWindow {
    /// Earliest feasible pit lap
    pub earliest_lap: u16,

    /// Latest feasible pit lap
    pub latest_lap: u16,

    /// Optimal window start
    pub optimal_start: u16,

    /// Optimal window end
    pub optimal_end: u16,

    /// Reason for window constraints
    pub constraints: Vec<String>,
}

/// Strategy comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyComparison {
    pub strategy_a: RaceStrategy,
    pub strategy_b: RaceStrategy,

    /// Time difference (seconds, positive = A is faster)
    pub time_delta: f32,

    /// Risk score difference (positive = A is riskier)
    pub risk_delta: f32,

    /// Detailed comparison breakdown
    pub breakdown: ComparisonBreakdown,
}

/// Detailed comparison breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonBreakdown {
    pub tire_wear_difference: f32,
    pub fuel_efficiency_difference: f32,
    pub pit_loss_difference: f32,
    pub track_position_risk: f32,
    pub overtaking_opportunities: i8,
}

/// Optimize pit stop strategy using dynamic programming
///
/// This function finds the optimal pit stop strategy considering:
/// - Tire degradation and compound characteristics
/// - Fuel consumption and weight impact
/// - Pit lane time loss
/// - Track position and competitor strategies
/// - FIA regulations (minimum pit stops, compound requirements)
pub fn optimize_pit_strategy(config: &OptimizationConfig) -> Result<RaceStrategy, String> {
    // Validate configuration
    validate_config(config)?;

    // Initialize DP table: dp[lap][num_stops][compound] = best state
    let mut dp: HashMap<(u16, u8, TireCompound), DPState> = HashMap::new();

    // Base case: start of race with starting compound
    let starting_compound = config.available_compounds[0];
    dp.insert((1, 0, starting_compound), DPState {
        best_time: 0.0,
        pit_stops: vec![],
        num_stops: 0,
        last_compound: starting_compound,
    });

    // Dynamic programming: iterate through all laps
    for lap in 1..=config.total_laps {
        // Try all possible current states
        for num_stops in 0..=config.max_pit_stops {
            for &compound in &config.available_compounds {
                let state_key = (lap, num_stops, compound);

                if let Some(current_state) = dp.get(&state_key).cloned() {
                    // Option 1: Continue without pitting
                    if lap < config.total_laps {
                        let tire_age = calculate_tire_age(lap, &current_state.pit_stops);
                        let lap_time = calculate_lap_time(
                            compound,
                            tire_age,
                            config,
                            lap,
                        );

                        let next_key = (lap + 1, num_stops, compound);
                        let next_time = current_state.best_time + lap_time;

                        update_dp_state(&mut dp, next_key, DPState {
                            best_time: next_time,
                            pit_stops: current_state.pit_stops.clone(),
                            num_stops,
                            last_compound: compound,
                        });
                    }

                    // Option 2: Pit for a different compound
                    if num_stops < config.max_pit_stops && lap < config.total_laps {
                        let pit_window = calculate_pit_window(
                            lap,
                            compound,
                            config,
                        );

                        if lap >= pit_window.earliest_lap && lap <= pit_window.latest_lap {
                            for &new_compound in &config.available_compounds {
                                // Must use different compound (regulations)
                                if new_compound != compound {
                                    let pit_loss = estimate_time_loss(config, lap);
                                    let tire_age = calculate_tire_age(lap, &current_state.pit_stops);
                                    let lap_time = calculate_lap_time(
                                        compound,
                                        tire_age,
                                        config,
                                        lap,
                                    );

                                    let next_key = (lap + 1, num_stops + 1, new_compound);
                                    let next_time = current_state.best_time + lap_time + pit_loss;

                                    let mut new_pit_stops = current_state.pit_stops.clone();
                                    new_pit_stops.push(PitStop {
                                        lap: LapNumber(lap),
                                        compound: new_compound,
                                        pit_loss,
                                        reason: determine_pit_reason(lap, config, &new_pit_stops),
                                        confidence: 0.85,
                                    });

                                    update_dp_state(&mut dp, next_key, DPState {
                                        best_time: next_time,
                                        pit_stops: new_pit_stops,
                                        num_stops: num_stops + 1,
                                        last_compound: new_compound,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Find best final state (must meet minimum pit stop requirement)
    let mut best_strategy: Option<DPState> = None;
    let mut best_time = f32::INFINITY;

    for num_stops in config.min_pit_stops..=config.max_pit_stops {
        for &compound in &config.available_compounds {
            let final_key = (config.total_laps, num_stops, compound);

            if let Some(state) = dp.get(&final_key) {
                if state.best_time < best_time && is_valid_strategy(&state.pit_stops, config) {
                    best_time = state.best_time;
                    best_strategy = Some(state.clone());
                }
            }
        }
    }

    match best_strategy {
        Some(strategy) => {
            // Build expected lap times
            let expected_lap_times = calculate_expected_lap_times(&strategy.pit_stops, config);

            Ok(RaceStrategy {
                id: uuid::Uuid::new_v4().to_string(),
                starting_compound,
                pit_stops: strategy.pit_stops,
                fuel_strategy: FuelStrategy {
                    starting_fuel: config.starting_fuel,
                    fuel_saving_per_lap: 0.0,
                    fuel_saving_laps: vec![],
                    minimum_buffer: 1.0,
                },
                ers_plan: ErsDeploymentPlan {
                    default_mode: ErsMode::Medium,
                    lap_overrides: BTreeMap::new(),
                    overtake_laps: vec![],
                },
                expected_lap_times,
                predicted_race_time: best_time,
                confidence: 0.80,
                metadata: StrategyMetadata {
                    generated_at: chrono::Utc::now(),
                    num_simulations: 1,
                    contributing_agents: vec!["pit-strategy-optimizer".to_string()],
                    version_hash: None,
                    parent_strategy_id: None,
                },
            })
        }
        None => Err("No valid strategy found within constraints".to_string()),
    }
}

/// Calculate valid pit window for current race state
pub fn calculate_pit_window(
    current_lap: u16,
    current_compound: TireCompound,
    config: &OptimizationConfig,
) -> PitWindow {
    let tire_chars = TireCharacteristics::for_compound(current_compound);
    let track_severity = config.circuit.characteristics.tire_severity;

    // Calculate tire life with degradation factors
    let adjusted_life = (tire_chars.typical_life as f32
        / config.degradation_factors.total_multiplier()) as u16;

    // Earliest: when tire starts to degrade significantly (70% life)
    let earliest_lap = current_lap + (adjusted_life as f32 * 0.7) as u16;

    // Latest: before tire is completely worn (95% life)
    let latest_lap = current_lap + (adjusted_life as f32 * 0.95) as u16;

    // Optimal window: 80-90% of tire life
    let optimal_start = current_lap + (adjusted_life as f32 * 0.80) as u16;
    let optimal_end = current_lap + (adjusted_life as f32 * 0.90) as u16;

    let mut constraints = vec![];

    // Check for strategic constraints
    if track_severity > 1.2 {
        constraints.push("High tire degradation track".to_string());
    }

    // Check competitor positions for undercut opportunities
    for competitor in &config.competitors_ahead {
        if let Some(comp_pit_lap) = competitor.estimated_pit_lap {
            if comp_pit_lap >= earliest_lap && comp_pit_lap <= latest_lap {
                constraints.push(format!(
                    "Potential undercut opportunity on P{} at lap {}",
                    competitor.position, comp_pit_lap - 1
                ));
            }
        }
    }

    PitWindow {
        earliest_lap: earliest_lap.min(config.total_laps - 1),
        latest_lap: latest_lap.min(config.total_laps - 1),
        optimal_start: optimal_start.min(config.total_laps - 1),
        optimal_end: optimal_end.min(config.total_laps - 1),
        constraints,
    }
}

/// Estimate time loss for a pit stop on a given lap
pub fn estimate_time_loss(config: &OptimizationConfig, lap: u16) -> f32 {
    // Base pit loss = pit lane time + tire change time
    let base_loss = config.pit_lane_time_loss + config.tire_change_time;

    // Fuel load impact: lighter car = less time lost in acceleration
    // Early in race (heavy car) = more time lost, late in race (light car) = less time lost
    let laps_completed_ratio = lap as f32 / config.total_laps as f32;
    let fuel_factor = 1.0 + (1.0 - laps_completed_ratio) * 0.1; // 0-10% increase based on fuel

    // Track position impact: risk of losing positions (constant per pit stop)
    let position_penalty = if config.current_position <= 3 {
        1.5 // Higher risk in top positions
    } else if config.current_position <= 10 {
        1.0 // Moderate risk in points positions
    } else {
        0.5 // Lower risk outside points
    };

    base_loss * fuel_factor + position_penalty
}

/// Compare two race strategies
pub fn compare_strategies(
    strategy_a: &RaceStrategy,
    strategy_b: &RaceStrategy,
    config: &OptimizationConfig,
) -> StrategyComparison {
    let time_delta = strategy_a.predicted_race_time - strategy_b.predicted_race_time;

    // Calculate risk scores
    let risk_a = calculate_strategy_risk(strategy_a, config);
    let risk_b = calculate_strategy_risk(strategy_b, config);
    let risk_delta = risk_a - risk_b;

    // Detailed breakdown
    let tire_wear_a = estimate_total_tire_wear(strategy_a, config);
    let tire_wear_b = estimate_total_tire_wear(strategy_b, config);

    let fuel_efficiency_a = estimate_fuel_efficiency(strategy_a, config);
    let fuel_efficiency_b = estimate_fuel_efficiency(strategy_b, config);

    let pit_loss_a = strategy_a.total_pit_loss();
    let pit_loss_b = strategy_b.total_pit_loss();

    let breakdown = ComparisonBreakdown {
        tire_wear_difference: tire_wear_a - tire_wear_b,
        fuel_efficiency_difference: fuel_efficiency_a - fuel_efficiency_b,
        pit_loss_difference: pit_loss_a - pit_loss_b,
        track_position_risk: risk_delta,
        overtaking_opportunities: count_overtaking_opportunities(strategy_a, config)
            - count_overtaking_opportunities(strategy_b, config),
    };

    StrategyComparison {
        strategy_a: strategy_a.clone(),
        strategy_b: strategy_b.clone(),
        time_delta,
        risk_delta,
        breakdown,
    }
}

// Helper functions

fn validate_config(config: &OptimizationConfig) -> Result<(), String> {
    if config.total_laps == 0 {
        return Err("Total laps must be greater than 0".to_string());
    }

    if config.available_compounds.is_empty() {
        return Err("Must have at least one available compound".to_string());
    }

    if config.min_pit_stops > config.max_pit_stops {
        return Err("Min pit stops cannot exceed max pit stops".to_string());
    }

    Ok(())
}

fn calculate_tire_age(current_lap: u16, pit_stops: &[PitStop]) -> u16 {
    // Find the most recent pit stop before current lap
    let last_pit_lap = pit_stops
        .iter()
        .filter(|ps| ps.lap.0 < current_lap)
        .map(|ps| ps.lap.0)
        .max()
        .unwrap_or(0);

    current_lap - last_pit_lap
}

fn calculate_lap_time(
    compound: TireCompound,
    tire_age: u16,
    config: &OptimizationConfig,
    lap: u16,
) -> f32 {
    let tire_chars = TireCharacteristics::for_compound(compound);

    // Base lap time from circuit characteristics
    let base_time = config.circuit.lap_record * 1.03; // 3% slower than lap record

    // Tire degradation impact
    let wear_ratio = tire_age as f32 / tire_chars.typical_life as f32;
    let degradation_multiplier = config.degradation_factors.total_multiplier();
    let wear_penalty = wear_ratio * degradation_multiplier * 0.5; // Up to 0.5s per lap

    // Fuel load impact
    let fuel_remaining = config.fuel_model.fuel_needed_for_laps(
        (config.total_laps - lap) as u16,
        config.starting_fuel,
    );
    let fuel_penalty = (fuel_remaining / config.starting_fuel) * 0.3; // Up to 0.3s

    // Compound grip level impact (higher grip = faster lap times)
    let grip_bonus = (tire_chars.grip_level - 0.75) * 2.0; // Softer compounds are faster

    base_time + wear_penalty + fuel_penalty - grip_bonus
}

fn update_dp_state(
    dp: &mut HashMap<(u16, u8, TireCompound), DPState>,
    key: (u16, u8, TireCompound),
    new_state: DPState,
) {
    dp.entry(key)
        .and_modify(|existing| {
            if new_state.best_time < existing.best_time {
                *existing = new_state.clone();
            }
        })
        .or_insert(new_state);
}

fn determine_pit_reason(
    lap: u16,
    config: &OptimizationConfig,
    pit_stops: &[PitStop],
) -> PitStopReason {
    if pit_stops.is_empty() {
        PitStopReason::Mandatory
    } else if lap < config.total_laps / 3 {
        // Early stop - likely undercut
        PitStopReason::Undercut
    } else if lap > (config.total_laps * 2) / 3 {
        // Late stop - likely tire degradation
        PitStopReason::TireDegradation
    } else {
        PitStopReason::Opportunistic
    }
}

fn is_valid_strategy(pit_stops: &[PitStop], config: &OptimizationConfig) -> bool {
    // Must meet minimum pit stop requirement
    if pit_stops.len() < config.min_pit_stops as usize {
        return false;
    }

    // Collect all compounds used (including starting compound)
    let mut compounds: Vec<TireCompound> = pit_stops.iter().map(|ps| ps.compound).collect();
    compounds.push(config.available_compounds[0]); // Add starting compound
    compounds.sort();
    compounds.dedup();

    // In dry conditions, must use at least 2 different compounds
    compounds.len() >= 2 ||
        compounds.contains(&TireCompound::Intermediate) ||
        compounds.contains(&TireCompound::Wet)
}

fn calculate_expected_lap_times(
    pit_stops: &[PitStop],
    config: &OptimizationConfig,
) -> BTreeMap<StintNumber, Vec<f32>> {
    let mut lap_times = BTreeMap::new();
    let mut current_stint = 0;
    let mut stint_start_lap = 1;

    for lap in 1..=config.total_laps {
        // Calculate lap time for current stint BEFORE checking for pit
        let tire_age = if lap >= stint_start_lap {
            lap - stint_start_lap + 1
        } else {
            1
        };

        let compound = if current_stint == 0 {
            config.available_compounds[0]
        } else {
            pit_stops.get(current_stint - 1).map(|ps| ps.compound)
                .unwrap_or(config.available_compounds[0])
        };

        let lap_time = calculate_lap_time(compound, tire_age, config, lap);

        lap_times.entry(StintNumber(current_stint as u8))
            .or_insert_with(Vec::new)
            .push(lap_time);

        // Check if we pit AFTER this lap (for next lap's stint)
        if pit_stops.iter().any(|ps| ps.lap.0 == lap) {
            current_stint += 1;
            stint_start_lap = lap + 1;
        }
    }

    lap_times
}

fn calculate_strategy_risk(strategy: &RaceStrategy, config: &OptimizationConfig) -> f32 {
    let mut risk = 0.0;

    // Risk from number of pit stops (more stops = more risk)
    risk += strategy.num_pit_stops() as f32 * 0.1;

    // Risk from tire degradation
    for pit_stop in &strategy.pit_stops {
        let tire_chars = TireCharacteristics::for_compound(pit_stop.compound);
        if tire_chars.degradation_rate > 0.015 {
            risk += 0.2; // Higher degradation = higher risk
        }
    }

    // Risk from late pit stops (traffic)
    for pit_stop in &strategy.pit_stops {
        if pit_stop.lap.0 > (config.total_laps * 2) / 3 {
            risk += 0.15;
        }
    }

    risk
}

fn estimate_total_tire_wear(strategy: &RaceStrategy, config: &OptimizationConfig) -> f32 {
    let mut total_wear = 0.0;

    for (i, pit_stop) in strategy.pit_stops.iter().enumerate() {
        let stint_length = if i == 0 {
            pit_stop.lap.0
        } else {
            pit_stop.lap.0 - strategy.pit_stops[i - 1].lap.0
        };

        let tire_chars = TireCharacteristics::for_compound(pit_stop.compound);
        total_wear += stint_length as f32 * tire_chars.degradation_rate
            * config.degradation_factors.total_multiplier();
    }

    total_wear
}

fn estimate_fuel_efficiency(_strategy: &RaceStrategy, config: &OptimizationConfig) -> f32 {
    // Calculate average fuel consumption over the race
    let total_fuel_needed = config.fuel_model.fuel_needed_for_laps(
        config.total_laps,
        config.starting_fuel,
    );

    // Efficiency = inverse of consumption (lower consumption = higher efficiency)
    1.0 / (total_fuel_needed / config.total_laps as f32)
}

fn count_overtaking_opportunities(strategy: &RaceStrategy, config: &OptimizationConfig) -> i8 {
    let mut opportunities = 0;

    // Undercut opportunities
    for pit_stop in &strategy.pit_stops {
        if pit_stop.reason == PitStopReason::Undercut {
            opportunities += 1;
        }
    }

    // Tire advantage opportunities (fresher tires than competitors)
    for competitor in &config.competitors_ahead {
        if let Some(comp_pit_lap) = competitor.estimated_pit_lap {
            for pit_stop in &strategy.pit_stops {
                if pit_stop.lap.0 > comp_pit_lap && pit_stop.lap.0 - comp_pit_lap < 10 {
                    opportunities += 1;
                }
            }
        }
    }

    opportunities
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> OptimizationConfig {
        OptimizationConfig {
            total_laps: 50,
            circuit: Circuit::monaco(),
            available_compounds: vec![TireCompound::C3, TireCompound::C4, TireCompound::C5],
            pit_lane_time_loss: 18.0,
            tire_change_time: 2.5,
            current_position: 5,
            competitors_ahead: vec![],
            degradation_factors: DegradationFactors::default(),
            fuel_model: FuelConsumptionModel::default_model(),
            starting_fuel: 110.0,
            min_pit_stops: 1,
            max_pit_stops: 3,
        }
    }

    #[test]
    fn test_optimize_pit_strategy() {
        let config = create_test_config();
        let result = optimize_pit_strategy(&config);

        assert!(result.is_ok());
        let strategy = result.unwrap();

        // Should have at least 1 pit stop (mandatory)
        assert!(strategy.num_pit_stops() >= 1);

        // Should be valid
        assert!(strategy.is_valid(config.total_laps));

        // Should have reasonable race time
        assert!(strategy.predicted_race_time > 0.0);
        assert!(strategy.predicted_race_time < 10000.0); // Less than ~3 hours
    }

    #[test]
    fn test_calculate_pit_window() {
        let config = create_test_config();
        let window = calculate_pit_window(1, TireCompound::C3, &config);

        assert!(window.earliest_lap > 0);
        assert!(window.latest_lap <= config.total_laps);
        assert!(window.earliest_lap <= window.latest_lap);
        assert!(window.optimal_start >= window.earliest_lap);
        assert!(window.optimal_end <= window.latest_lap);
    }

    #[test]
    fn test_estimate_time_loss() {
        let config = create_test_config();

        // Early race pit stop
        let early_loss = estimate_time_loss(&config, 10);

        // Late race pit stop
        let late_loss = estimate_time_loss(&config, 40);

        // Should be positive
        assert!(early_loss > 0.0);
        assert!(late_loss > 0.0);

        // Late race should be slightly faster (lighter car)
        assert!(late_loss <= early_loss);
    }

    #[test]
    fn test_compare_strategies() {
        let config = create_test_config();

        let strategy_a = RaceStrategy {
            id: "strategy-a".to_string(),
            starting_compound: TireCompound::C3,
            pit_stops: vec![
                PitStop {
                    lap: LapNumber(20),
                    compound: TireCompound::C4,
                    pit_loss: 20.5,
                    reason: PitStopReason::Mandatory,
                    confidence: 0.9,
                },
            ],
            fuel_strategy: FuelStrategy {
                starting_fuel: 110.0,
                fuel_saving_per_lap: 0.0,
                fuel_saving_laps: vec![],
                minimum_buffer: 1.0,
            },
            ers_plan: ErsDeploymentPlan {
                default_mode: ErsMode::Medium,
                lap_overrides: BTreeMap::new(),
                overtake_laps: vec![],
            },
            expected_lap_times: BTreeMap::new(),
            predicted_race_time: 5400.0,
            confidence: 0.85,
            metadata: StrategyMetadata {
                generated_at: chrono::Utc::now(),
                num_simulations: 1000,
                contributing_agents: vec!["test".to_string()],
                version_hash: None,
                parent_strategy_id: None,
            },
        };

        let strategy_b = RaceStrategy {
            id: "strategy-b".to_string(),
            starting_compound: TireCompound::C3,
            pit_stops: vec![
                PitStop {
                    lap: LapNumber(25),
                    compound: TireCompound::C5,
                    pit_loss: 20.5,
                    reason: PitStopReason::Mandatory,
                    confidence: 0.85,
                },
            ],
            fuel_strategy: FuelStrategy {
                starting_fuel: 110.0,
                fuel_saving_per_lap: 0.0,
                fuel_saving_laps: vec![],
                minimum_buffer: 1.0,
            },
            ers_plan: ErsDeploymentPlan {
                default_mode: ErsMode::Medium,
                lap_overrides: BTreeMap::new(),
                overtake_laps: vec![],
            },
            expected_lap_times: BTreeMap::new(),
            predicted_race_time: 5410.0,
            confidence: 0.80,
            metadata: StrategyMetadata {
                generated_at: chrono::Utc::now(),
                num_simulations: 1000,
                contributing_agents: vec!["test".to_string()],
                version_hash: None,
                parent_strategy_id: None,
            },
        };

        let comparison = compare_strategies(&strategy_a, &strategy_b, &config);

        assert_eq!(comparison.time_delta, -10.0); // A is 10s faster
        assert!(comparison.breakdown.pit_loss_difference == 0.0); // Same pit loss
    }

    #[test]
    fn test_tire_age_calculation() {
        let pit_stops = vec![
            PitStop {
                lap: LapNumber(20),
                compound: TireCompound::C4,
                pit_loss: 20.5,
                reason: PitStopReason::Mandatory,
                confidence: 0.9,
            },
        ];

        assert_eq!(calculate_tire_age(10, &pit_stops), 10); // Before first pit
        assert_eq!(calculate_tire_age(25, &pit_stops), 5);  // 5 laps after pit on lap 20
        assert_eq!(calculate_tire_age(50, &pit_stops), 30); // 30 laps after pit
    }

    #[test]
    fn test_strategy_validation() {
        let config = create_test_config();

        // Valid strategy: 1 pit stop with different compounds
        let valid_stops = vec![
            PitStop {
                lap: LapNumber(25),
                compound: TireCompound::C4,
                pit_loss: 20.5,
                reason: PitStopReason::Mandatory,
                confidence: 0.9,
            },
        ];
        assert!(is_valid_strategy(&valid_stops, &config));

        // Invalid: no pit stops
        assert!(!is_valid_strategy(&[], &config));
    }

    #[test]
    fn test_lap_time_calculation() {
        let config = create_test_config();

        // Fresh tires should be faster
        let fresh_time = calculate_lap_time(TireCompound::C5, 0, &config, 10);
        let worn_time = calculate_lap_time(TireCompound::C5, 15, &config, 10);

        assert!(fresh_time < worn_time);

        // Grippier compound should be faster (all else equal)
        let c3_time = calculate_lap_time(TireCompound::C3, 5, &config, 10);
        let c5_time = calculate_lap_time(TireCompound::C5, 5, &config, 10);

        assert!(c5_time < c3_time); // C5 is softer/grippier
    }
}
