//! Race state management

use crate::types::*;
use crate::strategy::RaceStrategy;
use crate::telemetry::TelemetrySnapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current race state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceState {
    /// Session identifier
    pub session_id: SessionId,

    /// Session type
    pub session_type: SessionType,

    /// Track identifier
    pub track_id: String,

    /// Current lap number
    pub current_lap: LapNumber,

    /// Total race laps
    pub total_laps: u16,

    /// Current flag status
    pub flag_status: FlagStatus,

    /// Weather condition
    pub weather: WeatherCondition,

    /// Track condition
    pub track_condition: TrackCondition,

    /// Positions of all cars
    pub positions: HashMap<CarId, CarPosition>,

    /// Active strategies per car
    pub strategies: HashMap<CarId, RaceStrategy>,

    /// Latest telemetry per car
    pub telemetry: HashMap<CarId, TelemetrySnapshot>,

    /// Race incidents
    pub incidents: Vec<RaceIncident>,

    /// Safety car deployments
    pub safety_car_periods: Vec<SafetyCarPeriod>,
}

/// Car position and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarPosition {
    pub car_id: CarId,
    pub position: Position,
    pub lap: LapNumber,
    pub gap_to_leader: f32, // seconds
    pub gap_to_ahead: f32,  // seconds
    pub last_lap_time: f32, // seconds
    pub is_in_pit: bool,
    pub is_retired: bool,
    pub retirement_reason: Option<RetirementReason>,
}

/// Reason for retirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetirementReason {
    Accident,
    Mechanical,
    PowerUnit,
    Gearbox,
    Transmission,
    Hydraulics,
    Electrical,
    Brakes,
    Collision,
    Disqualified,
}

/// Race incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceIncident {
    pub lap: LapNumber,
    pub sector: Option<Sector>,
    pub incident_type: IncidentType,
    pub involved_cars: Vec<CarId>,
    pub description: String,
}

/// Type of race incident
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentType {
    Collision,
    Spin,
    OffTrack,
    Debris,
    Mechanical,
    Penalty,
    Investigation,
}

/// Safety car period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCarPeriod {
    pub start_lap: LapNumber,
    pub end_lap: Option<LapNumber>,
    pub is_virtual: bool,
    pub reason: String,
}

impl RaceState {
    /// Get current leader
    pub fn leader(&self) -> Option<&CarPosition> {
        self.positions.values().find(|p| p.position == Position(1))
    }

    /// Get position for a specific car
    pub fn car_position(&self, car_id: CarId) -> Option<&CarPosition> {
        self.positions.get(&car_id)
    }

    /// Check if race is under safety car
    pub fn is_safety_car(&self) -> bool {
        matches!(
            self.flag_status,
            FlagStatus::SafetyCar | FlagStatus::VirtualSafetyCar
        )
    }

    /// Check if race is stopped
    pub fn is_stopped(&self) -> bool {
        matches!(self.flag_status, FlagStatus::Red)
    }

    /// Count number of cars still running
    pub fn running_cars(&self) -> usize {
        self.positions
            .values()
            .filter(|p| !p.is_retired)
            .count()
    }

    /// Get remaining race distance
    pub fn remaining_laps(&self) -> u16 {
        self.total_laps.saturating_sub(self.current_lap.0)
    }

    /// Calculate race progression (0.0-1.0)
    pub fn race_progress(&self) -> f32 {
        if self.total_laps > 0 {
            self.current_lap.0 as f32 / self.total_laps as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_car_position(car_id: u8, position: u8) -> (CarId, CarPosition) {
        let id = CarId::new(car_id).unwrap();
        let pos = CarPosition {
            car_id: id,
            position: Position(position),
            lap: LapNumber(10),
            gap_to_leader: (position as f32 - 1.0) * 2.5,
            gap_to_ahead: if position == 1 { 0.0 } else { 2.5 },
            last_lap_time: 92.5,
            is_in_pit: false,
            is_retired: false,
            retirement_reason: None,
        };
        (id, pos)
    }

    #[test]
    fn test_race_state() {
        let mut race_state = RaceState {
            session_id: SessionId::new(),
            session_type: SessionType::Race,
            track_id: "monaco".to_string(),
            current_lap: LapNumber(10),
            total_laps: 78,
            flag_status: FlagStatus::Green,
            weather: WeatherCondition::Dry,
            track_condition: TrackCondition::Dry,
            positions: HashMap::new(),
            strategies: HashMap::new(),
            telemetry: HashMap::new(),
            incidents: vec![],
            safety_car_periods: vec![],
        };

        race_state.positions.insert(create_test_car_position(1, 1).0, create_test_car_position(1, 1).1);
        race_state.positions.insert(create_test_car_position(2, 2).0, create_test_car_position(2, 2).1);

        assert_eq!(race_state.running_cars(), 2);
        assert_eq!(race_state.remaining_laps(), 68);
        assert!(!race_state.is_safety_car());
        assert!(!race_state.is_stopped());
        assert_eq!(race_state.leader().unwrap().car_id, CarId::new(1).unwrap());
        assert!((race_state.race_progress() - 0.128).abs() < 0.01);
    }
}
