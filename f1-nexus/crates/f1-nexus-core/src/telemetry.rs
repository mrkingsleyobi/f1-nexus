//! Real-time telemetry data structures

use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete telemetry snapshot from a single car at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    /// Unique session identifier
    pub session_id: SessionId,

    /// Car number
    pub car_id: CarId,

    /// Timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Current lap number
    pub lap: LapNumber,

    /// Position on track
    pub position: Position,

    /// Speed and motion
    pub motion: MotionData,

    /// Tire telemetry
    pub tires: TireData,

    /// Power unit data
    pub power_unit: PowerUnitData,

    /// Aerodynamic data
    pub aero: AeroData,

    /// Brake data
    pub brakes: BrakeData,

    /// Driver inputs
    pub inputs: DriverInputs,

    /// Fuel data
    pub fuel: FuelData,

    /// DRS (Drag Reduction System) status
    pub drs: DrsStatus,
}

/// Motion and velocity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionData {
    /// Speed in km/h
    pub speed: f32,

    /// Acceleration in m/s² (forward/backward)
    pub acceleration: f32,

    /// Lateral G-force
    pub lateral_g: f32,

    /// Longitudinal G-force
    pub longitudinal_g: f32,

    /// Vertical G-force
    pub vertical_g: f32,

    /// Yaw rate (rad/s)
    pub yaw_rate: f32,

    /// Pitch angle (degrees)
    pub pitch: f32,

    /// Roll angle (degrees)
    pub roll: f32,
}

/// Tire-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireData {
    pub front_left: TireSensor,
    pub front_right: TireSensor,
    pub rear_left: TireSensor,
    pub rear_right: TireSensor,

    /// Current compound
    pub compound: TireCompound,

    /// Age in laps
    pub age_laps: u16,
}

/// Individual tire sensor readings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireSensor {
    /// Surface temperature (°C)
    pub surface_temp: f32,

    /// Inner temperature (°C)
    pub inner_temp: f32,

    /// Brake temperature (°C)
    pub brake_temp: f32,

    /// Tire pressure (PSI)
    pub pressure: f32,

    /// Wear percentage (0.0 = new, 1.0 = dead)
    pub wear: f32,

    /// Damage level (0.0 = perfect, 1.0 = destroyed)
    pub damage: f32,
}

/// Tire compound types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TireCompound {
    C0,  // Hardest
    C1,
    C2,
    C3,
    C4,
    C5,  // Softest
    Intermediate,
    Wet,
}

/// Power unit telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerUnitData {
    /// Engine RPM
    pub rpm: u16,

    /// Throttle position (0.0-1.0)
    pub throttle: f32,

    /// ERS deployment mode
    pub ers_mode: ErsMode,

    /// ERS battery level (0.0-1.0)
    pub ers_battery: f32,

    /// MGU-K deployment (kW)
    pub mgu_k_deployment: f32,

    /// MGU-H recovery (kW)
    pub mgu_h_recovery: f32,

    /// Engine temperature (°C)
    pub engine_temp: f32,

    /// Oil temperature (°C)
    pub oil_temp: f32,

    /// Oil pressure (bar)
    pub oil_pressure: f32,
}

/// ERS deployment modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErsMode {
    None,
    Low,
    Medium,
    High,
    Hotlap,
    Overtake,
}

/// Aerodynamic data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AeroData {
    /// Front wing angle (degrees)
    pub front_wing_angle: f32,

    /// Rear wing angle (degrees)
    pub rear_wing_angle: f32,

    /// Downforce (Newtons)
    pub downforce: f32,

    /// Drag coefficient
    pub drag_coefficient: f32,
}

/// Brake system data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrakeData {
    /// Brake bias (0.0 = full rear, 1.0 = full front)
    pub bias: f32,

    /// Brake pressure (0.0-1.0)
    pub pressure: f32,

    /// Front brake temperature (°C)
    pub front_temp: f32,

    /// Rear brake temperature (°C)
    pub rear_temp: f32,
}

/// Driver control inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInputs {
    /// Steering angle (-1.0 to 1.0, left to right)
    pub steering: f32,

    /// Throttle (0.0-1.0)
    pub throttle: f32,

    /// Brake (0.0-1.0)
    pub brake: f32,

    /// Clutch (0.0-1.0)
    pub clutch: f32,

    /// Current gear (-1 = reverse, 0 = neutral, 1-8 = gears)
    pub gear: i8,
}

