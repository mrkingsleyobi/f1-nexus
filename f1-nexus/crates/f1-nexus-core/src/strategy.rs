//! Race strategy representation and manipulation

use crate::types::*;
use crate::tire::TireCompound;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Complete race strategy with pit stop plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceStrategy {
    /// Strategy unique identifier
    pub id: String,

    /// Starting tire compound
    pub starting_compound: TireCompound,

    /// Planned pit stops
    pub pit_stops: Vec<PitStop>,

    /// Fuel strategy
    pub fuel_strategy: FuelStrategy,

    /// ERS deployment plan
    pub ers_plan: ErsDeploymentPlan,

    /// Expected lap times per stint
    pub expected_lap_times: BTreeMap<StintNumber, Vec<f32>>,

    /// Total race time prediction (seconds)
    pub predicted_race_time: f32,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Strategy generation metadata
    pub metadata: StrategyMetadata,
}

/// Planned pit stop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitStop {
    /// Target lap for pit stop
    pub lap: LapNumber,

    /// Tire compound to fit
    pub compound: TireCompound,

    /// Expected pit loss (seconds)
    pub pit_loss: f32,

    /// Reason for pit stop
    pub reason: PitStopReason,

    /// Confidence in this decision (0.0-1.0)
    pub confidence: f32,
}

/// Reason for pit stop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PitStopReason {
    Mandatory,
    TireDegradation,
    TireDamage,
    WeatherChange,
    Undercut,
    Overcut,
    SafetyCar,
    Virtual SafetyCar,
    Opportunistic,
}

/// Stint number (0-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StintNumber(pub u8);

/// Fuel management strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelStrategy {
    /// Starting fuel load (kg)
    pub starting_fuel: f32,

    /// Target fuel saving per lap (kg)
    pub fuel_saving_per_lap: f32,

    /// Laps requiring fuel saving mode
    pub fuel_saving_laps: Vec<LapNumber>,

    /// Minimum fuel buffer (kg)
    pub minimum_buffer: f32,
}

/// ERS deployment planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErsDeploymentPlan {
    /// Default ERS mode
    pub default_mode: ErsMode,

    /// Lap-specific ERS overrides
    pub lap_overrides: BTreeMap<LapNumber, ErsMode>,

    /// Overtake opportunities (laps to use max deployment)
    pub overtake_laps: Vec<LapNumber>,
}

/// ERS deployment modes (from telemetry)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErsMode {
    None,
    Low,
    Medium,
    High,
    Hotlap,
    Overtake,
}

/// Strategy generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetadata {
    /// Timestamp of strategy generation
    pub generated_at: chrono::DateTime<chrono::Utc>,

    /// Number of simulations run
    pub num_simulations: u64,

    /// Agents that contributed to this strategy
    pub contributing_agents: Vec<String>,

    /// Version hash (for agentic-jujutsu integration)
    pub version_hash: Option<String>,

    /// Parent strategy ID (if this is a refinement)
    pub parent_strategy_id: Option<String>,
}

impl RaceStrategy {
    /// Calculate total number of pit stops
    pub fn num_pit_stops(&self) -> usize {
        self.pit_stops.len()
    }

    /// Get pit stop for a specific lap, if any
    pub fn pit_stop_on_lap(&self, lap: LapNumber) -> Option<&PitStop> {
        self.pit_stops.iter().find(|ps| ps.lap == lap)
    }

    /// Check if strategy is valid according to FIA regulations
    pub fn is_valid(&self, total_race_laps: u16) -> bool {
        // Must have at least one pit stop (2024+ regulations)
        if self.pit_stops.is_empty() {
            return false;
        }

        // All pit stops must be within race duration
        if self.pit_stops.iter().any(|ps| ps.lap.0 > total_race_laps) {
            return false;
        }

        // Must use at least two different compounds (dry race)
        let mut compounds: Vec<_> = self.pit_stops
            .iter()
            .map(|ps| ps.compound)
            .collect();
        compounds.push(self.starting_compound);
        compounds.sort();
        compounds.dedup();

        compounds.len() >= 2 || compounds.contains(&TireCompound::Intermediate) || compounds.contains(&TireCompound::Wet)
    }

    /// Calculate total expected pit loss
    pub fn total_pit_loss(&self) -> f32 {
        self.pit_stops.iter().map(|ps| ps.pit_loss).sum()
    }

    /// Get stint number for a given lap
    pub fn stint_for_lap(&self, lap: LapNumber) -> StintNumber {
        let stint_count = self.pit_stops.iter().filter(|ps| ps.lap.0 < lap.0).count();
        StintNumber(stint_count as u8)
    }

    /// Get tire compound for a given lap
    pub fn compound_for_lap(&self, lap: LapNumber) -> TireCompound {
        // Find the most recent pit stop before this lap
        self.pit_stops
            .iter()
            .rev()
            .find(|ps| ps.lap.0 < lap.0)
            .map(|ps| ps.compound)
            .unwrap_or(self.starting_compound)
    }
}

/// Strategy comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyComparison {
    pub strategy_a_id: String,
    pub strategy_b_id: String,
    pub time_delta: f32, // seconds (positive = A is faster)
    pub risk_delta: f32, // (positive = A is riskier)
    pub recommendation: ComparisonRecommendation,
}

/// Strategy comparison recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonRecommendation {
    PreferA,
    PreferB,
    Equivalent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_validity() {
        let mut strategy = RaceStrategy {
            id: "test-strat-1".to_string(),
            starting_compound: TireCompound::C3,
            pit_stops: vec![
                PitStop {
                    lap: LapNumber(25),
                    compound: TireCompound::C2,
                    pit_loss: 22.5,
                    reason: PitStopReason::Mandatory,
                    confidence: 0.95,
                },
            ],
            fuel_strategy: FuelStrategy {
                starting_fuel: 110.0,
                fuel_saving_per_lap: 0.0,
                fuel_saving_laps: vec![],
                minimum_buffer: 2.0,
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
                num_simulations: 10_000,
                contributing_agents: vec!["strategy-agent".to_string()],
                version_hash: None,
                parent_strategy_id: None,
            },
        };

        assert!(strategy.is_valid(50));
        assert_eq!(strategy.num_pit_stops(), 1);
        assert_eq!(strategy.total_pit_loss(), 22.5);
        assert_eq!(strategy.compound_for_lap(LapNumber(20)), TireCompound::C3);
        assert_eq!(strategy.compound_for_lap(LapNumber(30)), TireCompound::C2);

        // Invalid: pit stop beyond race duration
        strategy.pit_stops[0].lap = LapNumber(100);
        assert!(!strategy.is_valid(50));
    }
}
