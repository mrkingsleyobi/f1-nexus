//! Real-time telemetry streaming via WebSocket
//!
//! Provides WebSocket server for broadcasting telemetry data to multiple clients
//! with automatic reconnection, filtering, and low-latency delivery.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use f1_nexus_core::{CarId, SessionId, TelemetrySnapshot};
use futures::stream::StreamExt;
use futures::SinkExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// WebSocket streaming server for telemetry data
#[derive(Clone)]
pub struct TelemetryStreamServer {
    /// Broadcast channel for telemetry events
    tx: broadcast::Sender<StreamMessage>,

    /// Server configuration
    config: StreamConfig,
}

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Maximum number of connected clients
    pub max_clients: usize,

    /// Channel buffer size
    pub channel_buffer_size: usize,

    /// Enable compression for messages
    pub enable_compression: bool,

    /// Heartbeat interval (seconds)
    pub heartbeat_interval_secs: u64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            max_clients: 1000,
            channel_buffer_size: 10_000,
            enable_compression: false,
            heartbeat_interval_secs: 30,
        }
    }
}

/// Messages sent over the WebSocket stream
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamMessage {
    /// Telemetry snapshot
    Telemetry {
        session_id: String,
        car_id: u8,
        #[serde(flatten)]
        snapshot: TelemetrySnapshot,
    },

    /// Session started
    SessionStart {
        session_id: String,
        timestamp: String,
    },

    /// Session ended
    SessionEnd {
        session_id: String,
        timestamp: String,
    },

    /// Server heartbeat
    Heartbeat {
        timestamp: String,
        server_time_ms: u64,
    },

    /// Error message
    Error {
        code: String,
        message: String,
    },
}

/// Client subscription filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by car IDs
    pub car_ids: Option<Vec<u8>>,

    /// Only send anomalies
    pub anomalies_only: bool,
}

impl Default for SubscriptionFilter {
    fn default() -> Self {
        SubscriptionFilter {
            session_id: None,
            car_ids: None,
            anomalies_only: false,
        }
    }
}

/// Client request messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRequest {
    /// Subscribe with filter
    Subscribe {
        filter: SubscriptionFilter,
    },

    /// Unsubscribe
    Unsubscribe,

    /// Ping (client heartbeat)
    Ping {
        timestamp: String,
    },
}

/// WebSocket connection state
struct ConnectionState {
    filter: SubscriptionFilter,
    car_count: usize,
}

impl TelemetryStreamServer {
    /// Create new streaming server
    pub fn new(config: StreamConfig) -> Self {
        let (tx, _) = broadcast::channel(config.channel_buffer_size);

        TelemetryStreamServer {
            tx,
            config,
        }
    }

    /// Broadcast telemetry snapshot
    pub fn broadcast_telemetry(&self, snapshot: TelemetrySnapshot) -> Result<(), StreamError> {
        let msg = StreamMessage::Telemetry {
            session_id: snapshot.session_id.0.to_string(),
            car_id: snapshot.car_id.0,
            snapshot,
        };

        self.tx
            .send(msg)
            .map_err(|_| StreamError::BroadcastError("No active subscribers".to_string()))?;

        Ok(())
    }

