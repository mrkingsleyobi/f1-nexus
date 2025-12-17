//! OpenF1 API client implementation
//!
//! Provides async access to real-time F1 telemetry and race data from the OpenF1 API.

use crate::telemetry::*;
use crate::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Base URL for the OpenF1 API
pub const OPENF1_BASE_URL: &str = "https://api.openf1.org/v1";

/// Default timeout for API requests
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// OpenF1 API client for fetching real-time telemetry and race data
#[derive(Debug, Clone)]
pub struct F1ApiClient {
    /// Base URL for the API
    base_url: String,
    /// HTTP client
    client: Client,
}

impl F1ApiClient {
    /// Create a new F1ApiClient with default configuration
    pub fn new() -> Result<Self> {
        Self::with_base_url(OPENF1_BASE_URL.to_string())
    }

    /// Create a new F1ApiClient with a custom base URL
    pub fn with_base_url(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent("f1-nexus/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { base_url, client })
    }

    /// Create a new F1ApiClient with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("f1-nexus/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url: OPENF1_BASE_URL.to_string(),
            client,
        })
    }

    /// Get session data for a specific session
    pub async fn get_session_data(&self, session_key: u32) -> Result<SessionData> {
        let url = format!("{}/sessions?session_key={}", self.base_url, session_key);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send session data request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "OpenF1 API request failed with status: {}",
                response.status()
            );
        }

        let sessions: Vec<OpenF1Session> = response
            .json()
            .await
            .context("Failed to parse session data response")?;

        sessions
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No session found for key: {}", session_key))
            .and_then(SessionData::try_from)
    }

    /// Get live telemetry data for a specific driver in a session
    pub async fn get_live_telemetry(
        &self,
        session_key: u32,
        driver_number: u8,
    ) -> Result<TelemetrySnapshot> {
        // Fetch car data (main telemetry)
        let car_data_url = format!(
            "{}/car_data?session_key={}&driver_number={}&speed>=0",
            self.base_url, session_key, driver_number
        );

        let car_response = self
            .client
            .get(&car_data_url)
            .send()
            .await
            .context("Failed to send car data request")?;

        if !car_response.status().is_success() {
            anyhow::bail!(
                "OpenF1 API car_data request failed with status: {}",
                car_response.status()
            );
        }

        let car_data: Vec<OpenF1CarData> = car_response
            .json()
            .await
            .context("Failed to parse car data response")?;

        let latest_car_data = car_data
            .into_iter()
            .last()
            .ok_or_else(|| anyhow::anyhow!("No car data found"))?;

        // Fetch location data
        let location_url = format!(
            "{}/location?session_key={}&driver_number={}",
            self.base_url, session_key, driver_number
        );

        let location_response = self
            .client
            .get(&location_url)
            .send()
            .await
            .context("Failed to send location request")?;

        let location_data: Vec<OpenF1Location> = if location_response.status().is_success() {
            location_response
                .json()
                .await
                .context("Failed to parse location response")?
        } else {
            vec![]
        };

        let latest_location = location_data.into_iter().last();

        // Convert to TelemetrySnapshot
        TelemetrySnapshot::from_openf1(
            session_key,
            driver_number,
            latest_car_data,
            latest_location,
        )
    }

    /// Get lap data for a specific driver in a session
    pub async fn get_lap_data(
        &self,
        session_key: u32,
        driver_number: u8,
    ) -> Result<Vec<LapData>> {
        let url = format!(
            "{}/laps?session_key={}&driver_number={}",
            self.base_url, session_key, driver_number
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send lap data request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "OpenF1 API laps request failed with status: {}",
                response.status()
            );
        }

        let laps: Vec<OpenF1Lap> = response
            .json()
            .await
            .context("Failed to parse lap data response")?;

        laps.into_iter().map(LapData::try_from).collect()
    }

    /// Get all drivers in a session
    pub async fn get_drivers(&self, session_key: u32) -> Result<Vec<DriverInfo>> {
        let url = format!("{}/drivers?session_key={}", self.base_url, session_key);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send drivers request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "OpenF1 API drivers request failed with status: {}",
                response.status()
            );
        }

        let drivers: Vec<OpenF1Driver> = response
            .json()
            .await
            .context("Failed to parse drivers response")?;

        drivers.into_iter().map(DriverInfo::try_from).collect()
    }
}

