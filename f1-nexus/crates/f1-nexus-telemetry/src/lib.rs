//! F1 Nexus Telemetry Processing Engine
//!
//! Real-time telemetry data ingestion, processing, and anomaly detection
//! with sub-millisecond latency using SIMD optimization and neural inference.

pub mod processor;
pub mod stream;
pub mod anomaly;
pub mod buffer;
pub mod predictor;

pub use processor::*;
pub use stream::*;
pub use anomaly::*;
pub use buffer::*;
pub use predictor::*;

use f1_nexus_core::TelemetrySnapshot;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Telemetry processing engine
pub struct TelemetryEngine {
    processor: Arc<TelemetryProcessor>,
    anomaly_detector: Arc<AnomalyDetector>,
    tx: broadcast::Sender<TelemetryEvent>,
}

/// Telemetry events
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    Snapshot(TelemetrySnapshot),
    Anomaly(AnomalyInfo),
    LegacyAnomaly(AnomalyAlert), // For backward compatibility
    StreamStart { session_id: String },
    StreamEnd { session_id: String },
}

impl TelemetryEngine {
    /// Create new telemetry engine
    pub fn new(config: TelemetryConfig) -> Self {
        let (tx, _) = broadcast::channel(10_000);

        TelemetryEngine {
            processor: Arc::new(TelemetryProcessor::new(config.clone())),
            anomaly_detector: Arc::new(AnomalyDetector::new(config)),
            tx,
        }
    }

    /// Process incoming telemetry snapshot
    pub async fn process(&self, snapshot: TelemetrySnapshot) -> Result<(), TelemetryError> {
        // Process telemetry (validation, normalization, etc.)
        self.processor.process(&snapshot)?;

        // Run anomaly detection (new statistical detector)
        let anomalies = self.anomaly_detector.detect(&snapshot);
        for anomaly in anomalies {
            let _ = self.tx.send(TelemetryEvent::Anomaly(anomaly));
        }

        // Broadcast processed snapshot
        let _ = self.tx.send(TelemetryEvent::Snapshot(snapshot));

        Ok(())
    }

    /// Subscribe to telemetry events
    pub fn subscribe(&self) -> broadcast::Receiver<TelemetryEvent> {
        self.tx.subscribe()
    }

    /// Get processing statistics
    pub fn stats(&self) -> ProcessingStats {
        self.processor.stats()
    }
}

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,

    /// Anomaly detection threshold
    pub anomaly_threshold: f32,

    /// Buffer size for sliding window analysis
    pub buffer_size: usize,

    /// Enable SIMD optimizations
    pub enable_simd: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            enable_anomaly_detection: true,
            anomaly_threshold: 0.95,
            buffer_size: 1000,
            enable_simd: true,
        }
    }
}

/// Telemetry processing errors
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Invalid telemetry data: {0}")]
    InvalidData(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Anomaly detection error: {0}")]
    AnomalyDetectionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_engine_creation() {
        let config = TelemetryConfig::default();
        let _engine = TelemetryEngine::new(config);
        // Engine created successfully
    }
}
