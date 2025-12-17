//! Telemetry data processing and validation

use crate::{TelemetryConfig, TelemetryError};
use f1_nexus_core::TelemetrySnapshot;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// Anomaly severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,      // Minor deviation from expected values
    Medium,   // Significant deviation requiring attention
    High,     // Critical safety issue requiring immediate action
}

/// Detailed anomaly information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyInfo {
    /// Field name where anomaly was detected
    pub field: String,

    /// Expected value range (min, max)
    pub expected_range: (f32, f32),

    /// Actual value observed
    pub actual_value: f32,

    /// Severity level
    pub severity: AnomalySeverity,

    /// Timestamp of detection
    pub timestamp: DateTime<Utc>,
}

/// Statistical data for a single metric
#[derive(Debug, Clone)]
struct MetricStats {
    values: VecDeque<f32>,
    mean: f32,
    std_dev: f32,
    window_size: usize,
}

impl MetricStats {
    fn new(window_size: usize) -> Self {
        MetricStats {
            values: VecDeque::with_capacity(window_size),
            mean: 0.0,
            std_dev: 0.0,
            window_size,
        }
    }

    fn push(&mut self, value: f32) {
        if self.values.len() >= self.window_size {
            self.values.pop_front();
        }
        self.values.push_back(value);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        if self.values.is_empty() {
            return;
        }

        // Calculate mean
        self.mean = self.values.iter().sum::<f32>() / self.values.len() as f32;

        // Calculate standard deviation
        let variance: f32 = self.values.iter()
            .map(|&x| {
                let diff = x - self.mean;
                diff * diff
            })
            .sum::<f32>() / self.values.len() as f32;

        self.std_dev = variance.sqrt();
    }

    fn z_score(&self, value: f32) -> f32 {
        if self.std_dev == 0.0 {
            return 0.0;
        }
        (value - self.mean) / self.std_dev
    }

    fn has_enough_data(&self) -> bool {
        self.values.len() >= (self.window_size / 2)
    }
}

/// Anomaly detector with statistical analysis
pub struct AnomalyDetector {
    config: TelemetryConfig,
    speed_stats: RwLock<MetricStats>,
    tire_temp_stats: RwLock<MetricStats>,
    brake_temp_stats: RwLock<MetricStats>,
    rpm_stats: RwLock<MetricStats>,
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(config: TelemetryConfig) -> Self {
        let window_size = config.buffer_size.min(100); // Use config buffer size, cap at 100

        AnomalyDetector {
            config,
            speed_stats: RwLock::new(MetricStats::new(window_size)),
            tire_temp_stats: RwLock::new(MetricStats::new(window_size)),
            brake_temp_stats: RwLock::new(MetricStats::new(window_size)),
            rpm_stats: RwLock::new(MetricStats::new(window_size)),
        }
    }

    /// Detect anomalies in telemetry snapshot
    pub fn detect(&self, snapshot: &TelemetrySnapshot) -> Vec<AnomalyInfo> {
        if !self.config.enable_anomaly_detection {
            return Vec::new();
        }

        let mut anomalies = Vec::new();
        let timestamp = snapshot.timestamp;

        // 1. Speed anomalies
        anomalies.extend(self.detect_speed_anomalies(snapshot, timestamp));

        // 2. Tire temperature anomalies
        anomalies.extend(self.detect_tire_temp_anomalies(snapshot, timestamp));

        // 3. Brake temperature anomalies
        anomalies.extend(self.detect_brake_temp_anomalies(snapshot, timestamp));

        // 4. Throttle/brake conflict
        if let Some(anomaly) = self.detect_throttle_brake_conflict(snapshot, timestamp) {
            anomalies.push(anomaly);
        }

        // 5. ERS battery anomalies
        if let Some(anomaly) = self.detect_ers_anomalies(snapshot, timestamp) {
            anomalies.push(anomaly);
        }

        // 6. RPM anomalies
        if let Some(anomaly) = self.detect_rpm_anomalies(snapshot, timestamp) {
            anomalies.push(anomaly);
        }

        // Update statistics with current values
        self.update_statistics(snapshot);

        anomalies
    }

