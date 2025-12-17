//! WebSocket telemetry streaming server
//!
//! Provides real-time telemetry data streaming to connected clients via WebSocket.
//! Supports multiple concurrent clients with automatic heartbeat monitoring.

use f1_nexus_core::TelemetrySnapshot;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::{interval, Instant};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Result as WsResult},
    WebSocketStream,
};
use tracing::{debug, error, info, warn};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsMessage {
    /// Telemetry data broadcast
    Telemetry {
        timestamp: String,
        data: TelemetrySnapshot,
    },
    /// Heartbeat ping
    Ping {
        timestamp: String,
    },
    /// Heartbeat pong response
    Pong {
        timestamp: String,
    },
    /// Connection established
    Connected {
        message: String,
        server_time: String,
    },
    /// Error message
    Error {
        message: String,
    },
}

/// Client connection state
struct ClientConnection {
    id: u64,
    addr: SocketAddr,
    last_pong: Arc<RwLock<Instant>>,
}

impl ClientConnection {
    fn new(id: u64, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            last_pong: Arc::new(RwLock::new(Instant::now())),
        }
    }

    fn update_pong(&self) {
        *self.last_pong.write() = Instant::now();
    }

    fn time_since_pong(&self) -> Duration {
        self.last_pong.read().elapsed()
    }
}

/// WebSocket telemetry streaming server
pub struct TelemetryWebSocketServer {
    addr: SocketAddr,
    tx: broadcast::Sender<TelemetrySnapshot>,
    client_counter: Arc<RwLock<u64>>,
    heartbeat_interval: Duration,
    client_timeout: Duration,
}

impl TelemetryWebSocketServer {
    /// Create a new WebSocket server
    ///
    /// # Arguments
    /// * `addr` - Socket address to bind to (e.g., "127.0.0.1:8080")
    /// * `channel_capacity` - Broadcast channel capacity (default: 1000)
    pub fn new(addr: SocketAddr, channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);

