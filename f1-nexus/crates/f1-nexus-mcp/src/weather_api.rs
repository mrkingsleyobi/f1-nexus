//! Weather API integration for real-time weather data
//!
//! Integrates with OpenWeatherMap API to fetch current conditions and forecasts

use anyhow::Result;
use f1_nexus_core::{WeatherCondition, WeatherForecast, WeatherPrediction, SectorWeather, Sector};
use serde::Deserialize;
use tracing::{info, warn};

/// OpenWeatherMap API client
pub struct WeatherApiClient {
    api_key: String,
    base_url: String,
}

impl WeatherApiClient {
    /// Create a new weather API client
    pub fn new(api_key: String) -> Self {
        WeatherApiClient {
            api_key,
            base_url: "https://api.openweathermap.org/data/2.5".to_string(),
        }
    }

    /// Fetch current weather for track coordinates
    pub async fn get_track_weather(&self, lat: f32, lon: f32) -> Result<WeatherForecast> {
        info!("Fetching weather for coordinates: ({}, {})", lat, lon);

        // Fetch current weather
        let current_url = format!(
            "{}/weather?lat={}&lon={}&appid={}&units=metric",
            self.base_url, lat, lon, self.api_key
        );

        let current_response = reqwest::get(&current_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch current weather: {}", e))?;

        if !current_response.status().is_success() {
            let status = current_response.status();
            let body = current_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Weather API error {}: {}",
                status,
                body
            ));
        }

        let current: OpenWeatherCurrentResponse = current_response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse current weather: {}", e))?;

        // Fetch forecast
        let forecast_url = format!(
            "{}/forecast?lat={}&lon={}&appid={}&units=metric&cnt=8",
            self.base_url, lat, lon, self.api_key
        );

        let forecast_response = reqwest::get(&forecast_url).await;

        let predictions = if let Ok(response) = forecast_response {
            if response.status().is_success() {
                if let Ok(forecast_data) = response.json::<OpenWeatherForecastResponse>().await {
                    convert_forecast_predictions(&forecast_data)
                } else {
                    warn!("Failed to parse forecast data, using empty predictions");
                    vec![]
                }
            } else {
                warn!("Forecast API returned error, using empty predictions");
                vec![]
            }
        } else {
            warn!("Failed to fetch forecast, using empty predictions");
            vec![]
        };

        // Convert to our WeatherForecast format
        let forecast = convert_to_weather_forecast(&current, predictions);

        Ok(forecast)
    }

    /// Get weather for a specific F1 circuit by name
    pub async fn get_circuit_weather(&self, circuit_name: &str) -> Result<WeatherForecast> {
        let coords = get_circuit_coordinates(circuit_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown circuit: {}", circuit_name))?;

        self.get_track_weather(coords.0, coords.1).await
    }
}

/// OpenWeatherMap current weather response
#[derive(Debug, Deserialize)]
struct OpenWeatherCurrentResponse {
    weather: Vec<WeatherDescription>,
    main: MainWeatherData,
    wind: WindData,
    #[serde(default)]
    rain: Option<RainData>,
    clouds: CloudsData,
}

