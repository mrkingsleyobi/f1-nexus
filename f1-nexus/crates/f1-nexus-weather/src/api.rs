//! Weather API client for F1 race forecasting
//!
//! This module provides integration with OpenWeatherMap API for fetching
//! current weather conditions and forecasts for F1 circuits.

use f1_nexus_core::{Circuit, WeatherCondition, WeatherForecast, WeatherPrediction, SectorWeather, Sector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Weather API client for OpenWeatherMap
pub struct WeatherApiClient {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
}

/// Current weather conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    /// Air temperature (°C)
    pub temperature: f32,
    /// Humidity (0.0-1.0)
    pub humidity: f32,
    /// Wind speed (km/h)
    pub wind_speed: f32,
    /// Wind direction (degrees, 0-360)
    pub wind_direction: f32,
    /// Rain probability (0.0-1.0)
    pub rain_probability: f32,
    /// Rainfall intensity (mm/hour)
    pub rainfall_intensity: f32,
    /// Weather condition
    pub condition: WeatherCondition,
}

/// Race-specific forecast for 2-hour race window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceForecast {
    /// Race start conditions
    pub start_conditions: WeatherConditions,
    /// Expected conditions during race
    pub race_conditions: Vec<WeatherConditions>,
    /// Rain probability throughout race
    pub rain_probability_timeline: Vec<(u16, f32)>, // (minutes_from_start, probability)
    /// Track temperature prediction
    pub track_temperature_timeline: Vec<(u16, f32)>, // (minutes_from_start, temp_celsius)
    /// Recommended tire strategy
    pub recommended_tire: String,
}

/// Circuit GPS coordinates
#[derive(Debug, Clone)]
pub struct CircuitCoordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// OpenWeatherMap API response for current weather
#[derive(Debug, Deserialize)]
struct OwmCurrentResponse {
    main: OwmMain,
    weather: Vec<OwmWeather>,
    wind: OwmWind,
    #[serde(default)]
    rain: Option<OwmRain>,
}

#[derive(Debug, Deserialize)]
struct OwmMain {
    temp: f32,
    humidity: f32,
}

#[derive(Debug, Deserialize)]
struct OwmWeather {
    id: u16,
    main: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct OwmWind {
    speed: f32,
    #[serde(default)]
    deg: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OwmRain {
    #[serde(rename = "1h", default)]
    one_hour: Option<f32>,
}

/// OpenWeatherMap API response for forecast
#[derive(Debug, Deserialize)]
struct OwmForecastResponse {
    list: Vec<OwmForecastItem>,
}

#[derive(Debug, Deserialize)]
struct OwmForecastItem {
    dt: i64, // Unix timestamp
    main: OwmMain,
    weather: Vec<OwmWeather>,
    wind: OwmWind,
    #[serde(default)]
    rain: Option<OwmRain>,
    pop: f32, // Probability of precipitation
}

impl WeatherApiClient {
    /// Create a new weather API client
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenWeatherMap API key (free tier available at https://openweathermap.org/api)
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://api.openweathermap.org/data/2.5".to_string(),
        }
    }

    /// Get current weather conditions at given coordinates
    ///
    /// # Arguments
    ///
    /// * `lat` - Latitude
    /// * `lon` - Longitude
    pub async fn get_current_weather(&self, lat: f64, lon: f64) -> Result<WeatherConditions, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/weather?lat={}&lon={}&appid={}&units=metric",
            self.base_url, lat, lon, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            // Fallback to default conditions on API error
            return Ok(Self::default_conditions());
        }