/// Fuel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelData {
    /// Remaining fuel (kg)
    pub remaining: f32,

    /// Fuel consumption rate (kg/lap)
    pub consumption_rate: f32,

    /// Fuel temperature (°C)
    pub temperature: f32,

    /// Fuel pressure (bar)
    pub pressure: f32,
}

/// DRS status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrsStatus {
    Unavailable,
    Available,
    Activated,
}

impl TelemetrySnapshot {
    /// Calculate total tire wear across all four tires
    pub fn average_tire_wear(&self) -> f32 {
        (self.tires.front_left.wear
            + self.tires.front_right.wear
            + self.tires.rear_left.wear
            + self.tires.rear_right.wear)
            / 4.0
    }

    /// Check if any tire temperature is critical (>120°C)
    pub fn has_critical_tire_temp(&self) -> bool {
        let temps = [
            self.tires.front_left.surface_temp,
            self.tires.front_right.surface_temp,
            self.tires.rear_left.surface_temp,
            self.tires.rear_right.surface_temp,
        ];
        temps.iter().any(|&t| t > 120.0)
    }

    /// Estimate laps remaining on current fuel
    pub fn estimated_fuel_laps(&self) -> f32 {
        if self.fuel.consumption_rate > 0.0 {
            self.fuel.remaining / self.fuel.consumption_rate
        } else {
            f32::INFINITY
        }
    }

    /// Check if car is under braking
    pub fn is_braking(&self) -> bool {
        self.inputs.brake > 0.1
    }

    /// Check if car is at full throttle
    pub fn is_full_throttle(&self) -> bool {
        self.inputs.throttle > 0.95
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tire_sensor() -> TireSensor {
        TireSensor {
            surface_temp: 95.0,
            inner_temp: 100.0,
            brake_temp: 350.0,
            pressure: 21.5,
            wear: 0.3,
            damage: 0.0,
        }
    }

    #[test]
    fn test_average_tire_wear() {
        let snapshot = TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id: CarId::new(1).unwrap(),
            timestamp: Utc::now(),
            lap: LapNumber(10),
            position: Position(5),
            motion: MotionData {
                speed: 250.0,
                acceleration: 2.0,
                lateral_g: 4.0,
                longitudinal_g: 2.0,
                vertical_g: 1.0,
                yaw_rate: 0.1,
                pitch: 0.0,
                roll: 0.0,
            },
            tires: TireData {
                front_left: TireSensor { wear: 0.2, ..create_test_tire_sensor() },
                front_right: TireSensor { wear: 0.25, ..create_test_tire_sensor() },
                rear_left: TireSensor { wear: 0.3, ..create_test_tire_sensor() },
                rear_right: TireSensor { wear: 0.35, ..create_test_tire_sensor() },
                compound: TireCompound::C3,
                age_laps: 15,
            },
            power_unit: PowerUnitData {
                rpm: 11000,
                throttle: 1.0,
                ers_mode: ErsMode::Medium,
                ers_battery: 0.8,
                mgu_k_deployment: 120.0,
                mgu_h_recovery: 0.0,
                engine_temp: 105.0,
                oil_temp: 140.0,
                oil_pressure: 5.5,
            },
            aero: AeroData {
                front_wing_angle: 15.0,
                rear_wing_angle: 12.0,
                downforce: 15000.0,
                drag_coefficient: 0.78,
            },
            brakes: BrakeData {
                bias: 0.58,
                pressure: 0.0,
                front_temp: 350.0,
                rear_temp: 320.0,
            },
            inputs: DriverInputs {
                steering: 0.0,
                throttle: 1.0,
                brake: 0.0,
                clutch: 0.0,
                gear: 7,
            },
            fuel: FuelData {
                remaining: 60.0,
                consumption_rate: 1.2,
                temperature: 45.0,
                pressure: 6.0,
            },
            drs: DrsStatus::Available,
        };

        assert_eq!(snapshot.average_tire_wear(), 0.275);
        assert!(!snapshot.has_critical_tire_temp());
        assert!((snapshot.estimated_fuel_laps() - 50.0).abs() < 0.01);
        assert!(snapshot.is_full_throttle());
        assert!(!snapshot.is_braking());
    }
}
