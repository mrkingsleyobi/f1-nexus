//! F1 Nexus Physics - Aerodynamics Calculations
//!
//! This module provides accurate aerodynamic calculations for Formula 1 cars,
//! including downforce, drag, ground effect, and DRS (Drag Reduction System) modeling.

use f1_nexus_core::telemetry::{AeroData, DrsStatus};
use serde::{Deserialize, Serialize};

/// Physical constants for aerodynamic calculations
pub mod constants {
    /// Standard air density at sea level (kg/m³)
    pub const AIR_DENSITY_SEA_LEVEL: f32 = 1.225;

    /// Temperature lapse rate (°C per meter)
    pub const TEMPERATURE_LAPSE_RATE: f32 = 0.0065;

    /// Standard temperature at sea level (°C)
    pub const STANDARD_TEMP_SEA_LEVEL: f32 = 15.0;

    /// Gas constant for dry air (J/(kg·K))
    pub const GAS_CONSTANT: f32 = 287.05;

    /// Gravitational acceleration (m/s²)
    pub const GRAVITY: f32 = 9.81;

    /// Typical F1 car frontal area (m²)
    pub const F1_FRONTAL_AREA: f32 = 1.6;

    /// Typical F1 car wing area (m²)
    pub const F1_WING_AREA: f32 = 1.5;

    /// DRS drag reduction percentage when active
    pub const DRS_DRAG_REDUCTION: f32 = 0.15; // 15% reduction

    /// Ground effect coefficient (increases with proximity to ground)
    /// Reduced to keep total downforce realistic
    pub const GROUND_EFFECT_COEFFICIENT: f32 = 0.15;

    /// Minimum ride height for ground effect (mm)
    pub const MIN_RIDE_HEIGHT: f32 = 75.0;

    /// Maximum ride height for ground effect benefit (mm)
    pub const MAX_RIDE_HEIGHT: f32 = 150.0;
}

/// Wing configuration for aerodynamic calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingConfig {
    /// Front wing angle in degrees (0-20°)
    pub front_wing_angle: f32,

    /// Rear wing angle in degrees (0-25°)
    pub rear_wing_angle: f32,

    /// Current ride height in millimeters
    pub ride_height: f32,
}

impl WingConfig {
    /// Create a new wing configuration
    pub fn new(front_wing_angle: f32, rear_wing_angle: f32, ride_height: f32) -> Self {
        Self {
            front_wing_angle: front_wing_angle.clamp(0.0, 20.0),
            rear_wing_angle: rear_wing_angle.clamp(0.0, 25.0),
            ride_height: ride_height.clamp(50.0, 200.0),
        }
    }

    /// Create configuration from telemetry AeroData
    pub fn from_aero_data(aero: &AeroData, ride_height: f32) -> Self {
        Self::new(aero.front_wing_angle, aero.rear_wing_angle, ride_height)
    }
}

/// Aerodynamics model for F1 cars
#[derive(Debug, Clone)]
pub struct AerodynamicsModel {
    /// Air density at current conditions (kg/m³)
    air_density: f32,

    /// Frontal area of the car (m²)
    frontal_area: f32,

    /// Wing area (m²)
    wing_area: f32,
}

impl Default for AerodynamicsModel {
    fn default() -> Self {
        Self::new(
            constants::AIR_DENSITY_SEA_LEVEL,
            constants::F1_FRONTAL_AREA,
            constants::F1_WING_AREA,
        )
    }
}

impl AerodynamicsModel {
    /// Create a new aerodynamics model with custom parameters
    pub fn new(air_density: f32, frontal_area: f32, wing_area: f32) -> Self {
        Self {
            air_density,
            frontal_area,
            wing_area,
        }
    }