        let data: OwmCurrentResponse = response.json().await?;
        Ok(map_weather_conditions(&data))
    }

    /// Get weather forecast for next N hours
    ///
    /// # Arguments
    ///
    /// * `lat` - Latitude
    /// * `lon` - Longitude
    /// * `hours` - Number of hours to forecast (max 120)
    pub async fn get_forecast(&self, lat: f64, lon: f64, hours: u32) -> Result<Vec<WeatherConditions>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/forecast?lat={}&lon={}&appid={}&units=metric",
            self.base_url, lat, lon, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            // Fallback to default forecast
            return Ok(vec![Self::default_conditions()]);
        }

        let data: OwmForecastResponse = response.json().await?;

        // OpenWeatherMap provides 3-hour intervals
        let intervals_needed = (hours / 3).min(data.list.len() as u32);

        Ok(data.list
            .iter()
            .take(intervals_needed as usize)
            .map(map_forecast_item)
            .collect())
    }

    /// Get race-specific forecast for a circuit
    ///
    /// # Arguments
    ///
    /// * `circuit` - F1 circuit to get forecast for
    pub async fn get_race_forecast(&self, circuit: &Circuit) -> Result<RaceForecast, Box<dyn std::error::Error>> {
        let coords = get_circuit_coordinates(&circuit.id)
            .ok_or("Unknown circuit")?;

        // Get current conditions
        let start_conditions = self.get_current_weather(coords.latitude, coords.longitude).await?;

        // Get 2-hour forecast (race duration)
        let forecast = self.get_forecast(coords.latitude, coords.longitude, 6).await?;

        // Build timeline predictions
        let mut rain_probability_timeline = vec![(0, start_conditions.rain_probability)];
        let mut track_temperature_timeline = vec![(0, predict_track_temperature(start_conditions.temperature, &circuit.characteristics))];

        for (idx, conditions) in forecast.iter().enumerate() {
            let minutes = ((idx + 1) * 180) as u16; // 3-hour intervals = 180 minutes
            if minutes <= 120 {
                rain_probability_timeline.push((minutes, conditions.rain_probability));
                track_temperature_timeline.push((
                    minutes,
                    predict_track_temperature(conditions.temperature, &circuit.characteristics)
                ));
            }
        }

        // Determine recommended tire
        let recommended_tire = if start_conditions.rainfall_intensity > 5.0 {
            "Wet".to_string()
        } else if start_conditions.rainfall_intensity > 0.5 || start_conditions.rain_probability > 0.7 {
            "Intermediate".to_string()
        } else {
            "Dry (Soft/Medium/Hard)".to_string()
        };

        Ok(RaceForecast {
            start_conditions: start_conditions.clone(),
            race_conditions: forecast,
            rain_probability_timeline,
            track_temperature_timeline,
            recommended_tire,
        })
    }

    /// Get default weather conditions (fallback)
    fn default_conditions() -> WeatherConditions {
        WeatherConditions {
            temperature: 20.0,
            humidity: 0.5,
            wind_speed: 10.0,
            wind_direction: 0.0,
            rain_probability: 0.0,
            rainfall_intensity: 0.0,
            condition: WeatherCondition::Dry,
        }
    }
}

/// Map OpenWeatherMap current weather response to WeatherConditions
fn map_weather_conditions(data: &OwmCurrentResponse) -> WeatherConditions {
    let condition = map_owm_condition(&data.weather);
    let rainfall = data.rain.as_ref()
        .and_then(|r| r.one_hour)
        .unwrap_or(0.0);

    WeatherConditions {
        temperature: data.main.temp,
        humidity: data.main.humidity / 100.0, // Convert percentage to 0-1
        wind_speed: data.wind.speed * 3.6, // Convert m/s to km/h
        wind_direction: data.wind.deg.unwrap_or(0.0),
        rain_probability: if rainfall > 0.0 { 1.0 } else { 0.0 },
        rainfall_intensity: rainfall,
        condition,
    }
}

/// Map OpenWeatherMap forecast item to WeatherConditions
fn map_forecast_item(item: &OwmForecastItem) -> WeatherConditions {
    let condition = map_owm_condition(&item.weather);
    let rainfall = item.rain.as_ref()
        .and_then(|r| r.one_hour)
        .unwrap_or(0.0);

    WeatherConditions {
        temperature: item.main.temp,
        humidity: item.main.humidity / 100.0,
        wind_speed: item.wind.speed * 3.6,
        wind_direction: item.wind.deg.unwrap_or(0.0),
        rain_probability: item.pop,
        rainfall_intensity: rainfall,
        condition,
    }
}

/// Map OpenWeatherMap weather codes to F1 Nexus weather conditions
fn map_owm_condition(weather: &[OwmWeather]) -> WeatherCondition {
    if weather.is_empty() {
        return WeatherCondition::Dry;
    }

    let code = weather[0].id;

    match code {
        // Thunderstorm (200-232)
        200..=232 => WeatherCondition::HeavyRain,
        // Drizzle (300-321)
        300..=321 => WeatherCondition::LightRain,
        // Light rain (500-501, 520)
        500..=501 | 520 => WeatherCondition::LightRain,
        // Heavy rain (502-531)
        502..=531 => WeatherCondition::HeavyRain,
        // Snow is treated as wet (600-622)
        600..=622 => WeatherCondition::HeavyRain,
        // Clear (800)
        800 => WeatherCondition::Dry,
        // Few clouds (801)
        801 => WeatherCondition::PartlyCloudy,
        // Scattered/broken/overcast clouds (802-804)
        802..=804 => WeatherCondition::Cloudy,
        // Default to dry
        _ => WeatherCondition::Dry,
    }
}

