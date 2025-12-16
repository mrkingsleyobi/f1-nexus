//! Physics-based lap time prediction using telemetry data
//!
//! Predicts lap times using real-time telemetry, track characteristics,
//! tire degradation models, and physics calculations.

use f1_nexus_core::{
    Circuit, TelemetrySnapshot, TireCompound, TireCharacteristics,
    WeatherCondition, FuelConsumptionModel,
};
use serde::{Deserialize, Serialize};

/// Lap time predictor using physics-based models
#[derive(Debug, Clone)]
pub struct LapTimePredictor {
    /// Circuit information
    circuit: Circuit,

    /// Fuel consumption model
    fuel_model: FuelConsumptionModel,

    /// Current weather conditions
    weather: WeatherCondition,

    /// Track temperature (°C)
    track_temp: f32,

    /// Air temperature (°C)
    air_temp: f32,
}

/// Lap time prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapTimePrediction {
    /// Predicted lap time (seconds)
    pub predicted_time: f32,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,

    /// Breakdown of time components
    pub breakdown: TimeBreakdown,

    /// Factors affecting prediction
    pub factors: PredictionFactors,
}

/// Time breakdown by component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBreakdown {
    /// Base lap time from circuit characteristics
    pub base_time: f32,

    /// Time penalty from tire degradation
    pub tire_degradation: f32,

    /// Time penalty from fuel load
    pub fuel_load: f32,

    /// Time penalty from weather conditions
    pub weather_penalty: f32,

    /// Time bonus/penalty from tire temperature
    pub tire_temp_delta: f32,

    /// Time bonus/penalty from aerodynamic efficiency
    pub aero_delta: f32,

    /// Time bonus/penalty from power unit performance
    pub power_unit_delta: f32,
}

/// Factors affecting the prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionFactors {
    /// Tire age (laps)
    pub tire_age: u16,

    /// Tire compound
    pub compound: TireCompound,

    /// Current fuel load (kg)
    pub fuel_load: f32,

    /// Average tire temperature (°C)
    pub avg_tire_temp: f32,

    /// ERS battery level (0.0 to 1.0)
    pub ers_battery: f32,

    /// Current speed (km/h)
    pub current_speed: f32,
}

impl LapTimePredictor {
    /// Create a new lap time predictor
    pub fn new(
        circuit: Circuit,
        fuel_model: FuelConsumptionModel,
        weather: WeatherCondition,
        track_temp: f32,
        air_temp: f32,
    ) -> Self {
        LapTimePredictor {
            circuit,
            fuel_model,
            weather,
            track_temp,
            air_temp,
        }
    }

    /// Predict lap time from telemetry snapshot
    pub fn predict(&self, snapshot: &TelemetrySnapshot, tire_age: u16) -> LapTimePrediction {
        let tire_chars = TireCharacteristics::for_compound(snapshot.tires.compound);

        // Calculate base lap time (3% slower than lap record for racing conditions)
        let base_time = self.circuit.lap_record * 1.03;

        // Calculate tire degradation penalty
        let tire_degradation = self.calculate_tire_degradation(
            tire_age,
            &tire_chars,
            snapshot.tires.compound,
        );

        // Calculate fuel load penalty
        let fuel_load = self.calculate_fuel_penalty(snapshot.fuel.remaining);

        // Calculate weather penalty
        let weather_penalty = self.calculate_weather_penalty(snapshot.tires.compound);

        // Calculate tire temperature delta
        let tire_temp_delta = self.calculate_tire_temp_delta(
            snapshot,
            &tire_chars,
        );

        // Calculate aerodynamic efficiency delta
        let aero_delta = self.calculate_aero_delta(snapshot);

        // Calculate power unit performance delta
        let power_unit_delta = self.calculate_power_unit_delta(snapshot);

        let breakdown = TimeBreakdown {
            base_time,
            tire_degradation,
            fuel_load,
            weather_penalty,
            tire_temp_delta,
            aero_delta,
            power_unit_delta,
        };

        let predicted_time = base_time
            + tire_degradation
            + fuel_load
            + weather_penalty
            + tire_temp_delta
            + aero_delta
            + power_unit_delta;

        // Calculate confidence based on data quality
        let confidence = self.calculate_confidence(snapshot, tire_age);

        let factors = PredictionFactors {
            tire_age,
            compound: snapshot.tires.compound,
            fuel_load: snapshot.fuel.remaining,
            avg_tire_temp: self.average_tire_temp(snapshot),
            ers_battery: snapshot.power_unit.ers_battery,
            current_speed: snapshot.motion.speed,
        };

        LapTimePrediction {
            predicted_time,
            confidence,
            breakdown,
            factors,
        }
    }