        Self {
            addr,
            tx,
            client_counter: Arc::new(RwLock::new(0)),
            heartbeat_interval: Duration::from_secs(30),
            client_timeout: Duration::from_secs(90),
        }
    }

    /// Create a new WebSocket server with default settings
    ///
    /// Binds to 0.0.0.0:8080 with a channel capacity of 1000
    pub fn with_defaults() -> Result<Self> {
        let addr = "0.0.0.0:8080".parse()
            .context("Failed to parse default address")?;
        Ok(Self::new(addr, 1000))
    }

    /// Set the heartbeat interval
    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Set the client timeout duration
    pub fn with_client_timeout(mut self, timeout: Duration) -> Self {
        self.client_timeout = timeout;
        self
    }

    /// Start the WebSocket server
    ///
    /// This will bind to the configured address and start accepting connections.
    /// The server will run until the task is cancelled.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .context("Failed to bind to address")?;

        info!("WebSocket telemetry server listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            error!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Broadcast telemetry snapshot to all connected clients
    ///
    /// This can be called from external code to push telemetry data to the WebSocket server.
    pub fn broadcast_telemetry(&self, snapshot: TelemetrySnapshot) -> Result<()> {
        self.tx
            .send(snapshot)
            .context("Failed to broadcast telemetry")?;
        Ok(())
    }

    /// Get the broadcast sender for external use
    ///
    /// This allows external code to send telemetry snapshots directly
    pub fn sender(&self) -> broadcast::Sender<TelemetrySnapshot> {
        self.tx.clone()
    }

    /// Get the number of currently connected clients
    pub fn client_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Handle a single WebSocket connection
    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        let ws_stream = accept_async(stream)
            .await
            .context("WebSocket handshake failed")?;

        let client_id = {
            let mut counter = self.client_counter.write();
            *counter += 1;
            *counter
        };

        info!("Client #{} connected from {}", client_id, addr);

        let client = Arc::new(ClientConnection::new(client_id, addr));

        // Send welcome message
        let welcome = WsMessage::Connected {
            message: format!("Connected to F1 Nexus Telemetry Server (Client #{})", client_id),
            server_time: chrono::Utc::now().to_rfc3339(),
        };

        if let Err(e) = self.handle_client(ws_stream, client).await {
            warn!("Client #{} disconnected: {}", client_id, e);
        }

        info!("Client #{} from {} disconnected", client_id, addr);

        Ok(())
    }

    /// Handle client communication
    async fn handle_client(
        &self,
        ws_stream: WebSocketStream<TcpStream>,
        client: Arc<ClientConnection>,
    ) -> Result<()> {
        let (mut write, mut read) = ws_stream.split();

        // Send welcome message
        let welcome = WsMessage::Connected {
            message: format!("Connected to F1 Nexus Telemetry Server (Client #{})", client.id),
            server_time: chrono::Utc::now().to_rfc3339(),
        };
        let welcome_json = serde_json::to_string(&welcome)?;
        write.send(Message::Text(welcome_json)).await?;

        // Subscribe to telemetry broadcasts
        let mut rx = self.tx.subscribe();

        // Heartbeat timer
        let mut heartbeat = interval(self.heartbeat_interval);
        let client_for_timeout = Arc::clone(&client);
        let timeout_duration = self.client_timeout;

        loop {
            tokio::select! {
                // Receive telemetry from broadcast channel
                telemetry = rx.recv() => {
                    match telemetry {
                        Ok(snapshot) => {
                            let msg = WsMessage::Telemetry {
                                timestamp: snapshot.timestamp.to_rfc3339(),
                                data: snapshot,
                            };

                            let json = match serde_json::to_string(&msg) {
                                Ok(j) => j,
                                Err(e) => {
                                    error!("Failed to serialize telemetry: {}", e);
                                    continue;
                                }
                            };

                            if let Err(e) = write.send(Message::Text(json)).await {
                                warn!("Failed to send telemetry to client #{}: {}", client.id, e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("Client #{} lagged, skipped {} messages", client.id, skipped);
                            // Send error notification to client
                            let error_msg = WsMessage::Error {
                                message: format!("Client lagged, skipped {} messages", skipped),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = write.send(Message::Text(json)).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Broadcast channel closed");
                            break;
                        }
                    }
                }

                // Receive messages from client
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Handle incoming text messages (e.g., Pong responses)
                            if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                match ws_msg {
                                    WsMessage::Pong { .. } => {
                                        client.update_pong();
                                        debug!("Received pong from client #{}", client.id);
                                    }
                                    _ => {
                                        debug!("Received message from client #{}: {:?}", client.id, ws_msg);
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // WebSocket protocol-level pong
                            client.update_pong();
                            debug!("Received protocol pong from client #{}", client.id);
                        }
                        Some(Ok(Message::Close(_))) => {
                            debug!("Client #{} sent close frame", client.id);
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // Respond to protocol-level ping
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                warn!("Failed to send pong to client #{}: {}", client.id, e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            warn!("WebSocket error from client #{}: {}", client.id, e);
                            break;
                        }
                        None => {
                            debug!("Client #{} connection closed", client.id);
                            break;
                        }
                        _ => {}
                    }
                }

                // Send periodic heartbeat
                _ = heartbeat.tick() => {
                    // Check if client has timed out
                    if client_for_timeout.time_since_pong() > timeout_duration {
                        warn!("Client #{} timed out (no pong in {:?})", client.id, timeout_duration);
                        break;
                    }

                    let ping_msg = WsMessage::Ping {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let json = match serde_json::to_string(&ping_msg) {
                        Ok(j) => j,
                        Err(e) => {
                            error!("Failed to serialize ping: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = write.send(Message::Text(json)).await {
                        warn!("Failed to send ping to client #{}: {}", client.id, e);
                        break;
                    }

                    debug!("Sent ping to client #{}", client.id);
                }
            }
        }

        // Graceful shutdown
        let _ = write.close().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::telemetry::*;
    use f1_nexus_core::types::*;

    fn create_test_snapshot() -> TelemetrySnapshot {
        TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id: CarId::new(1).unwrap(),
            timestamp: chrono::Utc::now(),
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
    fn test_server_creation() {
        let addr = "127.0.0.1:9090".parse().unwrap();
        let server = TelemetryWebSocketServer::new(addr, 100);
        assert_eq!(server.addr, addr);
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_server_with_defaults() {
        let server = TelemetryWebSocketServer::with_defaults().unwrap();
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_broadcast_telemetry() {
        let addr = "127.0.0.1:9091".parse().unwrap();
        let server = TelemetryWebSocketServer::new(addr, 100);
        let snapshot = create_test_snapshot();

        // Broadcasting without any subscribers should work
        assert!(server.broadcast_telemetry(snapshot).is_ok());
    }

    #[test]
    fn test_ws_message_serialization() {
        let snapshot = create_test_snapshot();
        let msg = WsMessage::Telemetry {
            timestamp: snapshot.timestamp.to_rfc3339(),
            data: snapshot,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"telemetry\""));
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_ping_message_serialization() {
        let msg = WsMessage::Ping {
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ping\""));
        assert!(json.contains("\"timestamp\""));
    }

    #[test]
    fn test_connected_message_serialization() {
        let msg = WsMessage::Connected {
            message: "Welcome".to_string(),
            server_time: chrono::Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"connected\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"server_time\""));
    }

    #[test]
    fn test_client_connection_pong_tracking() {
        let addr = "127.0.0.1:9092".parse().unwrap();
        let client = ClientConnection::new(1, addr);

        let initial_time = client.time_since_pong();
        std::thread::sleep(Duration::from_millis(100));
        let later_time = client.time_since_pong();

        assert!(later_time > initial_time);

        client.update_pong();
        let updated_time = client.time_since_pong();
        assert!(updated_time < later_time);
    }

    // Integration tests (marked as #[ignore] for CI)

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_server_start_and_stop() {
        let addr = "127.0.0.1:18080".parse().unwrap();
        let server = Arc::new(TelemetryWebSocketServer::new(addr, 100));
        let server_clone = Arc::clone(&server);

        let handle = tokio::spawn(async move {
            server_clone.start().await
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cancel the server
        handle.abort();

        // Wait a bit for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_client_connection_and_broadcast() {
        use tokio_tungstenite::connect_async;

        let addr = "127.0.0.1:18081".parse().unwrap();
        let server = Arc::new(TelemetryWebSocketServer::new(addr, 100));
        let server_clone = Arc::clone(&server);

        // Start server
        let server_handle = tokio::spawn(async move {
            server_clone.start().await
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Connect client
        let (ws_stream, _) = connect_async("ws://127.0.0.1:18081")
            .await
            .expect("Failed to connect");

        let (mut write, mut read) = ws_stream.split();

        // Broadcast telemetry
        let snapshot = create_test_snapshot();
        server.broadcast_telemetry(snapshot.clone()).unwrap();

        // Read messages
        let mut received_welcome = false;
        let mut received_telemetry = false;

        for _ in 0..10 {
            if let Some(Ok(msg)) = tokio::time::timeout(
                Duration::from_millis(500),
                read.next()
            ).await.ok().flatten() {
                if let Message::Text(text) = msg {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        match ws_msg {
                            WsMessage::Connected { .. } => received_welcome = true,
                            WsMessage::Telemetry { .. } => received_telemetry = true,
                            _ => {}
                        }
                    }
                }
            }

            if received_welcome && received_telemetry {
                break;
            }
        }

        assert!(received_welcome, "Should receive welcome message");
        assert!(received_telemetry, "Should receive telemetry message");

        // Cleanup
        let _ = write.close().await;
        server_handle.abort();
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_heartbeat_mechanism() {
        use tokio_tungstenite::connect_async;

        let addr = "127.0.0.1:18082".parse().unwrap();
        let server = Arc::new(
            TelemetryWebSocketServer::new(addr, 100)
                .with_heartbeat_interval(Duration::from_millis(500))
        );
        let server_clone = Arc::clone(&server);

        // Start server
        let server_handle = tokio::spawn(async move {
            server_clone.start().await
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Connect client
        let (ws_stream, _) = connect_async("ws://127.0.0.1:18082")
            .await
            .expect("Failed to connect");

        let (mut write, mut read) = ws_stream.split();

        // Wait for ping
        let mut received_ping = false;

        for _ in 0..20 {
            if let Some(Ok(msg)) = tokio::time::timeout(
                Duration::from_millis(100),
                read.next()
            ).await.ok().flatten() {
                if let Message::Text(text) = msg {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        if matches!(ws_msg, WsMessage::Ping { .. }) {
                            received_ping = true;

                            // Send pong response
                            let pong = WsMessage::Pong {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            let pong_json = serde_json::to_string(&pong).unwrap();
                            let _ = write.send(Message::Text(pong_json)).await;
                            break;
                        }
                    }
                }
            }
        }

        assert!(received_ping, "Should receive ping message");

        // Cleanup
        let _ = write.close().await;
        server_handle.abort();
    }
}