/// Predict track surface temperature from air temperature
///
/// Track surface is typically 10-15°C warmer than air temperature in sunny conditions
/// and closer to air temperature in cloudy/wet conditions.
pub fn predict_track_temperature(air_temp: f32, characteristics: &f1_nexus_core::TrackCharacteristics) -> f32 {
    // Base track temperature is air temp + solar heating
    let base_heating = 12.0; // Average 12°C difference

    // Adjust for track characteristics
    // Low downforce tracks (like Monza) have less rubber buildup = less heat
    let downforce_factor = 1.0 + (characteristics.downforce_level - 0.5) * 0.2;

    air_temp + (base_heating * downforce_factor)
}

/// Calculate lap time impact from rain probability
///
/// Returns the expected lap time delta in seconds due to weather.
/// Positive values mean slower laps.
pub fn calculate_rain_impact(rain_probability: f32, rainfall_intensity: f32, base_lap_time: f32) -> f32 {
    if rain_probability < 0.3 {
        return 0.0; // No significant impact
    }

    // Light rain: 2-5% slower
    // Heavy rain: 10-20% slower
    let intensity_factor = if rainfall_intensity > 5.0 {
        0.15 // 15% slower in heavy rain
    } else if rainfall_intensity > 0.5 {
        0.035 // 3.5% slower in light rain
    } else {
        0.02 * rain_probability // Probabilistic impact
    };

    base_lap_time * intensity_factor
}

/// Get GPS coordinates for F1 circuits
pub fn get_circuit_coordinates(circuit_id: &str) -> Option<CircuitCoordinates> {
    let mut coords = HashMap::new();

    // Major F1 circuits with GPS coordinates
    coords.insert("monaco", CircuitCoordinates {
        latitude: 43.7347,
        longitude: 7.4206,
    });

    coords.insert("silverstone", CircuitCoordinates {
        latitude: 52.0786,
        longitude: -1.0169,
    });

    coords.insert("monza", CircuitCoordinates {
        latitude: 45.6156,
        longitude: 9.2811,
    });

    coords.insert("spa", CircuitCoordinates {
        latitude: 50.4372,
        longitude: 5.9714,
    });

    coords.insert("suzuka", CircuitCoordinates {
        latitude: 34.8431,
        longitude: 136.5407,
    });

    coords.insert("singapore", CircuitCoordinates {
        latitude: 1.2914,
        longitude: 103.8639,
    });

    coords.insert("interlagos", CircuitCoordinates {
        latitude: -23.7036,
        longitude: -46.6997,
    });

    coords.insert("austin", CircuitCoordinates {
        latitude: 30.1328,
        longitude: -97.6411,
    });

    coords.insert("bahrain", CircuitCoordinates {
        latitude: 26.0325,
        longitude: 50.5106,
    });

    coords.insert("jeddah", CircuitCoordinates {
        latitude: 21.6319,
        longitude: 39.1044,
    });

    coords.insert("melbourne", CircuitCoordinates {
        latitude: -37.8497,
        longitude: 144.9680,
    });

    coords.insert("shanghai", CircuitCoordinates {
        latitude: 31.3389,
        longitude: 121.2200,
    });

    coords.insert("barcelona", CircuitCoordinates {
        latitude: 41.5700,
        longitude: 2.2611,
    });

    coords.insert("hungaroring", CircuitCoordinates {
        latitude: 47.5789,
        longitude: 19.2486,
    });

    coords.insert("zandvoort", CircuitCoordinates {
        latitude: 52.3888,
        longitude: 4.5409,
    });

    coords.insert("imola", CircuitCoordinates {
        latitude: 44.3439,
        longitude: 11.7167,
    });

    coords.insert("red-bull-ring", CircuitCoordinates {
        latitude: 47.2197,
        longitude: 14.7647,
    });

    coords.insert("baku", CircuitCoordinates {
        latitude: 40.3725,
        longitude: 49.8533,
    });

    coords.insert("montreal", CircuitCoordinates {
        latitude: 45.5000,
        longitude: -73.5228,
    });

    coords.insert("mexico", CircuitCoordinates {
        latitude: 19.4042,
        longitude: -99.0907,
    });

    coords.insert("abu-dhabi", CircuitCoordinates {
        latitude: 24.4672,
        longitude: 54.6031,
    });

    coords.insert("las-vegas", CircuitCoordinates {
        latitude: 36.1147,
        longitude: -115.1728,
    });

    coords.insert("miami", CircuitCoordinates {
        latitude: 25.9581,
        longitude: -80.2389,
    });

    coords.insert("saudi-arabia", CircuitCoordinates {
        latitude: 21.6319,
        longitude: 39.1044,
    });

    coords.get(circuit_id).cloned()
}

