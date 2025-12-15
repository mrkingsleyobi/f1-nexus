//! Telemetry data processing and validation

use crate::{TelemetryConfig, TelemetryError};
use f1_nexus_core::TelemetrySnapshot;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Telemetry processor
pub struct TelemetryProcessor {
    config: TelemetryConfig,
    stats: ProcessingStatsInner,
}

#[derive(Default)]
struct ProcessingStatsInner {
    total_processed: AtomicU64,
    total_errors: AtomicU64,
    total_latency_us: AtomicU64,
}

impl TelemetryProcessor {
    pub fn new(config: TelemetryConfig) -> Self {
        TelemetryProcessor {
            config,
            stats: ProcessingStatsInner::default(),
        }
    }

    /// Process a telemetry snapshot
    pub fn process(&self, snapshot: &TelemetrySnapshot) -> Result<(), TelemetryError> {
        let start = std::time::Instant::now();

        // Validate telemetry data
        self.validate(snapshot)?;

        // Normalize data if needed
        // (In production, this would apply calibrations, unit conversions, etc.)

        // Update statistics
        let latency_us = start.elapsed().as_micros() as u64;
        self.stats.total_processed.fetch_add(1, Ordering::Relaxed);
        self.stats.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);

        Ok(())
    }

    /// Validate telemetry snapshot
    fn validate(&self, snapshot: &TelemetrySnapshot) -> Result<(), TelemetryError> {
        // Validate speed is within reasonable bounds
        if snapshot.motion.speed < 0.0 || snapshot.motion.speed > 400.0 {
            return Err(TelemetryError::InvalidData(
                format!("Invalid speed: {}", snapshot.motion.speed)
            ));
        }

        // Validate tire temperatures
        let tire_temps = [
            snapshot.tires.front_left.surface_temp,
            snapshot.tires.front_right.surface_temp,
            snapshot.tires.rear_left.surface_temp,
            snapshot.tires.rear_right.surface_temp,
        ];

        for temp in tire_temps {
            if temp < -50.0 || temp > 200.0 {
                return Err(TelemetryError::InvalidData(
                    format!("Invalid tire temperature: {}", temp)
                ));
            }
        }

        // Validate throttle/brake inputs
        if snapshot.inputs.throttle < 0.0 || snapshot.inputs.throttle > 1.0 {
            return Err(TelemetryError::InvalidData(
                format!("Invalid throttle: {}", snapshot.inputs.throttle)
            ));
        }

        if snapshot.inputs.brake < 0.0 || snapshot.inputs.brake > 1.0 {
            return Err(TelemetryError::InvalidData(
                format!("Invalid brake: {}", snapshot.inputs.brake)
            ));
        }

        Ok(())
    }

    /// Get processing statistics
    pub fn stats(&self) -> ProcessingStats {
        let total = self.stats.total_processed.load(Ordering::Relaxed);
        let errors = self.stats.total_errors.load(Ordering::Relaxed);
        let total_latency = self.stats.total_latency_us.load(Ordering::Relaxed);

        ProcessingStats {
            total_processed: total,
            total_errors: errors,
            average_latency_us: if total > 0 {
                total_latency / total
            } else {
                0
            },
        }
    }
}

/// Processing statistics
#[derive(Debug, Clone, Copy)]
pub struct ProcessingStats {
    pub total_processed: u64,
    pub total_errors: u64,
    pub average_latency_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::{*, telemetry::*};
    use f1_nexus_core::telemetry::ErsMode;
    use chrono::Utc;

    fn create_test_snapshot() -> TelemetrySnapshot {
        TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id: CarId::new(1).unwrap(),
            timestamp: Utc::now(),
            lap: LapNumber(1),
            position: Position(1),
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
                front_left: TireSensor {
                    surface_temp: 95.0,
                    inner_temp: 100.0,
                    brake_temp: 350.0,
                    pressure: 21.5,
                    wear: 0.1,
                    damage: 0.0,
                },
                front_right: TireSensor {
                    surface_temp: 95.0,
                    inner_temp: 100.0,
                    brake_temp: 350.0,
                    pressure: 21.5,
                    wear: 0.1,
                    damage: 0.0,
                },
                rear_left: TireSensor {
                    surface_temp: 100.0,
                    inner_temp: 105.0,
                    brake_temp: 300.0,
                    pressure: 20.0,
                    wear: 0.15,
                    damage: 0.0,
                },
                rear_right: TireSensor {
                    surface_temp: 100.0,
                    inner_temp: 105.0,
                    brake_temp: 300.0,
                    pressure: 20.0,
                    wear: 0.15,
                    damage: 0.0,
                },
                compound: TireCompound::C3,
                age_laps: 5,
            },
            power_unit: PowerUnitData {
                rpm: 11000,
                throttle: 0.95,
                ers_mode: ErsMode::Medium,
                ers_battery: 0.7,
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
                throttle: 0.95,
                brake: 0.0,
                clutch: 0.0,
                gear: 7,
            },
            fuel: FuelData {
                remaining: 80.0,
                consumption_rate: 1.5,
                temperature: 45.0,
                pressure: 6.0,
            },
            drs: DrsStatus::Available,
        }
    }

    #[test]
    fn test_valid_telemetry() {
        let processor = TelemetryProcessor::new(TelemetryConfig::default());
        let snapshot = create_test_snapshot();

        assert!(processor.process(&snapshot).is_ok());

        let stats = processor.stats();
        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.total_errors, 0);
        assert!(stats.average_latency_us < 1000); // Should be sub-millisecond
    }

    #[test]
    fn test_invalid_speed() {
        let processor = TelemetryProcessor::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.motion.speed = 500.0; // Invalid

        assert!(processor.process(&snapshot).is_err());
    }
}
