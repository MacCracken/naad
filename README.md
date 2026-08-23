# naad

**naad** (नाद — *primordial sound/vibration*) — audio synthesis primitives for
the AGNOS ecosystem.

A Cyrius library. naad owns the DSP building blocks: oscillators, filters,
envelopes, wavetables, modulation, delay and effects, dynamics, EQ, reverb,
noise, panning, tuning and spectral analysis — plus higher-level synthesis
algorithms (subtractive, FM, additive, formant, granular, physical modelling,
vocoder, drum) and an acoustics layer that wraps [goonj](https://github.com/MacCracken/goonj).
Math comes from [hisab](https://github.com/MacCracken/hisab); the consumers are
dhvani (sound engine) and svara (music composition).

Current version is in [`VERSION`](VERSION); the toolchain pin and the resolved
dependency tags are in [`cyrius.cyml`](cyrius.cyml) and [`cyrius.lock`](cyrius.lock).

This is a port. The original 13,465-line Rust library is frozen at `rust-old/`
and is the parity oracle — never a build target. All 41 of its modules are
ported.

## Features

**Core DSP**

- **Oscillators** — sine, saw, square, triangle, pulse, plus noise-backed
  waveforms (white/pink/brown); PolyBLEP anti-aliasing on the discontinuous
  shapes. Unison (detune + stereo spread), sub-oscillator, hard sync.
- **Filters** — biquad from the Audio EQ Cookbook (lowpass, highpass, bandpass,
  notch, allpass, low shelf, high shelf, peak) and a state variable filter.
- **Envelopes** — ADSR, arbitrary multi-stage segment envelopes, and
  Catmull-Rom point envelopes.
- **Wavetables** — build from raw samples or from a harmonic series,
  interpolated read, and morphing across a table set.
- **Modulation** — LFO, FM, ring modulation, and a modulation matrix routing
  sources onto pitch / cutoff / resonance / amplitude / pan / pulse width /
  FM index / LFO rate.
- **Delay** — fractional delay lines, feedback comb filters, allpass delays.
- **Effects** — chorus, flanger, phaser, distortion (tanh soft clip, hard clip,
  wave fold).
- **Dynamics** — level detector, compressor (hard and soft knee), limiter,
  noise gate with hold.
- **EQ** — parametric (arbitrary biquad bands), graphic, de-esser.
- **Reverb** — damped-comb + allpass network with pre-delay and wet/dry mix.
- **Noise** — white (xorshift32), pink (Voss-McCartney, 16 octaves), brown
  (leaky integrated white).
- **Panning** — equal-power and linear pan laws, stereo balance.
- **Smoothing** — one-pole parameter smoothing with settle detection.
- **Voices** — polyphonic voice manager; poly/mono/legato modes, voice stealing
  by oldest / quietest / lowest / none.
- **Tuning** — equal temperament, just intonation, Pythagorean, custom ratio
  tables, MIDI↔Hz, cent differences.
- **Spectral** — FFT magnitudes, power spectrum, STFT, chromagram, onset
  detection, autocorrelation pitch detection, B-spline evaluation and
  least-squares polynomial fit. FFT and B-spline come from hisab; the
  least-squares solver (`ganita_mat_*`) is the stdlib `ganita` leaf, not hisab.

**Synthesis**

- **Subtractive** — two oscillators into a filter with amplitude and filter
  envelopes.
- **FM** — operator-based frequency modulation.
- **Additive** — partial bank with DCT amplitude compression/restore.
- **Formant** — vowel formant banks with morphing between vowels.
- **Granular** — grain scheduler with Hann/Gaussian/Tukey/rectangular windows,
  pitch shift, position, spray.
- **Physical** — Karplus-Strong plucked string, digital waveguide.
- **Vocoder** — multi-band modulator/carrier vocoder.
- **Drum** — kick, snare, hi-hat (open and closed).

**Acoustics** (thin wrappers over goonj)

- Shoebox room simulation reverb, convolution reverb (time-domain per sample,
  FFT per block), FDN reverb over goonj's room-derived network plus a
  naad-owned 8×8 Sylvester-Hadamard matrix FDN.
- Binaural HRTF spatialization, first-order Ambisonics B-format encoding,
  source directivity patterns.
- ISO 3382-1 analysis (C50, C80, D50, STI, Sabine RT60) and absorption-placement
  advice; coupled-room double-slope decay.

## Usage

Consumers include the single bundled fold `dist/naad.cyr`. The example below is
compiled and run as written:

```cyrius
include "lib/hisab.cyr"
include "lib/goonj.cyr"
include "dist/naad.cyr"

fn main() {
    alloc_init();

    var sr = f64_from(44100);

    # 440 Hz saw, PolyBLEP anti-aliased.
    var osc = osc_new(WAVEFORM_SAW, f64_from(440), sr);
    if (naad_is_err(osc) == 1) { return 1; }

    # ADSR: 10 ms attack, 100 ms decay, 0.7 sustain, 300 ms release.
    var env = envelope_adsr_new(f64_div(f64_from(1), f64_from(100)),
                                f64_div(f64_from(1), f64_from(10)),
                                f64_div(f64_from(7), f64_from(10)),
                                f64_div(f64_from(3), f64_from(10)));
    if (naad_is_err(env) == 1) { return 1; }

    # 2 kHz lowpass, Q = 0.707.
    var lp = filter_biquad_new(NAAD_FILTER_LOWPASS, sr, f64_from(2000),
                               f64_div(f64_from(707), f64_from(1000)));
    if (naad_is_err(lp) == 1) { return 1; }

    envelope_adsr_gate_on(env);
    var i = 0;
    while (i < 128) {
        var s = f64_mul(osc_next_sample(osc), envelope_adsr_next_value(env));
        var out = filter_biquad_process_sample(lp, s);
        if (naad_is_finite(out) == 0) { return 1; }
        i = i + 1;
    }
    println("ok");
    return 0;
}

var r = main();
syscall(60, r);
```

Public constants and the more generic helper names carry a `NAAD_`/`naad_`
prefix — `NAAD_ERR_*`, `NAAD_FILTER_*`, `NAAD_VOICE_*`, `naad_lerp`, `naad_rms`
— because Cyrius has one flat namespace once distlib bundles are concatenated.
Domain-specific families (`WAVEFORM_*`, `NOISE_*`, `MODULATION_LFO_*`, …) are
already namespaced by their own compound prefix and are left bare.

Constructors return either a struct pointer or a negative `NAAD_ERR_*` code — check
with `naad_is_err` before use. `naad_err_name` maps a code to a string.
Non-integer f64 arguments are built with `f64_from` / `f64_div` (or as IEEE-754
bit patterns, the form the test suites use).

`naadex.cyr` at the repo root is a second worked example: a 440 Hz sine
oscillator, built as a bare-syscall ring-3 program for agnos (it signals
success through its exit status, because `syscall(60, …)` is a no-op there).

## Consumption

Depend on naad's bundle, not on `src/`:

```toml
[deps.naad]
git = "https://github.com/MacCracken/naad.git"
tag = "<release tag>"
modules = ["dist/naad.cyr"]
```

`dist/naad.cyr` is the 39 library modules concatenated in dependency order,
with no `include` lines of its own. It **externalizes** its dependencies — hisab
and goonj are not inlined — so a consumer must pin both in its own manifest at
the tags naad pins in [`cyrius.cyml`](cyrius.cyml) (sakshi arrives transitively
via hisab). The stdlib leaves the fold needs are listed in
[`dist/naad.deps`](dist/naad.deps), which `cyrius deps` reads.

**Include order matters.** The topological order across the stack is:

```
sakshi → hisab → goonj → naad
```

Order still matters for type references (`dist/naad.cyr` does
`alloc(sizeof(CoupledRooms))` on a goonj struct, so goonj must precede naad),
but it is no longer load-bearing for *correctness*. Until 2.1.3 both libraries
defined a bare top-level `ERR_INVALID_FREQUENCY` in Cyrius's flat namespace
(naad `-1`, goonj `-3`) and duplicate top-level `var`s draw no diagnostic —
unlike duplicate `fn`s — so which value you got depended on include order.
naad's error constants now carry the `NAAD_ERR_` prefix and the two libraries'
symbol sets are disjoint. `tests/bundle.tcyr` asserts both codes are visible at
once with their own distinct values.

naad's own builds include only hisab, goonj and naad (see `src/main.cyr:9-11`
and `tests/bundle.tcyr`): naad calls nothing in sakshi, so goonj's sakshi
references resolve as unreachable-undefined and are pruned. A consumer that also
uses goonj's logging includes sakshi first. dhvani vendors the whole chain into
its own `lib/` and includes it in exactly this order.

## Module layout

39 modules in `src/`, bundled into `dist/naad.cyr` in dependency order. The
`[lib].modules` list in [`cyrius.cyml`](cyrius.cyml) is the source of truth.

| Layer | Modules |
|-------|---------|
| L0 base | `error`, `dsp_util`, `dsp_spectral` |
| L0 leaves | `panning`, `mod_matrix`, `tuning`, `voice`, `smoothing`, `delay`, `filter`, `noise`, `granular` |
| L1 | `wavetable`, `dynamics`, `reverb`, `eq`, `envelope`, `additive`, `physical`, `formant`, `fm`, `vocoder`, `drum` |
| L2 oscillators | `osc_core`, `osc_unison`, `osc_sub`, `osc_sync` |
| L3 routing | `modulation`, `effects`, `subtractive` |
| Acoustics | `acoustics` + `acoustics_room`, `acoustics_directivity`, `acoustics_ambisonics`, `acoustics_fdn`, `acoustics_binaural`, `acoustics_analysis`, `acoustics_convolution`, `acoustics_coupled` |

`src/` holds 40 files. The 40th, `src/main.cyr`, is the smoke binary — it
includes the bundle and generates 128 sine samples to prove `dist/naad.cyr`
compiles and links as an executable. It is deliberately **not** in
`[lib].modules` and is not bundled.

The Rust layout does not map one-to-one: `lib.rs` folded into `src/error.cyr`,
`oscillator/{core,unison,sub,sync}.rs` renamed to
`src/osc_{core,unison,sub,sync}.cyr`, and the `synth/`, `oscillator/` and
`acoustics/` subdirectories flattened into `src/` with descriptive names. Per-module parity is tracked in
[`docs/development/port-audit.md`](docs/development/port-audit.md).

## Build and test

```sh
cyrius deps                            # resolve deps into lib/
cyrius build src/main.cyr build/naad   # compile the smoke binary
cyrius test                            # run every tests/*.tcyr suite (what CI runs)
cyrius test tests/filter.tcyr          # run ONE suite
```

One `tests/<module>.tcyr` suite per module, ported one-for-one from that
module's Rust `#[test]` blocks (serde round-trips and `Display`-string tests
dropped — no serde, integer error codes). The four mutually referential `osc_*`
modules share `tests/oscillator.tcyr`; `tests/bundle.tcyr` is the cross-layer
smoke over `dist/naad.cyr`.