/// Convert WeatherConditions to F1 Nexus WeatherForecast
pub fn to_weather_forecast(
    conditions: &WeatherConditions,
    circuit: &Circuit,
) -> WeatherForecast {
    let track_temp = predict_track_temperature(conditions.temperature, &circuit.characteristics);

    // Create sector conditions (simplified - all sectors same for now)
    let sector_conditions = vec![
        SectorWeather {
            sector: Sector::Sector1,
            condition: conditions.condition,
            rain_intensity: conditions.rainfall_intensity,
            track_temp,
            grip_level: calculate_grip_level(conditions.rainfall_intensity),
        },
        SectorWeather {
            sector: Sector::Sector2,
            condition: conditions.condition,
            rain_intensity: conditions.rainfall_intensity,
            track_temp,
            grip_level: calculate_grip_level(conditions.rainfall_intensity),
        },
        SectorWeather {
            sector: Sector::Sector3,
            condition: conditions.condition,
            rain_intensity: conditions.rainfall_intensity,
            track_temp,
            grip_level: calculate_grip_level(conditions.rainfall_intensity),
        },
    ];

    // Create predictions (next 30 minutes in 10-minute intervals)
    let predictions = vec![
        WeatherPrediction {
            minutes_ahead: 10,
            condition: conditions.condition,
            rain_probability: conditions.rain_probability,
            confidence: 0.9,
        },
        WeatherPrediction {
            minutes_ahead: 20,
            condition: conditions.condition,
            rain_probability: conditions.rain_probability,
            confidence: 0.8,
        },
        WeatherPrediction {
            minutes_ahead: 30,
            condition: conditions.condition,
            rain_probability: conditions.rain_probability * 0.95,
            confidence: 0.7,
        },
    ];

    WeatherForecast {
        overall_condition: conditions.condition,
        air_temperature: conditions.temperature,
        track_temperature: track_temp,
        humidity: conditions.humidity,
        wind_speed: conditions.wind_speed,
        wind_direction: conditions.wind_direction,
        rain_probability: conditions.rain_probability,
        rainfall_intensity: conditions.rainfall_intensity,
        sector_conditions,
        predictions,
    }
}