    /// Create model with air density adjusted for altitude
    ///
    /// Uses the barometric formula to calculate air density at different altitudes.
    /// # Arguments
    /// * `altitude_meters` - Altitude above sea level in meters
    /// * `temperature_celsius` - Ambient temperature in Celsius
    pub fn with_altitude(altitude_meters: f32, temperature_celsius: f32) -> Self {
        let air_density = Self::calculate_air_density(altitude_meters, temperature_celsius);
        Self::new(air_density, constants::F1_FRONTAL_AREA, constants::F1_WING_AREA)
    }

    /// Calculate air density at a given altitude using barometric formula
    ///
    /// Uses the International Standard Atmosphere (ISA) model:
    /// ρ = ρ₀ × (1 - L×h/T₀)^((g×M)/(R×L) - 1)
    /// where:
    /// - ρ₀ = sea level density
    /// - L = temperature lapse rate
    /// - h = altitude
    /// - T₀ = sea level temperature
    /// - g = gravitational acceleration
    /// - M = molar mass of air (0.029 kg/mol)
    /// - R = universal gas constant (8.314 J/(mol·K))
    pub fn calculate_air_density(altitude_meters: f32, _temperature_celsius: f32) -> f32 {
        let sea_level_temp_kelvin = constants::STANDARD_TEMP_SEA_LEVEL + 273.15;

        // Temperature ratio at altitude using standard lapse rate
        let temp_ratio = 1.0 - (constants::TEMPERATURE_LAPSE_RATE * altitude_meters / sea_level_temp_kelvin);

        // Barometric formula exponent: (g*M)/(R*L) where M=0.029, R=8.314
        // This simplifies to approximately g/(R_specific * L) = 9.81/(287.05 * 0.0065) ≈ 5.256
        let exponent = (constants::GRAVITY / (constants::GAS_CONSTANT * constants::TEMPERATURE_LAPSE_RATE)) - 1.0;

        // Calculate density using barometric formula
        if temp_ratio > 0.0 {
            constants::AIR_DENSITY_SEA_LEVEL * temp_ratio.powf(exponent)
        } else {
            // For very high altitudes, use exponential approximation
            constants::AIR_DENSITY_SEA_LEVEL * (-altitude_meters / 8500.0).exp()
        }
    }

    /// Calculate total downforce generated by the car
    ///
    /// Formula: F_downforce = 0.5 × ρ × v² × A × C_L
    /// where:
    /// - ρ = air density
    /// - v = velocity (m/s)
    /// - A = wing area
    /// - C_L = lift coefficient (function of wing angles)
    ///
    /// # Arguments
    /// * `speed_kmh` - Speed in km/h
    /// * `wing_config` - Wing configuration
    /// * `drs_active` - Whether DRS is active
    ///
    /// # Returns
    /// Downforce in Newtons (negative value, as it pushes car down)
    pub fn calculate_downforce(
        &self,
        speed_kmh: f32,
        wing_config: &WingConfig,
        drs_active: DrsStatus,
    ) -> f32 {
        // Convert speed to m/s
        let speed_ms = speed_kmh / 3.6;

        // Calculate lift coefficient from wing angles
        let mut lift_coefficient = self.calculate_lift_coefficient(wing_config);

        // Apply DRS reduction if active (reduces rear downforce)
        if drs_active == DrsStatus::Activated {
            lift_coefficient *= 1.0 - (constants::DRS_DRAG_REDUCTION * 1.5); // DRS reduces downforce more than drag
        }

        // Add ground effect contribution
        let ground_effect = self.calculate_ground_effect(wing_config.ride_height, speed_ms);

        // Calculate dynamic pressure: 0.5 × ρ × v²
        let dynamic_pressure = 0.5 * self.air_density * speed_ms.powi(2);

        // Total downforce (negative because it pushes down)
        let base_downforce = dynamic_pressure * self.wing_area * lift_coefficient;

        -(base_downforce + ground_effect)
    }