Quality gates:

```sh
cyrius fmt <file>.cyr --check   # NOTE: without --check, fmt REWRITES the file in place
cyrius lint <file>.cyr
cyrius doc --check <file>.cyr
cyrius vet src/main.cyr
cyrius deny src/main.cyr
cyrius bench <file>.bcyr
cyrius coverage
cyrius audit                    # fmt + lint + docs + tests + bench sweep
```

`cyrius deps`, `cyrius build` and `cyrius test` re-resolve dependencies and race
on `cyrius.lock` — concurrent runs corrupt it. When running toolchain commands in
parallel, serialize them behind a lock file: `flock <lock> cyrius …`.

## Consumers

- **dhvani** — AGNOS sound engine; vendors naad's bundle into its own `lib/` and
  includes it in the dependency order above.
- **svara** — music composition; pins `[deps.naad]` by git tag and uses the
  biquad, noise and LFO backends for its glottal and vocal-tract models.
- **jalwa** — media player; reaches naad through dhvani.

## Documentation

- [`docs/guides/getting-started.md`](docs/guides/getting-started.md)
- [`docs/development/state.md`](docs/development/state.md) — live state
- [`docs/development/port-audit.md`](docs/development/port-audit.md) — per-module port parity
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — milestones
- [`docs/adr/`](docs/adr/) — architecture decision records

## License

GPL-3.0-only