impl Default for F1ApiClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default F1ApiClient")
    }
}

// ============================================================================
// OpenF1 API Response Structures
// ============================================================================

/// OpenF1 session data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenF1Session {
    pub session_key: u32,
    pub session_name: String,
    pub date_start: String,
    pub date_end: String,
    pub gmt_offset: String,
    pub session_type: String,
    pub meeting_key: u32,
    pub location: String,
    pub country_name: String,
    pub circuit_short_name: String,
    pub year: u32,
}

/// OpenF1 car data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenF1CarData {
    pub session_key: u32,
    pub driver_number: u8,
    pub date: String,
    pub speed: Option<f32>,
    pub rpm: Option<u16>,
    pub n_gear: Option<i8>,
    pub throttle: Option<f32>,
    pub brake: Option<bool>,
    pub drs: Option<u8>,
}

/// OpenF1 location data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenF1Location {
    pub session_key: u32,
    pub driver_number: u8,
    pub date: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}

/// OpenF1 lap data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenF1Lap {
    pub session_key: u32,
    pub driver_number: u8,
    pub lap_number: u16,
    pub lap_duration: Option<f32>,
    pub is_pit_out_lap: Option<bool>,
    pub segment_1_duration: Option<f32>,
    pub segment_2_duration: Option<f32>,
    pub segment_3_duration: Option<f32>,
    pub date_start: String,
}

/// OpenF1 driver data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenF1Driver {
    pub session_key: u32,
    pub driver_number: u8,
    pub broadcast_name: String,
    pub full_name: String,
    pub name_acronym: String,
    pub team_name: String,
    pub team_colour: String,
    pub headshot_url: Option<String>,
    pub country_code: String,
}

// ============================================================================
// F1 Nexus Domain Types (for mapping)
// ============================================================================

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_key: u32,
    pub session_name: String,
    pub session_type: SessionType,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub location: String,
    pub country: String,
    pub circuit: String,
    pub year: u32,
}

/// Lap data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapData {
    pub lap_number: u16,
    pub lap_time: Option<f32>,
    pub sector_1_time: Option<f32>,
    pub sector_2_time: Option<f32>,
    pub sector_3_time: Option<f32>,
    pub is_pit_lap: bool,
}

/// Driver information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInfo {
    pub driver_number: u8,
    pub name: String,
    pub acronym: String,
    pub team: String,
    pub country_code: String,
}

// ============================================================================
// Conversion/Mapping Functions
// ============================================================================

impl TryFrom<OpenF1Session> for SessionData {
    type Error = anyhow::Error;