    /// Calculate lift coefficient from wing angles
    ///
    /// C_L = k_front × α_front + k_rear × α_rear
    /// where k values are empirical constants for F1 wings
    fn calculate_lift_coefficient(&self, wing_config: &WingConfig) -> f32 {
        // Convert angles to radians
        let front_rad = wing_config.front_wing_angle.to_radians();
        let rear_rad = wing_config.rear_wing_angle.to_radians();

        // Empirical coefficients for F1 wings (adjusted for realistic downforce)
        // Modern F1 cars generate ~3000-4000N at 300 km/h
        let front_coefficient = 1.2; // Front wing contribution
        let rear_coefficient = 1.8; // Rear wing contribution (higher due to larger effect)

        // Total lift coefficient
        (front_coefficient * front_rad.sin()) + (rear_coefficient * rear_rad.sin())
    }

    /// Calculate ground effect contribution to downforce
    ///
    /// Ground effect increases exponentially as the car gets closer to the ground,
    /// creating a venturi effect that increases downforce.
    ///
    /// # Arguments
    /// * `ride_height_mm` - Ride height in millimeters
    /// * `speed_ms` - Speed in m/s
    fn calculate_ground_effect(&self, ride_height_mm: f32, speed_ms: f32) -> f32 {
        // Ground effect is most effective between 75-150mm ride height
        let normalized_height = ((ride_height_mm - constants::MIN_RIDE_HEIGHT)
            / (constants::MAX_RIDE_HEIGHT - constants::MIN_RIDE_HEIGHT))
            .clamp(0.0, 1.0);

        // Ground effect peaks at optimal ride height (around 100mm)
        let effect_multiplier = if ride_height_mm < 100.0 {
            normalized_height
        } else {
            1.0 - (normalized_height - 0.33).max(0.0) * 1.5
        };

        // Ground effect is proportional to velocity squared
        let dynamic_pressure = 0.5 * self.air_density * speed_ms.powi(2);

        dynamic_pressure * self.frontal_area * constants::GROUND_EFFECT_COEFFICIENT * effect_multiplier
    }

    /// Calculate drag force on the car
    ///
    /// Formula: F_drag = 0.5 × ρ × v² × A × C_D
    /// where:
    /// - ρ = air density
    /// - v = velocity (m/s)
    /// - A = frontal area
    /// - C_D = drag coefficient
    ///
    /// # Arguments
    /// * `speed_kmh` - Speed in km/h
    /// * `config` - Wing configuration
    ///
    /// # Returns
    /// Drag force in Newtons
    pub fn calculate_drag(&self, speed_kmh: f32, config: &WingConfig) -> f32 {
        // Convert speed to m/s
        let speed_ms = speed_kmh / 3.6;

        // Calculate drag coefficient from wing configuration
        let drag_coefficient = self.calculate_drag_coefficient(config);

        // Calculate dynamic pressure: 0.5 × ρ × v²
        let dynamic_pressure = 0.5 * self.air_density * speed_ms.powi(2);

        // Total drag force
        dynamic_pressure * self.frontal_area * drag_coefficient
    }

    /// Calculate drag coefficient with DRS consideration
    ///
    /// C_D = C_D_base + k × (α_front + α_rear)
    pub fn calculate_drag_coefficient(&self, config: &WingConfig) -> f32 {
        // Base drag coefficient for F1 car (body, wheels, etc.)
        let base_drag = 0.70;

        // Additional drag from wing angles (higher angles = more drag)
        let wing_drag_factor = 0.015; // Empirical constant

        let total_wing_angle = config.front_wing_angle + config.rear_wing_angle;
        let wing_drag = wing_drag_factor * total_wing_angle;

        base_drag + wing_drag
    }

    /// Calculate drag coefficient with DRS status
    pub fn calculate_drag_with_drs(&self, speed_kmh: f32, config: &WingConfig, drs: DrsStatus) -> f32 {
        let mut drag = self.calculate_drag(speed_kmh, config);

        // Apply DRS reduction if active
        if drs == DrsStatus::Activated {
            drag *= 1.0 - constants::DRS_DRAG_REDUCTION;
        }

        drag
    }