#[derive(Debug, Deserialize)]
struct WeatherDescription {
    id: u32,
    main: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct MainWeatherData {
    temp: f32,
    feels_like: f32,
    humidity: f32,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f32,
    deg: f32,
}

#[derive(Debug, Deserialize)]
struct RainData {
    #[serde(rename = "1h")]
    one_hour: Option<f32>,
    #[serde(rename = "3h")]
    three_hour: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct CloudsData {
    all: f32,
}

/// OpenWeatherMap forecast response
#[derive(Debug, Deserialize)]
struct OpenWeatherForecastResponse {
    list: Vec<ForecastItem>,
}

#[derive(Debug, Deserialize)]
struct ForecastItem {
    dt: i64,
    main: MainWeatherData,
    weather: Vec<WeatherDescription>,
    pop: f32, // Probability of precipitation
    #[serde(default)]
    rain: Option<RainData>,
}

/// Convert OpenWeatherMap condition to our WeatherCondition
fn convert_weather_condition(weather_id: u32, rain_intensity: f32) -> WeatherCondition {
    match weather_id {
        // Thunderstorm
        200..=299 => WeatherCondition::HeavyRain,
        // Drizzle
        300..=399 => WeatherCondition::LightRain,
        // Rain
        500..=504 => {
            if rain_intensity > 5.0 {
                WeatherCondition::HeavyRain
            } else {
                WeatherCondition::LightRain
            }
        }
        511 => WeatherCondition::HeavyRain, // Freezing rain
        520..=599 => WeatherCondition::LightRain,
        // Snow
        600..=699 => WeatherCondition::HeavyRain, // Treat as heavy rain for racing
        // Atmosphere (mist, fog, etc.)
        700..=799 => WeatherCondition::Cloudy,
        // Clear
        800 => WeatherCondition::Dry,
        // Clouds
        801..=802 => WeatherCondition::PartlyCloudy,
        803..=804 => WeatherCondition::Cloudy,
        _ => WeatherCondition::Dry,
    }
}

/// Convert OpenWeatherMap response to our WeatherForecast
fn convert_to_weather_forecast(
    current: &OpenWeatherCurrentResponse,
    predictions: Vec<WeatherPrediction>,
) -> WeatherForecast {
    let rain_intensity = current
        .rain
        .as_ref()
        .and_then(|r| r.one_hour.or(r.three_hour.map(|v| v / 3.0)))
        .unwrap_or(0.0);

    let weather_id = current.weather.first().map(|w| w.id).unwrap_or(800);
    let overall_condition = convert_weather_condition(weather_id, rain_intensity);

    // Estimate track temperature (usually 5-15°C higher than air temp)
    let temp_delta = if overall_condition == WeatherCondition::Dry {
        10.0
    } else {
        5.0
    };
    let track_temperature = current.main.temp + temp_delta;

    // Calculate rain probability from clouds and current conditions
    let rain_probability = if rain_intensity > 0.0 {
        1.0
    } else {
        (current.clouds.all / 100.0) * 0.5 // Up to 50% based on cloud coverage
    };

    // Create sector conditions (simplified - same for all sectors)
    let grip_level = calculate_grip_level(&overall_condition, track_temperature);
    let sector_conditions = vec![
        SectorWeather {
            sector: Sector::Sector1,
            condition: overall_condition,
            rain_intensity,
            track_temp: track_temperature,
            grip_level,
        },
        SectorWeather {
            sector: Sector::Sector2,
            condition: overall_condition,
            rain_intensity,
            track_temp: track_temperature,
            grip_level,
        },
        SectorWeather {
            sector: Sector::Sector3,
            condition: overall_condition,
            rain_intensity,
            track_temp: track_temperature,
            grip_level,
        },
    ];

    WeatherForecast {
        overall_condition,
        air_temperature: current.main.temp,
        track_temperature,
        humidity: current.main.humidity / 100.0,
        wind_speed: current.wind.speed * 3.6, // m/s to km/h
        wind_direction: current.wind.deg,
        rain_probability,
        rainfall_intensity: rain_intensity,
        sector_conditions,
        predictions,
    }
}

/// Convert forecast data to predictions
fn convert_forecast_predictions(forecast: &OpenWeatherForecastResponse) -> Vec<WeatherPrediction> {
    let now = chrono::Utc::now().timestamp();

    forecast
        .list
        .iter()
        .map(|item| {
            let minutes_ahead = ((item.dt - now) / 60).max(0) as u16;

            let rain_intensity = item
                .rain
                .as_ref()
                .and_then(|r| r.three_hour.map(|v| v / 3.0))
                .unwrap_or(0.0);

            let weather_id = item.weather.first().map(|w| w.id).unwrap_or(800);
            let condition = convert_weather_condition(weather_id, rain_intensity);

            WeatherPrediction {
                minutes_ahead,
                condition,
                rain_probability: item.pop,
                confidence: 0.7, // OpenWeatherMap doesn't provide explicit confidence
            }
        })
        .collect()
}

/// Calculate grip level based on conditions
fn calculate_grip_level(condition: &WeatherCondition, track_temp: f32) -> f32 {
    let base_grip = match condition {
        WeatherCondition::Dry => 1.0,
        WeatherCondition::PartlyCloudy => 0.95,
        WeatherCondition::Cloudy => 0.90,
        WeatherCondition::LightRain => 0.60,
        WeatherCondition::HeavyRain => 0.35,
    };

    // Adjust for track temperature (optimal around 25-35°C)
    let temp_factor = if track_temp >= 25.0 && track_temp <= 35.0 {
        1.0
    } else if track_temp < 25.0 {
        0.95 - (25.0 - track_temp) * 0.01
    } else {
        0.95 - (track_temp - 35.0) * 0.005
    };

    (base_grip * temp_factor).max(0.3).min(1.0)
}

/// Get coordinates for known F1 circuits
fn get_circuit_coordinates(circuit_name: &str) -> Option<(f32, f32)> {
    match circuit_name.to_lowercase().as_str() {
        "monaco" => Some((43.7347, 7.4206)),
        "spa" | "spa-francorchamps" => Some((50.4372, 5.9714)),
        "silverstone" => Some((52.0733, -1.0167)),
        "monza" => Some((45.6156, 9.2811)),
        "suzuka" => Some((34.8431, 136.5408)),
        "interlagos" | "brazil" => Some((-23.7036, -46.6997)),
        "austin" | "cota" => Some((30.1328, -97.6411)),
        "barcelona" | "catalunya" => Some((41.5700, 2.2611)),
        "red-bull-ring" | "austria" => Some((47.2197, 14.7647)),
        "hungaroring" | "hungary" => Some((47.5789, 19.2486)),
        "zandvoort" | "netherlands" => Some((52.3888, 4.5409)),
        "singapore" => Some((1.2914, 103.8644)),
        "jeddah" | "saudi-arabia" => Some((21.6319, 39.1044)),
        "bahrain" | "sakhir" => Some((26.0325, 50.5106)),
        "imola" => Some((44.3439, 11.7167)),
        "miami" => Some((25.9581, -80.2389)),
        "baku" | "azerbaijan" => Some((40.3725, 49.8533)),
        "canada" | "montreal" => Some((45.5000, -73.5228)),
        "paul-ricard" | "france" => Some((43.2506, 5.7919)),
        "portimao" | "portugal" => Some((37.2272, -8.6267)),
        "mexico" => Some((19.4042, -99.0907)),
        "yas-marina" | "abu-dhabi" => Some((24.4672, 54.6031)),
        "las-vegas" => Some((36.1147, -115.1728)),
        "qatar" | "losail" => Some((25.4900, 51.4542)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_condition_conversion() {
        assert_eq!(convert_weather_condition(800, 0.0), WeatherCondition::Dry);
        assert_eq!(convert_weather_condition(801, 0.0), WeatherCondition::PartlyCloudy);
        assert_eq!(convert_weather_condition(803, 0.0), WeatherCondition::Cloudy);
        assert_eq!(convert_weather_condition(500, 1.0), WeatherCondition::LightRain);
        assert_eq!(convert_weather_condition(501, 3.0), WeatherCondition::LightRain);
        assert_eq!(convert_weather_condition(502, 10.0), WeatherCondition::HeavyRain);
    }

    #[test]
    fn test_circuit_coordinates() {
        assert!(get_circuit_coordinates("monaco").is_some());
        assert!(get_circuit_coordinates("spa").is_some());
        assert!(get_circuit_coordinates("silverstone").is_some());
        assert!(get_circuit_coordinates("unknown").is_none());
    }

    #[test]
    fn test_grip_level_calculation() {
        let dry_grip = calculate_grip_level(&WeatherCondition::Dry, 30.0);
        assert!((dry_grip - 1.0).abs() < 0.01);

        let rain_grip = calculate_grip_level(&WeatherCondition::HeavyRain, 25.0);
        assert!(rain_grip < 0.5);
    }
}