    /// Calculate tire degradation penalty
    fn calculate_tire_degradation(
        &self,
        tire_age: u16,
        tire_chars: &TireCharacteristics,
        compound: TireCompound,
    ) -> f32 {
        if tire_age == 0 {
            return 0.0;
        }

        // Calculate wear ratio
        let wear_ratio = tire_age as f32 / tire_chars.typical_life as f32;

        // Track severity multiplier
        let track_multiplier = self.circuit.characteristics.tire_severity;

        // Temperature effect on degradation
        let temp_multiplier = if self.track_temp > 40.0 {
            1.2 // High track temps accelerate degradation
        } else if self.track_temp < 20.0 {
            0.9 // Low temps reduce degradation
        } else {
            1.0
        };

        // Softer compounds degrade faster but are faster when fresh
        let compound_factor = match compound {
            TireCompound::C5 => 1.3,
            TireCompound::C4 => 1.2,
            TireCompound::C3 => 1.0,
            TireCompound::C2 => 0.8,
            TireCompound::C1 => 0.7,
            TireCompound::C0 => 0.6,
            TireCompound::Intermediate | TireCompound::Wet => 0.5,
        };

        // Exponential degradation curve (degradation accelerates)
        let degradation_curve = if wear_ratio > 0.8 {
            wear_ratio.powf(1.5) // Cliff after 80% life
        } else {
            wear_ratio
        };

        // Maximum penalty of 3 seconds per lap for completely worn tires
        degradation_curve * track_multiplier * temp_multiplier * compound_factor * 3.0
    }

    /// Calculate fuel load penalty
    fn calculate_fuel_penalty(&self, fuel_remaining: f32) -> f32 {
        // Fuel adds ~0.03s per lap per kg (approximately)
        // With 110kg max fuel, that's ~3.3s penalty at race start
        let fuel_factor = 0.03;
        fuel_remaining * fuel_factor
    }

    /// Calculate weather condition penalty
    fn calculate_weather_penalty(&self, compound: TireCompound) -> f32 {
        let is_dry_compound = matches!(
            compound,
            TireCompound::C0 | TireCompound::C1 | TireCompound::C2 |
            TireCompound::C3 | TireCompound::C4 | TireCompound::C5
        );

        match self.weather {
            WeatherCondition::Dry => 0.0,
            WeatherCondition::PartlyCloudy => 0.0,
            WeatherCondition::Cloudy => 0.0,
            WeatherCondition::LightRain => {
                if is_dry_compound {
                    // Wrong tire choice - massive penalty
                    10.0
                } else if compound == TireCompound::Intermediate {
                    // Correct choice
                    1.5 // Inters are ~1.5s slower than dry conditions
                } else {
                    // Wet tires in light rain
                    2.5
                }
            }
            WeatherCondition::HeavyRain => {
                if is_dry_compound {
                    // Extremely dangerous, huge penalty
                    20.0
                } else if compound == TireCompound::Wet {
                    // Correct choice
                    3.0 // Wets are ~3s slower than dry conditions
                } else {
                    // Inters in heavy rain - risky
                    5.0
                }
            }
        }
    }

    /// Calculate tire temperature delta
    fn calculate_tire_temp_delta(&self, snapshot: &TelemetrySnapshot, tire_chars: &TireCharacteristics) -> f32 {
        let avg_temp = self.average_tire_temp(snapshot);
        let (min_optimal, max_optimal) = tire_chars.optimal_temp_range;
        let optimal_temp = (min_optimal + max_optimal) / 2.0;

        let temp_diff = (avg_temp - optimal_temp).abs();

        if temp_diff < 5.0 {
            // Within 5°C of optimal - minimal penalty
            0.0
        } else if temp_diff < 10.0 {
            // 5-10°C off - small penalty
            0.1 * (temp_diff - 5.0)
        } else if temp_diff < 20.0 {
            // 10-20°C off - moderate penalty
            0.5 + 0.2 * (temp_diff - 10.0)
        } else {
            // >20°C off - severe penalty
            2.5 + 0.3 * (temp_diff - 20.0)
        }
    }