    /// Calculate maximum corner speed based on downforce and grip
    ///
    /// Formula: v_max = √(μ × (m×g + F_downforce) × r / m)
    /// where:
    /// - μ = coefficient of friction (tire grip)
    /// - m = car mass
    /// - g = gravitational acceleration
    /// - F_downforce = downforce at speed
    /// - r = corner radius
    ///
    /// # Arguments
    /// * `radius_meters` - Corner radius in meters
    /// * `downforce_newtons` - Downforce in Newtons (positive value)
    /// * `grip_coefficient` - Tire grip coefficient (0.0-2.5 for F1)
    ///
    /// # Returns
    /// Maximum corner speed in km/h
    pub fn calculate_corner_speed(
        &self,
        radius_meters: f32,
        downforce_newtons: f32,
        grip_coefficient: f32,
    ) -> f32 {
        // Typical F1 car mass (kg) - minimum weight with driver
        let car_mass = 798.0;

        // Weight force
        let weight_force = car_mass * constants::GRAVITY;

        // Total normal force (weight + downforce)
        let normal_force = weight_force + downforce_newtons.abs();

        // Maximum lateral force available from tires
        let max_lateral_force = grip_coefficient * normal_force;

        // Centripetal acceleration: a = v² / r
        // Maximum speed: v = √(a × r) = √((F/m) × r)
        let speed_ms = ((max_lateral_force / car_mass) * radius_meters).sqrt();

        // Convert to km/h
        speed_ms * 3.6
    }

    /// Calculate optimal wing angles for a given speed and corner
    ///
    /// Balances downforce for corner grip vs. drag for straight-line speed
    pub fn optimize_wing_angles(
        &self,
        target_speed_kmh: f32,
        corner_radius: f32,
        _grip_coefficient: f32,
    ) -> WingConfig {
        // For high-speed sections, minimize drag (lower wing angles)
        // For slow corners, maximize downforce (higher wing angles)

        let speed_factor = (target_speed_kmh / 300.0).clamp(0.0, 1.0);

        // High speed = low angles, low speed = high angles
        let front_wing_angle = 20.0 * (1.0 - speed_factor * 0.7);
        let rear_wing_angle = 25.0 * (1.0 - speed_factor * 0.6);

        // Consider corner radius (tight corners need more downforce)
        let corner_factor = (100.0 / corner_radius).clamp(0.0, 1.0);
        let front_wing_angle = front_wing_angle + (corner_factor * 5.0);
        let rear_wing_angle = rear_wing_angle + (corner_factor * 6.0);

        WingConfig::new(front_wing_angle, rear_wing_angle, 100.0)
    }

    /// Get the current air density
    pub fn air_density(&self) -> f32 {
        self.air_density
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_density_at_sea_level() {
        let density = AerodynamicsModel::calculate_air_density(0.0, 15.0);
        assert!((density - constants::AIR_DENSITY_SEA_LEVEL).abs() < 0.1);
    }

    #[test]
    fn test_air_density_at_altitude() {
        // Mexico City circuit is at ~2,200m altitude
        let density = AerodynamicsModel::calculate_air_density(2200.0, 20.0);
        // Air density should be significantly lower than sea level
        assert!(density < constants::AIR_DENSITY_SEA_LEVEL);
        assert!(density > 0.9); // Reasonable bounds
    }

    #[test]
    fn test_downforce_increases_with_speed() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        let downforce_100 = model.calculate_downforce(100.0, &config, DrsStatus::Unavailable);
        let downforce_200 = model.calculate_downforce(200.0, &config, DrsStatus::Unavailable);
        let downforce_300 = model.calculate_downforce(300.0, &config, DrsStatus::Unavailable);

        // Downforce should increase with speed (more negative)
        assert!(downforce_100 > downforce_200);
        assert!(downforce_200 > downforce_300);

        // Downforce should be proportional to v² (approximately)
        let ratio = downforce_200 / downforce_100;
        assert!((ratio - 4.0).abs() < 0.5); // Should be close to 4x (2² = 4)
    }