    fn detect_speed_anomalies(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Vec<AnomalyInfo> {
        let mut anomalies = Vec::new();
        let speed = snapshot.motion.speed;

        // Hard limits: unrealistic values
        if speed < 0.0 {
            anomalies.push(AnomalyInfo {
                field: "speed".to_string(),
                expected_range: (0.0, 380.0),
                actual_value: speed,
                severity: AnomalySeverity::High,
                timestamp,
            });
        } else if speed > 380.0 {
            anomalies.push(AnomalyInfo {
                field: "speed".to_string(),
                expected_range: (0.0, 380.0),
                actual_value: speed,
                severity: AnomalySeverity::High,
                timestamp,
            });
        } else {
            // Z-score based detection
            let stats = self.speed_stats.read();
            if stats.has_enough_data() {
                let z = stats.z_score(speed);

                if z.abs() > 3.0 {
                    // More than 3 standard deviations
                    anomalies.push(AnomalyInfo {
                        field: "speed".to_string(),
                        expected_range: (stats.mean - 3.0 * stats.std_dev, stats.mean + 3.0 * stats.std_dev),
                        actual_value: speed,
                        severity: AnomalySeverity::Medium,
                        timestamp,
                    });
                } else if z.abs() > 2.0 {
                    // More than 2 standard deviations
                    anomalies.push(AnomalyInfo {
                        field: "speed".to_string(),
                        expected_range: (stats.mean - 2.0 * stats.std_dev, stats.mean + 2.0 * stats.std_dev),
                        actual_value: speed,
                        severity: AnomalySeverity::Low,
                        timestamp,
                    });
                }
            }
        }

        anomalies
    }

    fn detect_tire_temp_anomalies(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Vec<AnomalyInfo> {
        let mut anomalies = Vec::new();

        let tire_temps = [
            ("tire_temp_front_left", snapshot.tires.front_left.surface_temp),
            ("tire_temp_front_right", snapshot.tires.front_right.surface_temp),
            ("tire_temp_rear_left", snapshot.tires.rear_left.surface_temp),
            ("tire_temp_rear_right", snapshot.tires.rear_right.surface_temp),
        ];

        for (field, temp) in tire_temps {
            if temp > 120.0 {
                // Critical high temperature
                anomalies.push(AnomalyInfo {
                    field: field.to_string(),
                    expected_range: (40.0, 120.0),
                    actual_value: temp,
                    severity: AnomalySeverity::High,
                    timestamp,
                });
            } else if temp < 40.0 {
                // Abnormally low temperature
                anomalies.push(AnomalyInfo {
                    field: field.to_string(),
                    expected_range: (40.0, 120.0),
                    actual_value: temp,
                    severity: AnomalySeverity::Medium,
                    timestamp,
                });
            } else {
                // Statistical detection
                let stats = self.tire_temp_stats.read();
                if stats.has_enough_data() {
                    let z = stats.z_score(temp);

                    if z.abs() > 2.5 {
                        anomalies.push(AnomalyInfo {
                            field: field.to_string(),
                            expected_range: (stats.mean - 2.5 * stats.std_dev, stats.mean + 2.5 * stats.std_dev),
                            actual_value: temp,
                            severity: AnomalySeverity::Low,
                            timestamp,
                        });
                    }
                }
            }
        }

        anomalies
    }

    fn detect_brake_temp_anomalies(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Vec<AnomalyInfo> {
        let mut anomalies = Vec::new();

        let brake_temps = [
            ("brake_temp_front_left", snapshot.tires.front_left.brake_temp),
            ("brake_temp_front_right", snapshot.tires.front_right.brake_temp),
            ("brake_temp_rear_left", snapshot.tires.rear_left.brake_temp),
            ("brake_temp_rear_right", snapshot.tires.rear_right.brake_temp),
        ];

        for (field, temp) in brake_temps {
            if temp > 1200.0 {
                // Critical brake temperature
                anomalies.push(AnomalyInfo {
                    field: field.to_string(),
                    expected_range: (0.0, 1200.0),
                    actual_value: temp,
                    severity: AnomalySeverity::High,
                    timestamp,
                });
            } else {
                // Statistical detection
                let stats = self.brake_temp_stats.read();
                if stats.has_enough_data() {
                    let z = stats.z_score(temp);

                    if z.abs() > 3.0 {
                        anomalies.push(AnomalyInfo {
                            field: field.to_string(),
                            expected_range: (stats.mean - 3.0 * stats.std_dev, stats.mean + 3.0 * stats.std_dev),
                            actual_value: temp,
                            severity: AnomalySeverity::Medium,
                            timestamp,
                        });
                    }
                }
            }
        }

        anomalies
    }

    fn detect_throttle_brake_conflict(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Option<AnomalyInfo> {
        // Check for simultaneous high throttle and brake (> 0.5 each)
        if snapshot.inputs.throttle > 0.5 && snapshot.inputs.brake > 0.5 {
            Some(AnomalyInfo {
                field: "throttle_brake_conflict".to_string(),
                expected_range: (0.0, 0.5),
                actual_value: snapshot.inputs.throttle.min(snapshot.inputs.brake),
                severity: AnomalySeverity::High,
                timestamp,
            })
        } else {
            None
        }
    }

    fn detect_ers_anomalies(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Option<AnomalyInfo> {
        let battery = snapshot.power_unit.ers_battery;

        // Check for battery level outside valid range
        if battery > 1.0 {
            Some(AnomalyInfo {
                field: "ers_battery".to_string(),
                expected_range: (0.0, 1.0),
                actual_value: battery,
                severity: AnomalySeverity::High,
                timestamp,
            })
        } else if battery < 0.0 {
            Some(AnomalyInfo {
                field: "ers_battery".to_string(),
                expected_range: (0.0, 1.0),
                actual_value: battery,
                severity: AnomalySeverity::High,
                timestamp,
            })
        } else {
            None
        }
    }

    fn detect_rpm_anomalies(&self, snapshot: &TelemetrySnapshot, timestamp: DateTime<Utc>) -> Option<AnomalyInfo> {
        let rpm = snapshot.power_unit.rpm as f32;

        // Hard limits for F1 engines
        if rpm > 15000.0 {
            Some(AnomalyInfo {
                field: "rpm".to_string(),
                expected_range: (0.0, 15000.0),
                actual_value: rpm,
                severity: AnomalySeverity::High,
                timestamp,
            })
        } else {
            // Statistical detection
            let stats = self.rpm_stats.read();
            if stats.has_enough_data() {
                let z = stats.z_score(rpm);

                if z.abs() > 3.0 {
                    Some(AnomalyInfo {
                        field: "rpm".to_string(),
                        expected_range: (stats.mean - 3.0 * stats.std_dev, stats.mean + 3.0 * stats.std_dev),
                        actual_value: rpm,
                        severity: AnomalySeverity::Medium,
                        timestamp,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    fn update_statistics(&self, snapshot: &TelemetrySnapshot) {
        // Update moving averages for statistical detection
        self.speed_stats.write().push(snapshot.motion.speed);

        // Average tire temperature
        let avg_tire_temp = (
            snapshot.tires.front_left.surface_temp +
            snapshot.tires.front_right.surface_temp +
            snapshot.tires.rear_left.surface_temp +
            snapshot.tires.rear_right.surface_temp
        ) / 4.0;
        self.tire_temp_stats.write().push(avg_tire_temp);

        // Average brake temperature
        let avg_brake_temp = (
            snapshot.tires.front_left.brake_temp +
            snapshot.tires.front_right.brake_temp +
            snapshot.tires.rear_left.brake_temp +
            snapshot.tires.rear_right.brake_temp
        ) / 4.0;
        self.brake_temp_stats.write().push(avg_brake_temp);

        // RPM
        self.rpm_stats.write().push(snapshot.power_unit.rpm as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::*;
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

    // ===== Anomaly Detection Tests =====

    #[test]
    fn test_anomaly_detector_no_anomalies() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let snapshot = create_test_snapshot();

        let anomalies = detector.detect(&snapshot);
        assert!(anomalies.is_empty(), "Normal telemetry should not trigger anomalies");
    }

    #[test]
    fn test_speed_anomaly_negative() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.motion.speed = -10.0;

        let anomalies = detector.detect(&snapshot);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].field, "speed");
        assert_eq!(anomalies[0].severity, AnomalySeverity::High);
        assert_eq!(anomalies[0].actual_value, -10.0);
    }

    #[test]
    fn test_speed_anomaly_too_high() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.motion.speed = 400.0;

        let anomalies = detector.detect(&snapshot);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].field, "speed");
        assert_eq!(anomalies[0].severity, AnomalySeverity::High);
        assert_eq!(anomalies[0].actual_value, 400.0);
    }

    #[test]
    fn test_tire_temp_too_high() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.tires.front_left.surface_temp = 130.0;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let temp_anomaly = anomalies.iter().find(|a| a.field == "tire_temp_front_left").unwrap();
        assert_eq!(temp_anomaly.severity, AnomalySeverity::High);
        assert_eq!(temp_anomaly.actual_value, 130.0);
        assert_eq!(temp_anomaly.expected_range, (40.0, 120.0));
    }

    #[test]
    fn test_tire_temp_too_low() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.tires.rear_right.surface_temp = 30.0;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let temp_anomaly = anomalies.iter().find(|a| a.field == "tire_temp_rear_right").unwrap();
        assert_eq!(temp_anomaly.severity, AnomalySeverity::Medium);
        assert_eq!(temp_anomaly.actual_value, 30.0);
    }

    #[test]
    fn test_brake_temp_critical() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.tires.front_left.brake_temp = 1250.0;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let brake_anomaly = anomalies.iter().find(|a| a.field == "brake_temp_front_left").unwrap();
        assert_eq!(brake_anomaly.severity, AnomalySeverity::High);
        assert_eq!(brake_anomaly.actual_value, 1250.0);
        assert_eq!(brake_anomaly.expected_range, (0.0, 1200.0));
    }

    #[test]
    fn test_throttle_brake_conflict() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.inputs.throttle = 0.8;
        snapshot.inputs.brake = 0.7;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let conflict = anomalies.iter().find(|a| a.field == "throttle_brake_conflict").unwrap();
        assert_eq!(conflict.severity, AnomalySeverity::High);
    }

    #[test]
    fn test_throttle_brake_no_conflict() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.inputs.throttle = 0.3;
        snapshot.inputs.brake = 0.4;

        let anomalies = detector.detect(&snapshot);
        let has_conflict = anomalies.iter().any(|a| a.field == "throttle_brake_conflict");
        assert!(!has_conflict, "No conflict should occur with both inputs below 0.5");
    }

    #[test]
    fn test_ers_battery_over_100() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.power_unit.ers_battery = 1.2;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let ers_anomaly = anomalies.iter().find(|a| a.field == "ers_battery").unwrap();
        assert_eq!(ers_anomaly.severity, AnomalySeverity::High);
        assert_eq!(ers_anomaly.actual_value, 1.2);
        assert_eq!(ers_anomaly.expected_range, (0.0, 1.0));
    }

