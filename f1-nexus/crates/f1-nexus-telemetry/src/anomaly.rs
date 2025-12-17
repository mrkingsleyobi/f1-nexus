//! Anomaly detection using neural models

use crate::{TelemetryConfig, TelemetryError};
use f1_nexus_core::TelemetrySnapshot;
use serde::{Deserialize, Serialize};

/// Legacy anomaly detector using simple rule-based inference
/// Note: Use `processor::AnomalyDetector` for statistical anomaly detection
pub struct LegacyAnomalyDetector {
    config: TelemetryConfig,
    // In production: temporal-neural-solver model would be here
}

impl LegacyAnomalyDetector {
    pub fn new(config: TelemetryConfig) -> Self {
        LegacyAnomalyDetector { config }
    }

    /// Detect anomalies in telemetry snapshot
    pub fn detect(&self, snapshot: &TelemetrySnapshot) -> Result<Option<AnomalyAlert>, TelemetryError> {
        if !self.config.enable_anomaly_detection {
            return Ok(None);
        }

        // Check for critical tire temperature
        if snapshot.has_critical_tire_temp() {
            return Ok(Some(AnomalyAlert {
                anomaly_type: AnomalyType::CriticalTireTemperature,
                severity: Severity::High,
                description: "Tire temperature exceeded critical threshold".to_string(),
                confidence: 1.0,
                recommended_action: Some("Reduce pace or pit immediately".to_string()),
            }));
        }

        // Check for sudden speed drop (potential crash/spin)
        if snapshot.motion.speed < 50.0 && snapshot.inputs.throttle > 0.5 {
            return Ok(Some(AnomalyAlert {
                anomaly_type: AnomalyType::SuddenSpeedLoss,
                severity: Severity::Critical,
                description: "Sudden speed loss detected".to_string(),
                confidence: 0.95,
                recommended_action: Some("Check for damage".to_string()),
            }));
        }

        // Check for fuel shortage risk
        if snapshot.estimated_fuel_laps() < 5.0 {
            return Ok(Some(AnomalyAlert {
                anomaly_type: AnomalyType::FuelShortage,
                severity: Severity::Medium,
                description: format!("Low fuel: {:.1} laps remaining", snapshot.estimated_fuel_laps()),
                confidence: 0.92,
                recommended_action: Some("Enable fuel-saving mode".to_string()),
            }));
        }

        Ok(None)
    }
}

/// Anomaly alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub description: String,
    pub confidence: f32,
    pub recommended_action: Option<String>,
}

/// Types of anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    CriticalTireTemperature,
    TireDamage,
    SuddenSpeedLoss,
    PowerUnitIssue,
    FuelShortage,
    BrakeFailure,
    AerodynamicDamage,
    SensorMalfunction,
}

/// Severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}