    /// Calculate aerodynamic efficiency delta
    fn calculate_aero_delta(&self, snapshot: &TelemetrySnapshot) -> f32 {
        // Higher downforce = more grip but more drag
        let downforce = snapshot.aero.downforce;
        let drag_coefficient = snapshot.aero.drag_coefficient;

        // Optimal downforce for this circuit
        let optimal_downforce = self.circuit.characteristics.downforce_level * 15000.0;
        let downforce_diff = (downforce - optimal_downforce).abs();

        // Penalty for suboptimal downforce setup
        let downforce_penalty = if downforce_diff < 1000.0 {
            0.0
        } else {
            (downforce_diff - 1000.0) * 0.0001 // ~0.1s per 1000N off
        };

        // High drag penalty (affects straight-line speed)
        let drag_penalty = if drag_coefficient > 0.8 {
            (drag_coefficient - 0.8) * 2.0
        } else {
            0.0
        };

        downforce_penalty + drag_penalty
    }

    /// Calculate power unit performance delta
    fn calculate_power_unit_delta(&self, snapshot: &TelemetrySnapshot) -> f32 {
        let mut delta = 0.0;

        // ERS deployment bonus
        let ers_deployment = snapshot.power_unit.mgu_k_deployment;
        if ers_deployment > 100.0 {
            // Full deployment = faster lap
            delta -= 0.1 * (ers_deployment / 120.0);
        }

        // Engine temperature penalty (overheating reduces performance)
        if snapshot.power_unit.engine_temp > 110.0 {
            delta += (snapshot.power_unit.engine_temp - 110.0) * 0.02;
        }

        // Low ERS battery penalty (can't deploy optimally)
        if snapshot.power_unit.ers_battery < 0.3 {
            delta += (0.3 - snapshot.power_unit.ers_battery) * 0.5;
        }

        delta
    }

    /// Calculate prediction confidence
    fn calculate_confidence(&self, snapshot: &TelemetrySnapshot, tire_age: u16) -> f32 {
        let mut confidence: f32 = 1.0;

        // Reduce confidence for very fresh or very worn tires
        if tire_age < 2 {
            confidence *= 0.9; // Tires still warming up
        } else if tire_age > 30 {
            confidence *= 0.8; // Very worn, unpredictable
        }

        // Reduce confidence in changing weather
        if matches!(self.weather, WeatherCondition::LightRain) {
            confidence *= 0.85; // Weather can change quickly
        }

        // Reduce confidence if tire temps are far from optimal
        let avg_temp = self.average_tire_temp(snapshot);
        let tire_chars = TireCharacteristics::for_compound(snapshot.tires.compound);
        let (min_optimal, max_optimal) = tire_chars.optimal_temp_range;

        if avg_temp < min_optimal - 10.0 || avg_temp > max_optimal + 10.0 {
            confidence *= 0.85; // Tires not in optimal window
        }

        // Reduce confidence if fuel is very low (end of race, different behavior)
        if snapshot.fuel.remaining < 5.0 {
            confidence *= 0.9;
        }

        confidence.max(0.5).min(1.0)
    }

    /// Calculate average tire temperature
    fn average_tire_temp(&self, snapshot: &TelemetrySnapshot) -> f32 {
        (snapshot.tires.front_left.surface_temp
            + snapshot.tires.front_right.surface_temp
            + snapshot.tires.rear_left.surface_temp
            + snapshot.tires.rear_right.surface_temp) / 4.0
    }