    #[test]
    fn test_drs_reduces_downforce() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        let downforce_normal = model.calculate_downforce(250.0, &config, DrsStatus::Available);
        let downforce_drs = model.calculate_downforce(250.0, &config, DrsStatus::Activated);

        // DRS should reduce downforce (make it less negative)
        assert!(downforce_drs > downforce_normal);
        assert!(downforce_drs.abs() < downforce_normal.abs());
    }

    #[test]
    fn test_realistic_downforce_values() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        // At 300 km/h, modern F1 cars generate ~3000-4000N of downforce
        let downforce = model.calculate_downforce(300.0, &config, DrsStatus::Unavailable);

        assert!(downforce < -2000.0, "Downforce too low at 300 km/h");
        assert!(downforce > -6000.0, "Downforce too high at 300 km/h");
    }

    #[test]
    fn test_drag_increases_with_speed() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        let drag_100 = model.calculate_drag(100.0, &config);
        let drag_200 = model.calculate_drag(200.0, &config);

        // Drag should increase with speed squared
        assert!(drag_200 > drag_100);

        // Should be approximately 4x (2² = 4)
        let ratio = drag_200 / drag_100;
        assert!((ratio - 4.0).abs() < 0.5);
    }

    #[test]
    fn test_drs_reduces_drag() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        let drag_normal = model.calculate_drag(250.0, &config);
        let drag_drs = model.calculate_drag_with_drs(250.0, &config, DrsStatus::Activated);

        // DRS should reduce drag by ~15%
        assert!(drag_drs < drag_normal);
        let reduction = (drag_normal - drag_drs) / drag_normal;
        assert!((reduction - constants::DRS_DRAG_REDUCTION).abs() < 0.01);
    }

    #[test]
    fn test_realistic_drag_coefficient() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        let cd = model.calculate_drag_coefficient(&config);

        // F1 cars typically have Cd between 0.7 and 1.1
        assert!(cd > 0.6, "Drag coefficient too low");
        assert!(cd < 1.2, "Drag coefficient too high");
    }

    #[test]
    fn test_corner_speed_increases_with_downforce() {
        let model = AerodynamicsModel::default();
        let radius = 50.0; // 50m radius corner
        let grip = 1.8; // Typical F1 tire grip

        let speed_low_df = model.calculate_corner_speed(radius, 1000.0, grip);
        let speed_high_df = model.calculate_corner_speed(radius, 3000.0, grip);

        // More downforce should allow higher corner speeds
        assert!(speed_high_df > speed_low_df);
    }

    #[test]
    fn test_realistic_corner_speeds() {
        let model = AerodynamicsModel::default();
        let config = WingConfig::new(15.0, 12.0, 100.0);

        // High-speed corner (100m radius) at 250 km/h
        let downforce = model.calculate_downforce(250.0, &config, DrsStatus::Unavailable).abs();
        let corner_speed = model.calculate_corner_speed(100.0, downforce, 1.8);

        // Should be able to take a 100m radius corner at ~200-250 km/h with downforce
        assert!(corner_speed > 180.0, "Corner speed too low");
        assert!(corner_speed < 280.0, "Corner speed too high");
    }

    #[test]
    fn test_ground_effect_optimal_ride_height() {
        let model = AerodynamicsModel::default();

        // Test different ride heights at same speed
        let speed_ms = 250.0 / 3.6; // 250 km/h in m/s

        let effect_low = model.calculate_ground_effect(75.0, speed_ms); // Minimum height
        let effect_optimal = model.calculate_ground_effect(100.0, speed_ms); // Optimal
        let effect_high = model.calculate_ground_effect(150.0, speed_ms); // High

        // Optimal ride height should provide good ground effect
        assert!(effect_optimal > 0.0);
        // Too low or too high should be less effective
        assert!(effect_optimal >= effect_low);
        assert!(effect_optimal > effect_high);
    }

    #[test]
    fn test_wing_config_clamping() {
        // Test that extreme values are clamped
        let config = WingConfig::new(100.0, 100.0, 500.0);

        assert!(config.front_wing_angle <= 20.0);
        assert!(config.rear_wing_angle <= 25.0);
        assert!(config.ride_height >= 50.0 && config.ride_height <= 200.0);
    }

    #[test]
    fn test_wing_config_from_aero_data() {
        let aero_data = AeroData {
            front_wing_angle: 15.0,
            rear_wing_angle: 12.0,
            downforce: 3000.0,
            drag_coefficient: 0.85,
        };

        let config = WingConfig::from_aero_data(&aero_data, 100.0);

        assert_eq!(config.front_wing_angle, 15.0);
        assert_eq!(config.rear_wing_angle, 12.0);
        assert_eq!(config.ride_height, 100.0);
    }

    #[test]
    fn test_optimize_wing_angles_for_high_speed() {
        let model = AerodynamicsModel::default();

        // High speed straight (300 km/h, large radius)
        let config = model.optimize_wing_angles(300.0, 500.0, 1.5);

        // Should have relatively low wing angles for low drag
        assert!(config.front_wing_angle < 15.0);
        assert!(config.rear_wing_angle < 18.0);
    }

    #[test]
    fn test_optimize_wing_angles_for_slow_corner() {
        let model = AerodynamicsModel::default();

        // Slow corner (100 km/h, tight radius)
        let config = model.optimize_wing_angles(100.0, 30.0, 1.8);

        // Should have higher wing angles for more downforce
        assert!(config.front_wing_angle > 15.0);
        assert!(config.rear_wing_angle > 18.0);
    }

    #[test]
    fn test_altitude_effects() {
        // Sea level model
        let model_sea_level = AerodynamicsModel::default();

        // High altitude model (e.g., Mexico City at 2200m)
        let model_altitude = AerodynamicsModel::with_altitude(2200.0, 20.0);

        let config = WingConfig::new(15.0, 12.0, 100.0);

        let downforce_sea_level = model_sea_level.calculate_downforce(250.0, &config, DrsStatus::Unavailable);
        let downforce_altitude = model_altitude.calculate_downforce(250.0, &config, DrsStatus::Unavailable);

        // Lower air density at altitude = less downforce (less negative)
        assert!(downforce_altitude > downforce_sea_level);
        assert!(model_altitude.air_density() < model_sea_level.air_density());
    }

    #[test]
    fn test_complete_aerodynamics_scenario() {
        // Simulate a complete lap scenario
        let model = AerodynamicsModel::with_altitude(600.0, 25.0); // Monaco-like conditions
        let config = WingConfig::new(18.0, 15.0, 95.0); // High downforce setup

        // Slow corner entry (100 km/h)
        let downforce_slow = model.calculate_downforce(100.0, &config, DrsStatus::Unavailable);
        let drag_slow = model.calculate_drag(100.0, &config);

        // Fast straight (300 km/h with DRS)
        let downforce_fast = model.calculate_downforce(300.0, &config, DrsStatus::Activated);
        let drag_fast = model.calculate_drag_with_drs(300.0, &config, DrsStatus::Activated);

        // Basic sanity checks
        assert!(downforce_slow.abs() > 0.0);
        assert!(downforce_fast.abs() > downforce_slow.abs());
        assert!(drag_fast > drag_slow);

        // Calculate corner speed with downforce
        let corner_speed = model.calculate_corner_speed(40.0, downforce_slow.abs(), 1.9);
        assert!(corner_speed > 50.0 && corner_speed < 200.0);
    }
}