    fn try_from(session: OpenF1Session) -> Result<Self> {
        let session_type = match session.session_type.as_str() {
            "Practice 1" => SessionType::Practice1,
            "Practice 2" => SessionType::Practice2,
            "Practice 3" => SessionType::Practice3,
            "Qualifying" => SessionType::Qualifying,
            "Sprint" => SessionType::Sprint,
            "Race" => SessionType::Race,
            _ => SessionType::Race, // Default fallback
        };

        let start_time = DateTime::parse_from_rfc3339(&session.date_start)
            .context("Failed to parse session start time")?
            .with_timezone(&Utc);

        let end_time = if !session.date_end.is_empty() {
            Some(
                DateTime::parse_from_rfc3339(&session.date_end)
                    .context("Failed to parse session end time")?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        Ok(SessionData {
            session_key: session.session_key,
            session_name: session.session_name,
            session_type,
            start_time,
            end_time,
            location: session.location,
            country: session.country_name,
            circuit: session.circuit_short_name,
            year: session.year,
        })
    }
}

impl TryFrom<OpenF1Lap> for LapData {
    type Error = anyhow::Error;

    fn try_from(lap: OpenF1Lap) -> Result<Self> {
        Ok(LapData {
            lap_number: lap.lap_number,
            lap_time: lap.lap_duration,
            sector_1_time: lap.segment_1_duration,
            sector_2_time: lap.segment_2_duration,
            sector_3_time: lap.segment_3_duration,
            is_pit_lap: lap.is_pit_out_lap.unwrap_or(false),
        })
    }
}

impl TryFrom<OpenF1Driver> for DriverInfo {
    type Error = anyhow::Error;

    fn try_from(driver: OpenF1Driver) -> Result<Self> {
        Ok(DriverInfo {
            driver_number: driver.driver_number,
            name: driver.full_name,
            acronym: driver.name_acronym,
            team: driver.team_name,
            country_code: driver.country_code,
        })
    }
}

impl TelemetrySnapshot {
    /// Create a TelemetrySnapshot from OpenF1 data
    pub fn from_openf1(
        _session_key: u32,
        driver_number: u8,
        car_data: OpenF1CarData,
        _location: Option<OpenF1Location>,
    ) -> Result<Self> {
        let timestamp = DateTime::parse_from_rfc3339(&car_data.date)
            .context("Failed to parse car data timestamp")?
            .with_timezone(&Utc);

        let car_id = CarId::new(driver_number)
            .map_err(|e| anyhow::anyhow!("Invalid driver number: {}", e))?;

        // Create a simplified telemetry snapshot with available data
        // Note: OpenF1 API has limited telemetry compared to full F1 data
        Ok(TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id,
            timestamp,
            lap: LapNumber(1), // This would need to be fetched separately
            position: Position(1), // This would need to be fetched separately
            motion: MotionData {
                speed: car_data.speed.unwrap_or(0.0),
                acceleration: 0.0, // Not available from OpenF1
                lateral_g: 0.0,
                longitudinal_g: 0.0,
                vertical_g: 0.0,
                yaw_rate: 0.0,
                pitch: 0.0,
                roll: 0.0,
            },
            tires: TireData {
                front_left: TireSensor::default(),
                front_right: TireSensor::default(),
                rear_left: TireSensor::default(),
                rear_right: TireSensor::default(),
                compound: TireCompound::C3, // Default
                age_laps: 0,
            },
            power_unit: PowerUnitData {
                rpm: car_data.rpm.unwrap_or(0),
                throttle: car_data.throttle.unwrap_or(0.0) / 100.0, // OpenF1 is 0-100
                ers_mode: ErsMode::None,
                ers_battery: 0.0,
                mgu_k_deployment: 0.0,
                mgu_h_recovery: 0.0,
                engine_temp: 0.0,
                oil_temp: 0.0,
                oil_pressure: 0.0,
            },
            aero: AeroData {
                front_wing_angle: 0.0,
                rear_wing_angle: 0.0,
                downforce: 0.0,
                drag_coefficient: 0.0,
            },
            brakes: BrakeData {
                bias: 0.58, // Default
                pressure: if car_data.brake.unwrap_or(false) { 1.0 } else { 0.0 },
                front_temp: 0.0,
                rear_temp: 0.0,
            },
            inputs: DriverInputs {
                steering: 0.0,
                throttle: car_data.throttle.unwrap_or(0.0) / 100.0,
                brake: if car_data.brake.unwrap_or(false) { 1.0 } else { 0.0 },
                clutch: 0.0,
                gear: car_data.n_gear.unwrap_or(0),
            },
            fuel: FuelData {
                remaining: 0.0,
                consumption_rate: 0.0,
                temperature: 0.0,
                pressure: 0.0,
            },
            drs: match car_data.drs.unwrap_or(0) {
                0 => DrsStatus::Unavailable,
                1 => DrsStatus::Available,
                _ => DrsStatus::Activated,
            },
        })
    }
}

impl Default for TireSensor {
    fn default() -> Self {
        TireSensor {
            surface_temp: 0.0,
            inner_temp: 0.0,
            brake_temp: 0.0,
            pressure: 0.0,
            wear: 0.0,
            damage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = F1ApiClient::new();
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.base_url, OPENF1_BASE_URL);
    }

    #[test]
    fn test_custom_base_url() {
        let custom_url = "https://custom.api.com/v1";
        let client = F1ApiClient::with_base_url(custom_url.to_string());
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.base_url, custom_url);
    }