    #[test]
    fn test_ers_battery_negative() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.power_unit.ers_battery = -0.1;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let ers_anomaly = anomalies.iter().find(|a| a.field == "ers_battery").unwrap();
        assert_eq!(ers_anomaly.severity, AnomalySeverity::High);
        assert_eq!(ers_anomaly.actual_value, -0.1);
    }

    #[test]
    fn test_rpm_over_limit() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.power_unit.rpm = 16000;

        let anomalies = detector.detect(&snapshot);
        assert!(!anomalies.is_empty());

        let rpm_anomaly = anomalies.iter().find(|a| a.field == "rpm").unwrap();
        assert_eq!(rpm_anomaly.severity, AnomalySeverity::High);
        assert_eq!(rpm_anomaly.actual_value, 16000.0);
        assert_eq!(rpm_anomaly.expected_range, (0.0, 15000.0));
    }

    #[test]
    fn test_statistical_detection_with_history() {
        let detector = AnomalyDetector::new(TelemetryConfig {
            buffer_size: 20,
            ..TelemetryConfig::default()
        });

        // Build up history with consistent speed around 250 km/h
        for i in 0..15 {
            let mut snapshot = create_test_snapshot();
            snapshot.motion.speed = 250.0 + (i as f32 * 2.0); // 250-278 km/h
            detector.detect(&snapshot);
        }

        // Now test with outlier
        let mut outlier = create_test_snapshot();
        outlier.motion.speed = 350.0; // Way outside normal range

        let anomalies = detector.detect(&outlier);

        // Should detect speed anomaly due to statistical deviation
        let speed_anomalies: Vec<_> = anomalies.iter().filter(|a| a.field == "speed").collect();
        assert!(!speed_anomalies.is_empty(), "Should detect statistical speed anomaly");
    }

    #[test]
    fn test_metric_stats_z_score() {
        let mut stats = MetricStats::new(10);

        // Add values: 10, 20, 30, 40, 50 (mean = 30, std_dev ≈ 14.14)
        for val in [10.0, 20.0, 30.0, 40.0, 50.0] {
            stats.push(val);
        }

        // Test z-score calculation
        assert!(stats.has_enough_data());
        assert!((stats.mean - 30.0).abs() < 0.01);

        // Value of 30 (mean) should have z-score near 0
        let z = stats.z_score(30.0);
        assert!(z.abs() < 0.01);

        // Value far from mean should have high z-score
        let z_outlier = stats.z_score(100.0);
        assert!(z_outlier > 2.0);
    }

    #[test]
    fn test_metric_stats_window_size() {
        let mut stats = MetricStats::new(5);

        // Add more values than window size
        for i in 0..10 {
            stats.push(i as f32);
        }

        // Should only keep last 5 values: 5, 6, 7, 8, 9
        assert_eq!(stats.values.len(), 5);
        assert_eq!(*stats.values.front().unwrap(), 5.0);
        assert_eq!(*stats.values.back().unwrap(), 9.0);
    }

    #[test]
    fn test_multiple_anomalies() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();

        // Create multiple anomalies
        snapshot.motion.speed = 400.0; // Too high
        snapshot.tires.front_left.surface_temp = 130.0; // Too hot
        snapshot.inputs.throttle = 0.9;
        snapshot.inputs.brake = 0.8; // Conflict
        snapshot.power_unit.ers_battery = 1.5; // Over 100%

        let anomalies = detector.detect(&snapshot);

        // Should detect all anomalies
        assert!(anomalies.len() >= 4, "Should detect multiple anomalies");
        assert!(anomalies.iter().any(|a| a.field == "speed"));
        assert!(anomalies.iter().any(|a| a.field == "tire_temp_front_left"));
        assert!(anomalies.iter().any(|a| a.field == "throttle_brake_conflict"));
        assert!(anomalies.iter().any(|a| a.field == "ers_battery"));
    }

    #[test]
    fn test_anomaly_detection_disabled() {
        let config = TelemetryConfig {
            enable_anomaly_detection: false,
            ..TelemetryConfig::default()
        };
        let detector = AnomalyDetector::new(config);

        let mut snapshot = create_test_snapshot();
        snapshot.motion.speed = 500.0; // Would be anomaly if enabled

        let anomalies = detector.detect(&snapshot);
        assert!(anomalies.is_empty(), "Anomaly detection should be disabled");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AnomalySeverity::Low < AnomalySeverity::Medium);
        assert!(AnomalySeverity::Medium < AnomalySeverity::High);
        assert!(AnomalySeverity::High > AnomalySeverity::Low);
    }

    #[test]
    fn test_anomaly_info_fields() {
        let detector = AnomalyDetector::new(TelemetryConfig::default());
        let mut snapshot = create_test_snapshot();
        snapshot.motion.speed = -5.0;

        let anomalies = detector.detect(&snapshot);
        assert_eq!(anomalies.len(), 1);

        let anomaly = &anomalies[0];
        assert_eq!(anomaly.field, "speed");
        assert_eq!(anomaly.expected_range, (0.0, 380.0));
        assert_eq!(anomaly.actual_value, -5.0);
        assert_eq!(anomaly.severity, AnomalySeverity::High);
        // Timestamp should be set
        assert!(anomaly.timestamp <= Utc::now());
    }
}
