//! Criterion benchmarks for naad synthesis primitives.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use naad::delay::{AllpassDelay, CombFilter};
use naad::effects::{Chorus, Distortion, DistortionType, Phaser};
use naad::envelope::Adsr;
use naad::filter::{BiquadFilter, FilterType, StateVariableFilter};
use naad::modulation::FmSynth;
use naad::noise::{NoiseGenerator, NoiseType};
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

fn svf_filter_1024(c: &mut Criterion) {
    c.bench_function("svf_filter_1024", |b| {
        let mut svf = StateVariableFilter::new(1000.0, 0.707, 44100.0).unwrap();
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(svf.process_sample(s));
            }
        });
    });
}

fn adsr_envelope_1024(c: &mut Criterion) {
    c.bench_function("adsr_envelope_1024", |b| {
        let mut env = Adsr::new(0.01, 0.1, 0.7, 0.3).unwrap();
        b.iter(|| {
            env.gate_on();
            for _ in 0..1024 {
                black_box(env.next_value());
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

fn noise_white_1024(c: &mut Criterion) {
    c.bench_function("noise_white_1024", |b| {
        let mut ng = NoiseGenerator::new(NoiseType::White, 42);
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            ng.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn noise_pink_1024(c: &mut Criterion) {
    c.bench_function("noise_pink_1024", |b| {
        let mut ng = NoiseGenerator::new(NoiseType::Pink, 42);
        let mut buffer = [0.0f32; 1024];
        b.iter(|| {
            ng.fill_buffer(black_box(&mut buffer));
            black_box(&buffer);
        });
    });
}

fn comb_filter_1024(c: &mut Criterion) {
    c.bench_function("comb_filter_1024", |b| {
        let mut comb = CombFilter::new(441, 0.7);
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(comb.process_sample(s));
            }
        });
    });
}

fn allpass_delay_1024(c: &mut Criterion) {
    c.bench_function("allpass_delay_1024", |b| {
        let mut ap = AllpassDelay::new(441, 0.7);
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(ap.process_sample(s));
            }
        });
    });
}

fn chorus_1024(c: &mut Criterion) {
    c.bench_function("chorus_1024", |b| {
        let mut chorus = Chorus::new(3, 0.5, 10.0, 2.0, 0.5, 44100.0).unwrap();
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(chorus.process_sample(s));
            }
        });
    });
}

fn phaser_1024(c: &mut Criterion) {
    c.bench_function("phaser_1024", |b| {
        let mut phaser = Phaser::new(6, 0.5, 200.0, 2000.0, 0.7, 0.5, 44100.0).unwrap();
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(phaser.process_sample(s));
            }
        });
    });
}

fn distortion_wavefold_1024(c: &mut Criterion) {
    c.bench_function("distortion_wavefold_1024", |b| {
        let dist = Distortion::new(DistortionType::WaveFold, 5.0, 1.0);
        let buffer = [0.5f32; 1024];
        b.iter(|| {
            for &s in black_box(&buffer) {
                black_box(dist.process_sample(s));
            }
        });
    });
}

criterion_group!(
    benches,
    oscillator_sine_1024,
    oscillator_saw_polyblep_1024,
    biquad_filter_1024,
    svf_filter_1024,
    adsr_envelope_1024,
    fm_synthesis_1024,
    wavetable_1024,
    noise_white_1024,
    noise_pink_1024,
    comb_filter_1024,
    allpass_delay_1024,
    chorus_1024,
    phaser_1024,
    distortion_wavefold_1024,
);
criterion_main!(benches);
