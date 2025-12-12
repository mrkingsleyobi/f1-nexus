//! Weather modeling and prediction

use crate::types::{Sector, WeatherCondition};
use serde::{Deserialize, Serialize};

/// Weather forecast for a race session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherForecast {
    /// Track-wide conditions
    pub overall_condition: WeatherCondition,

    /// Temperature (°C)
    pub air_temperature: f32,

    /// Track surface temperature (°C)
    pub track_temperature: f32,

    /// Humidity (0.0-1.0)
    pub humidity: f32,

    /// Wind speed (km/h)
    pub wind_speed: f32,

    /// Wind direction (degrees, 0-360)
    pub wind_direction: f32,

    /// Rain probability (0.0-1.0)
    pub rain_probability: f32,

    /// Expected rainfall intensity (mm/hour)
    pub rainfall_intensity: f32,

    /// Sector-specific conditions
    pub sector_conditions: Vec<SectorWeather>,

    /// Future predictions
    pub predictions: Vec<WeatherPrediction>,
}

/// Weather conditions per sector (microclimate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorWeather {
    pub sector: Sector,
    pub condition: WeatherCondition,
    pub rain_intensity: f32, // mm/hour
    pub track_temp: f32,     // °C
    pub grip_level: f32,     // 0.0-1.0
}

/// Future weather prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPrediction {
    /// Minutes from now
    pub minutes_ahead: u16,

    /// Predicted condition
    pub condition: WeatherCondition,

    /// Rain probability
    pub rain_probability: f32,

    /// Confidence in prediction (0.0-1.0)
    pub confidence: f32,
}

impl WeatherForecast {
    /// Check if any sector has rain
    pub fn has_rain_anywhere(&self) -> bool {
        self.sector_conditions.iter().any(|s| s.rain_intensity > 0.0)
    }

    /// Get maximum rain intensity across all sectors
    pub fn max_rain_intensity(&self) -> f32 {
        self.sector_conditions
            .iter()
            .map(|s| s.rain_intensity)
            .fold(0.0, f32::max)
    }

    /// Get average grip level across all sectors
    pub fn average_grip_level(&self) -> f32 {
        let sum: f32 = self.sector_conditions.iter().map(|s| s.grip_level).sum();
        sum / self.sector_conditions.len() as f32
    }

    /// Predict if rain will arrive in next N minutes
    pub fn rain_expected_in(&self, minutes: u16) -> bool {
        self.predictions
            .iter()
            .filter(|p| p.minutes_ahead <= minutes)
            .any(|p| p.rain_probability > 0.5)
    }

    /// Get recommended tire compound based on current conditions
    pub fn recommended_compound(&self) -> RecommendedTire {
        if self.max_rain_intensity() > 5.0 {
            RecommendedTire::Wet
        } else if self.max_rain_intensity() > 0.5 {
            RecommendedTire::Intermediate
        } else if self.rain_expected_in(10) && self.rain_probability > 0.7 {
            RecommendedTire::Intermediate
        } else {
            RecommendedTire::Dry
        }
    }
}

/// Recommended tire type based on weather
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedTire {
    Dry,
    Intermediate,
    Wet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_forecast() {
        let forecast = WeatherForecast {
            overall_condition: WeatherCondition::LightRain,
            air_temperature: 18.5,
            track_temperature: 22.0,
            humidity: 0.85,
            wind_speed: 15.0,
            wind_direction: 270.0,
            rain_probability: 0.75,
            rainfall_intensity: 2.5,
            sector_conditions: vec![
                SectorWeather {
                    sector: Sector::Sector1,
                    condition: WeatherCondition::LightRain,
                    rain_intensity: 2.0,
                    track_temp: 22.0,
                    grip_level: 0.65,
                },
                SectorWeather {
                    sector: Sector::Sector2,
                    condition: WeatherCondition::LightRain,
                    rain_intensity: 3.0,
                    track_temp: 21.5,
                    grip_level: 0.60,
                },
                SectorWeather {
                    sector: Sector::Sector3,
                    condition: WeatherCondition::Dry,
                    rain_intensity: 0.0,
                    track_temp: 23.0,
                    grip_level: 0.85,
                },
            ],
            predictions: vec![
                WeatherPrediction {
                    minutes_ahead: 5,
                    condition: WeatherCondition::LightRain,
                    rain_probability: 0.8,
                    confidence: 0.9,
                },
                WeatherPrediction {
                    minutes_ahead: 15,
                    condition: WeatherCondition::HeavyRain,
                    rain_probability: 0.9,
                    confidence: 0.75,
                },
            ],
        };

        assert!(forecast.has_rain_anywhere());
        assert_eq!(forecast.max_rain_intensity(), 3.0);
        assert!((forecast.average_grip_level() - 0.7).abs() < 0.01);
        assert!(forecast.rain_expected_in(10));
        assert_eq!(forecast.recommended_compound(), RecommendedTire::Intermediate);
    }
}