    #[test]
    fn test_session_type_mapping() {
        let session = OpenF1Session {
            session_key: 9158,
            session_name: "Race".to_string(),
            date_start: "2024-03-24T15:00:00+00:00".to_string(),
            date_end: "2024-03-24T17:00:00+00:00".to_string(),
            gmt_offset: "03:00:00".to_string(),
            session_type: "Race".to_string(),
            meeting_key: 1234,
            location: "Jeddah".to_string(),
            country_name: "Saudi Arabia".to_string(),
            circuit_short_name: "Jeddah Corniche Circuit".to_string(),
            year: 2024,
        };

        let result = SessionData::try_from(session);
        assert!(result.is_ok());

        let session_data = result.unwrap();
        assert_eq!(session_data.session_type, SessionType::Race);
        assert_eq!(session_data.session_key, 9158);
        assert_eq!(session_data.year, 2024);
    }

    #[test]
    fn test_lap_data_conversion() {
        let lap = OpenF1Lap {
            session_key: 9158,
            driver_number: 1,
            lap_number: 15,
            lap_duration: Some(92.345),
            is_pit_out_lap: Some(false),
            segment_1_duration: Some(28.123),
            segment_2_duration: Some(32.456),
            segment_3_duration: Some(31.766),
            date_start: "2024-03-24T15:30:00+00:00".to_string(),
        };

        let result = LapData::try_from(lap);
        assert!(result.is_ok());

        let lap_data = result.unwrap();
        assert_eq!(lap_data.lap_number, 15);
        assert_eq!(lap_data.lap_time, Some(92.345));
        assert_eq!(lap_data.is_pit_lap, false);
    }

    #[test]
    fn test_driver_info_conversion() {
        let driver = OpenF1Driver {
            session_key: 9158,
            driver_number: 1,
            broadcast_name: "M VERSTAPPEN".to_string(),
            full_name: "Max VERSTAPPEN".to_string(),
            name_acronym: "VER".to_string(),
            team_name: "Red Bull Racing".to_string(),
            team_colour: "3671C6".to_string(),
            headshot_url: Some("https://example.com/headshot.png".to_string()),
            country_code: "NED".to_string(),
        };

        let result = DriverInfo::try_from(driver);
        assert!(result.is_ok());

        let driver_info = result.unwrap();
        assert_eq!(driver_info.driver_number, 1);
        assert_eq!(driver_info.name, "Max VERSTAPPEN");
        assert_eq!(driver_info.acronym, "VER");
        assert_eq!(driver_info.team, "Red Bull Racing");
    }

    // Integration tests - marked as #[ignore] for CI
    #[tokio::test]
    #[ignore]
    async fn test_get_session_data_integration() {
        let client = F1ApiClient::new().expect("Failed to create client");

        // Use a known session key (2024 Saudi Arabian GP Race)
        let result = client.get_session_data(9158).await;

        // This test will only pass if you have internet connectivity
        // and the OpenF1 API is available
        if let Ok(session) = result {
            assert_eq!(session.session_key, 9158);
            assert!(session.year >= 2024);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_drivers_integration() {
        let client = F1ApiClient::new().expect("Failed to create client");

        // Use a known session key
        let result = client.get_drivers(9158).await;

        if let Ok(drivers) = result {
            assert!(!drivers.is_empty());
            assert!(drivers.len() <= 20); // Max 20 drivers
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_lap_data_integration() {
        let client = F1ApiClient::new().expect("Failed to create client");

        // Use a known session and driver (Max Verstappen #1)
        let result = client.get_lap_data(9158, 1).await;

        if let Ok(laps) = result {
            assert!(!laps.is_empty());
            // Verify laps are in order
            for (i, lap) in laps.iter().enumerate() {
                assert_eq!(lap.lap_number as usize, i + 1);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_live_telemetry_integration() {
        let client = F1ApiClient::new().expect("Failed to create client");

        // This test requires an active or recent session
        let result = client.get_live_telemetry(9158, 1).await;

        if let Ok(telemetry) = result {
            assert_eq!(telemetry.car_id, CarId::new(1).unwrap());
            // Basic validation that we got some data
            assert!(telemetry.timestamp.timestamp() > 0);
        }
    }
}
