use criterion::{black_box, criterion_group, criterion_main, Criterion};
use f1_nexus_core::*;

fn bench_circuit_data(c: &mut Criterion) {
    c.bench_function("circuit_monaco", |b| {
        b.iter(|| {
            let circuit = Circuit::monaco();
            black_box(circuit.race_distance_km())
        })
    });
}

fn bench_tire_characteristics(c: &mut Criterion) {
    c.bench_function("tire_characteristics", |b| {
        b.iter(|| {
            let chars = TireCharacteristics::for_compound(black_box(TireCompound::C3));
            black_box(chars.grip_multiplier_for_temp(100.0))
        })
    });
}

criterion_group!(benches, bench_circuit_data, bench_tire_characteristics);
criterion_main!(benches);
