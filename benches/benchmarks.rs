//! Criterion benchmarks for naad synthesis primitives.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use naad::envelope::Adsr;
use naad::filter::{BiquadFilter, FilterType};
use naad::modulation::FmSynth;
use naad::oscillator::{Oscillator, Waveform};
use naad::wavetable::{Wavetable, WavetableOscillator};

fn oscillator_sine_1024(c: &mut Criterion) {
    c.bench_function("oscillator_sine_1024", |b| {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            osc.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn oscillator_saw_polyblep_1024(c: &mut Criterion) {
    c.bench_function("oscillator_saw_polyblep_1024", |b| {
        let mut osc = Oscillator::new(Waveform::Saw, 440.0, 44100.0).unwrap();
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            osc.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn biquad_filter_1024(c: &mut Criterion) {
    c.bench_function("biquad_filter_1024", |b| {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 44100.0, 1000.0, 0.707).unwrap();
        let mut buffer = [0.5f32; 1024];
        b.iter(|| {
            filter.process_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn adsr_envelope_1024(c: &mut Criterion) {
    c.bench_function("adsr_envelope_1024", |b| {
        let mut env = Adsr::new(0.01, 0.1, 0.7, 0.3).unwrap();
        b.iter(|| {
            env.gate_on();
            for _ in 0..1024 {
                black_box(env.next_value(44100.0));
            }
        });
    });
}

fn fm_synthesis_1024(c: &mut Criterion) {
    c.bench_function("fm_synthesis_1024", |b| {
        let mut fm = FmSynth::new(440.0, 220.0, 2.0, 44100.0).unwrap();
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            fm.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn wavetable_1024(c: &mut Criterion) {
    c.bench_function("wavetable_1024", |b| {
        let table =
            Wavetable::from_harmonics(8, &[1.0, 0.5, 0.33, 0.25, 0.2, 0.167, 0.143, 0.125], 2048)
                .unwrap();
        let mut osc = WavetableOscillator::new(table, 440.0, 44100.0).unwrap();
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            osc.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

criterion_group!(
    benches,
    oscillator_sine_1024,
    oscillator_saw_polyblep_1024,
    biquad_filter_1024,
    adsr_envelope_1024,
    fm_synthesis_1024,
    wavetable_1024,
);
criterion_main!(benches);