/// Calculate grip level based on rainfall intensity
fn calculate_grip_level(rainfall_intensity: f32) -> f32 {
    if rainfall_intensity > 5.0 {
        0.4 // Very low grip in heavy rain
    } else if rainfall_intensity > 0.5 {
        0.65 // Reduced grip in light rain
    } else {
        0.95 // High grip in dry conditions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::Circuit;

    #[test]
    fn test_circuit_coordinates() {
        let monaco = get_circuit_coordinates("monaco");
        assert!(monaco.is_some());
        let coords = monaco.unwrap();
        assert!((coords.latitude - 43.7347).abs() < 0.001);
        assert!((coords.longitude - 7.4206).abs() < 0.001);
    }

    #[test]
    fn test_all_famous_circuits_have_coordinates() {
        let circuits = ["monaco", "silverstone", "monza", "spa", "suzuka"];
        for circuit_id in circuits {
            assert!(
                get_circuit_coordinates(circuit_id).is_some(),
                "Missing coordinates for {}", circuit_id
            );
        }
    }

    #[test]
    fn test_predict_track_temperature() {
        let circuit = Circuit::monaco();
        let air_temp = 20.0;
        let track_temp = predict_track_temperature(air_temp, &circuit.characteristics);

        // Track should be warmer than air
        assert!(track_temp > air_temp);
        // But not unrealistically hot
        assert!(track_temp < air_temp + 20.0);
    }

    #[test]
    fn test_calculate_rain_impact() {
        let base_lap_time = 80.0; // 80 seconds

        // No rain
        let impact = calculate_rain_impact(0.0, 0.0, base_lap_time);
        assert_eq!(impact, 0.0);

        // Heavy rain
        let impact = calculate_rain_impact(0.9, 10.0, base_lap_time);
        assert!(impact > 5.0); // Should be significantly slower

        // Light rain
        let impact = calculate_rain_impact(0.7, 2.0, base_lap_time);
        assert!(impact > 0.0);
        assert!(impact < 5.0);
    }

    #[test]
    fn test_weather_api_client_creation() {
        let client = WeatherApiClient::new("test_api_key".to_string());
        assert_eq!(client.api_key, "test_api_key");
    }

    #[test]
    fn test_default_conditions() {
        let conditions = WeatherApiClient::default_conditions();
        assert_eq!(conditions.temperature, 20.0);
        assert_eq!(conditions.condition, WeatherCondition::Dry);
    }

    #[test]
    fn test_map_owm_condition() {
        // Test clear weather
        let weather = vec![OwmWeather {
            id: 800,
            main: "Clear".to_string(),
            description: "clear sky".to_string(),
        }];
        assert_eq!(map_owm_condition(&weather), WeatherCondition::Dry);

        // Test rain
        let weather = vec![OwmWeather {
            id: 500,
            main: "Rain".to_string(),
            description: "light rain".to_string(),
        }];
        assert_eq!(map_owm_condition(&weather), WeatherCondition::LightRain);
    }

    #[test]
    fn test_calculate_grip_level() {
        assert!(calculate_grip_level(0.0) > 0.9); // Dry
        assert!(calculate_grip_level(2.0) < 0.7); // Light rain
        assert!(calculate_grip_level(10.0) < 0.5); // Heavy rain
    }

    #[test]
    fn test_to_weather_forecast() {
        let circuit = Circuit::monaco();
        let conditions = WeatherConditions {
            temperature: 22.0,
            humidity: 0.6,
            wind_speed: 15.0,
            wind_direction: 180.0,
            rain_probability: 0.3,
            rainfall_intensity: 0.0,
            condition: WeatherCondition::PartlyCloudy,
        };

        let forecast = to_weather_forecast(&conditions, &circuit);

        assert_eq!(forecast.overall_condition, WeatherCondition::PartlyCloudy);
        assert_eq!(forecast.sector_conditions.len(), 3);
        assert_eq!(forecast.predictions.len(), 3);
        assert!(forecast.track_temperature > forecast.air_temperature);
    }

    // Integration tests (marked as ignored - require API key)
    #[test]
    #[ignore]
    async fn test_get_current_weather_integration() {
        let api_key = std::env::var("OPENWEATHERMAP_API_KEY")
            .expect("Set OPENWEATHERMAP_API_KEY environment variable");

        let client = WeatherApiClient::new(api_key);

        // Test Monaco coordinates
        let result = client.get_current_weather(43.7347, 7.4206).await;
        assert!(result.is_ok());

        let conditions = result.unwrap();
        assert!(conditions.temperature > -50.0 && conditions.temperature < 60.0);
        assert!(conditions.humidity >= 0.0 && conditions.humidity <= 1.0);
    }

    #[test]
    #[ignore]
    async fn test_get_forecast_integration() {
        let api_key = std::env::var("OPENWEATHERMAP_API_KEY")
            .expect("Set OPENWEATHERMAP_API_KEY environment variable");

        let client = WeatherApiClient::new(api_key);

        let result = client.get_forecast(43.7347, 7.4206, 6).await;
        assert!(result.is_ok());

        let forecast = result.unwrap();
        assert!(!forecast.is_empty());
    }

    #[test]
    #[ignore]
    async fn test_get_race_forecast_integration() {
        let api_key = std::env::var("OPENWEATHERMAP_API_KEY")
            .expect("Set OPENWEATHERMAP_API_KEY environment variable");

        let client = WeatherApiClient::new(api_key);
        let circuit = Circuit::monaco();

        let result = client.get_race_forecast(&circuit).await;
        assert!(result.is_ok());

        let forecast = result.unwrap();
        assert!(!forecast.race_conditions.is_empty());
        assert!(!forecast.rain_probability_timeline.is_empty());
        assert!(!forecast.recommended_tire.is_empty());
    }
}