    /// Update weather conditions
    pub fn update_weather(&mut self, weather: WeatherCondition, track_temp: f32, air_temp: f32) {
        self.weather = weather;
        self.track_temp = track_temp;
        self.air_temp = air_temp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use f1_nexus_core::{
        MotionData, TireData, TireSensor, PowerUnitData, AeroData, BrakeData,
        DriverInputs, FuelData, DrsStatus, SessionId, CarId, LapNumber, Position,
        TrackCharacteristics,
    };
    use f1_nexus_core::telemetry::ErsMode;
    use chrono::Utc;

    fn create_test_circuit() -> Circuit {
        Circuit {
            id: "monaco".to_string(),
            name: "Monaco".to_string(),
            country: "Monaco".to_string(),
            length: 3337.0,
            num_turns: 19,
            lap_record: 70.0,
            characteristics: TrackCharacteristics {
                tire_severity: 1.2,
                fuel_consumption: 0.9,
                overtaking_difficulty: 0.95,
                downforce_level: 0.9,
                average_speed: 160.0,
                maximum_speed: 290.0,
                elevation_change: 42.0,
                weather_variability: 0.3,
            },
            sectors: vec![],
            drs_zones: vec![],
            typical_race_laps: 78,
        }
    }

    fn create_test_snapshot(compound: TireCompound, fuel: f32, tire_temp: f32) -> TelemetrySnapshot {
        TelemetrySnapshot {
            session_id: SessionId::new(),
            car_id: CarId::new(1).unwrap(),
            timestamp: Utc::now(),
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
                    surface_temp: tire_temp,
                    inner_temp: tire_temp + 5.0,
                    brake_temp: 350.0,
                    pressure: 21.5,
                    wear: 0.1,
                    damage: 0.0,
                },
                front_right: TireSensor {
                    surface_temp: tire_temp,
                    inner_temp: tire_temp + 5.0,
                    brake_temp: 350.0,
                    pressure: 21.5,
                    wear: 0.1,
                    damage: 0.0,
                },
                rear_left: TireSensor {
                    surface_temp: tire_temp + 5.0,
                    inner_temp: tire_temp + 10.0,
                    brake_temp: 300.0,
                    pressure: 20.0,
                    wear: 0.15,
                    damage: 0.0,
                },
                rear_right: TireSensor {
                    surface_temp: tire_temp + 5.0,
                    inner_temp: tire_temp + 10.0,
                    brake_temp: 300.0,
                    pressure: 20.0,
                    wear: 0.15,
                    damage: 0.0,
                },
                compound,
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
                downforce: 13500.0,
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
                remaining: fuel,
                consumption_rate: 1.5,
                temperature: 45.0,
                pressure: 6.0,
            },
            drs: DrsStatus::Available,
        }
    }

    #[test]
    fn test_lap_time_prediction_basic() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();
        let predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let snapshot = create_test_snapshot(TireCompound::C3, 80.0, 95.0);
        let prediction = predictor.predict(&snapshot, 5);

        // Should predict a reasonable lap time (within race pace range)
        assert!(prediction.predicted_time > 70.0 && prediction.predicted_time < 80.0);
        assert!(prediction.confidence > 0.8);
    }

    #[test]
    fn test_tire_degradation_effect() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();
        let predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let snapshot = create_test_snapshot(TireCompound::C5, 80.0, 95.0);

        let fresh_prediction = predictor.predict(&snapshot, 0);
        let worn_prediction = predictor.predict(&snapshot, 20);

        // Worn tires should be slower
        assert!(worn_prediction.predicted_time > fresh_prediction.predicted_time);
        assert!(worn_prediction.breakdown.tire_degradation > fresh_prediction.breakdown.tire_degradation);
    }

    #[test]
    fn test_fuel_load_effect() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();
        let predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let heavy = create_test_snapshot(TireCompound::C3, 100.0, 95.0);
        let light = create_test_snapshot(TireCompound::C3, 20.0, 95.0);

        let heavy_prediction = predictor.predict(&heavy, 5);
        let light_prediction = predictor.predict(&light, 5);

        // Heavy fuel should be slower
        assert!(heavy_prediction.predicted_time > light_prediction.predicted_time);
        assert!(heavy_prediction.breakdown.fuel_load > light_prediction.breakdown.fuel_load);
    }

    #[test]
    fn test_weather_penalty() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();

        let dry_predictor = LapTimePredictor::new(
            circuit.clone(),
            fuel_model.clone(),
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let rain_predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::LightRain,
            25.0,
            20.0,
        );

        let snapshot = create_test_snapshot(TireCompound::C3, 80.0, 95.0);

        let dry_prediction = dry_predictor.predict(&snapshot, 5);
        let rain_prediction = rain_predictor.predict(&snapshot, 5);

        // Wrong tire in rain should be much slower
        assert!(rain_prediction.predicted_time > dry_prediction.predicted_time + 5.0);
    }

    #[test]
    fn test_tire_temperature_effect() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();
        let predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let optimal = create_test_snapshot(TireCompound::C3, 80.0, 95.0);
        let cold = create_test_snapshot(TireCompound::C3, 80.0, 70.0);

        let optimal_prediction = predictor.predict(&optimal, 5);
        let cold_prediction = predictor.predict(&cold, 5);

        // Cold tires should be slower
        assert!(cold_prediction.predicted_time > optimal_prediction.predicted_time);
    }

    #[test]
    fn test_confidence_calculation() {
        let circuit = create_test_circuit();
        let fuel_model = FuelConsumptionModel::default_model();
        let predictor = LapTimePredictor::new(
            circuit,
            fuel_model,
            WeatherCondition::Dry,
            30.0,
            25.0,
        );

        let snapshot = create_test_snapshot(TireCompound::C3, 80.0, 95.0);

        let normal_prediction = predictor.predict(&snapshot, 10);
        let fresh_prediction = predictor.predict(&snapshot, 0);
        let very_worn_prediction = predictor.predict(&snapshot, 35);

        // Confidence should be reasonable
        assert!(normal_prediction.confidence > 0.9);
        assert!(fresh_prediction.confidence < normal_prediction.confidence);
        assert!(very_worn_prediction.confidence < normal_prediction.confidence);
    }
}
