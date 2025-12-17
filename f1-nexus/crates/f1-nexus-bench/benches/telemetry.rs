use criterion::{black_box, criterion_group, criterion_main, Criterion};
use f1_nexus_core::{*, telemetry::*};
use f1_nexus_core::telemetry::ErsMode;
use f1_nexus_telemetry::*;
use chrono::Utc;

fn create_test_snapshot() -> TelemetrySnapshot {
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

fn bench_telemetry_processing(c: &mut Criterion) {
    let processor = TelemetryProcessor::new(TelemetryConfig::default());
    let snapshot = create_test_snapshot();

    c.bench_function("telemetry_process", |b| {
        b.iter(|| {
            processor.process(black_box(&snapshot)).unwrap()
        })
    });
}

fn bench_anomaly_detection(c: &mut Criterion) {
    let detector = AnomalyDetector::new(TelemetryConfig::default());
    let snapshot = create_test_snapshot();

    c.bench_function("anomaly_detect", |b| {
        b.iter(|| {
            detector.detect(black_box(&snapshot)).unwrap()
        })
    });
}

criterion_group!(benches, bench_telemetry_processing, bench_anomaly_detection);
criterion_main!(benches);