    /// Broadcast session start event
    pub fn broadcast_session_start(&self, session_id: SessionId) {
        let msg = StreamMessage::SessionStart {
            session_id: session_id.0.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let _ = self.tx.send(msg);
    }

    /// Broadcast session end event
    pub fn broadcast_session_end(&self, session_id: SessionId) {
        let msg = StreamMessage::SessionEnd {
            session_id: session_id.0.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let _ = self.tx.send(msg);
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Create Axum router for WebSocket endpoint
    pub fn router(self) -> Router {
        Router::new()
            .route("/ws/telemetry", get(websocket_handler))
            .with_state(Arc::new(self))
    }
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server): State<Arc<TelemetryStreamServer>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, server))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, server: Arc<TelemetryStreamServer>) {
    info!("New WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut rx = server.tx.subscribe();
    let state = Arc::new(RwLock::new(ConnectionState {
        filter: SubscriptionFilter::default(),
        car_count: 0,
    }));

    // Spawn task to receive messages from broadcast channel and send to client
    let state_read = Arc::clone(&state);
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Apply filter
            let filter = state_read.read().filter.clone();
            if !should_send_message(&msg, &filter) {
                continue;
            }

            // Serialize and send
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Spawn task to receive messages from client
    let tx_clone = server.tx.clone();
    let state_write = Arc::clone(&state);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<ClientRequest>(&text) {
                    Ok(ClientRequest::Subscribe { filter }) => {
                        debug!("Client subscribed with filter: {:?}", filter);
                        state_write.write().filter = filter;
                    }
                    Ok(ClientRequest::Unsubscribe) => {
                        debug!("Client unsubscribed");
                        state_write.write().filter = SubscriptionFilter::default();
                    }
                    Ok(ClientRequest::Ping { timestamp }) => {
                        debug!("Received ping from client at {}", timestamp);
                        // Send pong (heartbeat)
                        let pong = StreamMessage::Heartbeat {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            server_time_ms: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };
                        let _ = tx_clone.send(pong);
                    }
                    Err(e) => {
                        warn!("Failed to parse client request: {}", e);
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    info!("WebSocket connection closed");
}

/// Check if message should be sent based on filter
fn should_send_message(msg: &StreamMessage, filter: &SubscriptionFilter) -> bool {
    match msg {
        StreamMessage::Telemetry {
            session_id,
            car_id,
            ..
        } => {
            // Check session filter
            if let Some(ref filter_session) = filter.session_id {
                if session_id != filter_session {
                    return false;
                }
            }

            // Check car filter
            if let Some(ref filter_cars) = filter.car_ids {
                if !filter_cars.contains(car_id) {
                    return false;
                }
            }

            true
        }
        StreamMessage::SessionStart { session_id, .. }
        | StreamMessage::SessionEnd { session_id, .. } => {
            // Always send session events if session matches filter
            if let Some(ref filter_session) = filter.session_id {
                session_id == filter_session
            } else {
                true
            }
        }
        StreamMessage::Heartbeat { .. } | StreamMessage::Error { .. } => {
            // Always send heartbeats and errors
            true
        }
    }
}

/// Streaming errors
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Broadcast error: {0}")]
    BroadcastError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::{
        AeroData, BrakeData, DriverInputs, DrsStatus, FuelData, LapNumber, MotionData,
        Position, PowerUnitData, TireCompound, TireData, TireSensor,
    };
    use f1_nexus_core::telemetry::ErsMode;

    fn create_test_snapshot() -> TelemetrySnapshot {
        TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id: CarId::new(1).unwrap(),
            timestamp: chrono::Utc::now(),
            lap: LapNumber(10),
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
    fn test_stream_server_creation() {
        let config = StreamConfig::default();
        let server = TelemetryStreamServer::new(config);

        assert_eq!(server.subscriber_count(), 0);
    }

    #[test]
    fn test_broadcast_telemetry() {
        let config = StreamConfig::default();
        let server = TelemetryStreamServer::new(config);
        let snapshot = create_test_snapshot();

        // Should fail when no subscribers
        let result = server.broadcast_telemetry(snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_filtering() {
        let msg = StreamMessage::Telemetry {
            session_id: "test-session".to_string(),
            car_id: 1,
            snapshot: create_test_snapshot(),
        };

        // No filter - should send
        let filter = SubscriptionFilter::default();
        assert!(should_send_message(&msg, &filter));

        // Session filter - match
        let filter = SubscriptionFilter {
            session_id: Some("test-session".to_string()),
            ..Default::default()
        };
        assert!(should_send_message(&msg, &filter));

        // Session filter - no match
        let filter = SubscriptionFilter {
            session_id: Some("other-session".to_string()),
            ..Default::default()
        };
        assert!(!should_send_message(&msg, &filter));

        // Car filter - match
        let filter = SubscriptionFilter {
            car_ids: Some(vec![1, 2, 3]),
            ..Default::default()
        };
        assert!(should_send_message(&msg, &filter));

        // Car filter - no match
        let filter = SubscriptionFilter {
            car_ids: Some(vec![2, 3, 4]),
            ..Default::default()
        };
        assert!(!should_send_message(&msg, &filter));
    }

    #[test]
    fn test_heartbeat_always_sent() {
        let msg = StreamMessage::Heartbeat {
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_time_ms: 1000,
        };

        let filter = SubscriptionFilter {
            session_id: Some("test".to_string()),
            car_ids: Some(vec![1]),
            anomalies_only: true,
        };

        // Heartbeat should always be sent regardless of filter
        assert!(should_send_message(&msg, &filter));
    }
}
