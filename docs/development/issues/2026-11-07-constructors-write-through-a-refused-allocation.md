# `noise_new`, `filter_biquad_with_gain`, `filter_svf_new`, `modulation_lfo_new` and `naad_vec_zeros` write through a refused allocation

**Status:** ✅ **RESOLVED in 2.3.4** (see *Resolution* at the end)
**Placement:** 2.3.4.
**Discovered:** 2026-11-07, from garjan 2.6.3's audit (`docs/audit/2026-11-07-audit-3.md` §3), whose fault-injection sweep refused each allocation in turn across 9,238 cases. Each case was re-verified here.
**Severity:** Medium. The process dies with SIGSEGV inside the constructor, so no caller can handle the failure. Reachable without a hook, under an address-space limit.
**Affects:** naad 2.3.3 (tag `2.3.3`, `5176778`, also `main`), built with cyrius 6.7.18.

## Summary

naad's constructors return "the struct pointer on success, or a negative `NAAD_ERR_*` on failure" (`src/filter.cyr` header). None of them checks `alloc`, though. When an allocation is refused, each one stores through the null pointer and the process dies with SIGSEGV before the constructor returns, so the caller never gets a chance to handle it. `naad_vec_zeros` does the same with `vec_new()`'s result and with its own `alloc(n * 8)`.

This happens without any test hook. The stdlib's bump `alloc` returns 0 when it cannot map a new chunk (cyrius 6.7.18 `lib/alloc.cyr:324-326`), for example under `ulimit -v`. Repro 2 below crashes `noise_new` that way.

garjan 2.6.3's fault-injection sweep found this. The sweep refuses each allocation in turn, one process per case, 9,238 cases in all. garjan already checks every naad constructor result, treating 0 or a negative value as an allocation failure, so a fix here needs no change in garjan.


## Where

The quotes come from the 2.3.3 bundle as vendored in garjan (`lib/naad.cyr`, byte-identical to `dist/naad.cyr` at the tag). The source file and line are in parentheses.

**`naad_vec_zeros`**: `lib/naad.cyr:121-127` (`src/error.cyr:114`)

```cyrius
fn naad_vec_zeros(n) {
    var v = vec_new();
    if (n > vec_cap(v)) {
        store64(v, alloc(n * 8));
        store64(v + 16, n);
    }
    var data = load64(v);
```

`vec_new()` returns 0 when refused, and `vec_cap(0)` then reads address 16. A refused `alloc(n * 8)` is stored as the data pointer, and the zero-fill loop writes to address 0. The function's own comment already describes this: *"A size alloc refuses stops at the null page on the first store, as a refused vec_new does."*

**`noise_new`**: `lib/naad.cyr:1271-1297` (`src/noise.cyr:47`)

```cyrius
    var rng = alloc(8);                                       # :1272
    ...
    store64(rng, state);                                      # :1275
    var pink_octaves = naad_vec_zeros(NOISE_PINK_OCTAVES);    # :1277
    ...
    var g = alloc(sizeof(NoiseGenerator));                    # :1290
    NoiseGenerator_set_noise_type(g, noise_type);
```

`noise_new` has no failure return at all today.

**`filter_biquad_with_gain`**: `lib/naad.cyr:2894-2895` (`src/filter.cyr:62`). This is also `filter_biquad_new`'s path.

```cyrius
    var self = alloc(sizeof(BiquadFilter));
    BiquadFilter_set_b0(self, F64_ONE);
```

**`filter_svf_new`**: `lib/naad.cyr:3152-3153` (`src/filter.cyr:326`)

```cyrius
    var self = alloc(sizeof(StateVariableFilter));
    StateVariableFilter_set_frequency(self, frequency);
```

**`modulation_lfo_new`**: `lib/naad.cyr:9500-9505` (`src/modulation.cyr:73`)

```cyrius
    var rng = alloc(8);
    store64(rng, MODULATION_SH_SEED);
    var initial_sh = xorshift32_signed_f32(rng);

    var self = alloc(sizeof(Lfo));
    Lfo_set_shape(self, shape);
```

## Reproduction

Both programs are also in this folder: [`repros/2026-11-07-constructors-write-through-a-refused-allocation.cyr`](repros/2026-11-07-constructors-write-through-a-refused-allocation.cyr) is `repro.cyr` and [`repros/2026-11-07-constructors-write-through-a-refused-allocation-real-heap.cyr`](repros/2026-11-07-constructors-write-through-a-refused-allocation-real-heap.cyr) is `real.cyr`. Copy them into a scratch project next to the `cyrius.cyml` shown below.

### 1. Refuse the Nth allocation (deterministic)

The compiler keeps the last definition of a function, and says so: `warning: duplicate fn 'alloc' (last definition wins; first defined in lib/alloc.cyr)`. The repro therefore defines its own `alloc`. It refuses the Nth request made after `fi_arm(N)` and serves every other request from a private zeroed mmap, as the stdlib does. Every stdlib allocation path, `vec_new` included through `default_alloc()`'s `_bump_alloc`, ends in `alloc`, so this one override covers them all.

`cyrius.cyml`:

```toml
[package]
name = "naad_oom_repro"
version = "0.1.0"
language = "cyrius"
cyrius = "6.7.18"

[build]
entry = "repro.cyr"
output = "build/repro"

[deps]
stdlib = ["syscalls", "string", "alloc", "str", "fmt", "vec", "io", "args", "math", "ganita"]

[deps.naad]
git = "https://github.com/MacCracken/naad.git"
tag = "2.3.3"
modules = ["dist/naad.cyr"]

[deps.hisab]
git = "https://github.com/MacCracken/hisab.git"
tag = "3.4.1"
modules = ["dist/hisab.cyr"]

[deps.goonj]
git = "https://github.com/MacCracken/goonj.git"
tag = "2.3.1"
modules = ["dist/goonj.cyr"]

[deps.sakshi]
git = "https://github.com/MacCracken/sakshi.git"
tag = "2.5.10"
modules = ["dist/sakshi.cyr"]
```

`repro.cyr`:

```cyrius
# naad 2.3.3: constructors write through a refused allocation.
#
# Replaces the stdlib `alloc` (the compiler keeps the last definition) with a
# bump allocator that refuses the Nth allocation made after fi_arm(N). Every
# other request is served from a private zeroed mmap, as the stdlib does.
#
# usage: repro CASE N
#   1 noise_new(NOISE_WHITE, 1)
#   2 filter_biquad_with_gain(LOWPASS, 44100, 1000, 0.707, 0)
#   3 filter_svf_new(1000, 0.707, 44100)
#   4 modulation_lfo_new(SINE, 1, 44100)
#   5 naad_vec_zeros(100)
#   6 naad_vec_zeros(2^61 + 1)   (n * 8 wraps to 8; run with N = 0)
# Exits 0 and prints the return value if the call returns; a segfault is 139.

var FI_ARMED = 0;
var FI_COUNT = 0;
var FI_N = 0;
var FI_BASE = 0;
var FI_PTR = 0;
var FI_END = 0;

fn alloc(size): i64 {
    if (size <= 0) { return 0; }
    if (FI_ARMED == 1) {
        FI_COUNT = FI_COUNT + 1;
        if (FI_COUNT == FI_N) { return 0; }
    }
    if (FI_BASE == 0) {
        FI_BASE = syscall(9, 0, 0x4000000, 3, 0x22, 0 - 1, 0);   # 64 MiB, RW, private anon
        FI_PTR = FI_BASE;
        FI_END = FI_BASE + 0x4000000;
    }
    size = (size + 7) & (0 - 8);
    if (FI_PTR + size > FI_END) { return 0; }
    var p = FI_PTR;
    FI_PTR = p + size;
    return p;
}

fn fi_arm(n) { FI_COUNT = 0; FI_N = n; FI_ARMED = 1; return 0; }
fn fi_disarm() { FI_ARMED = 0; return FI_COUNT; }

fn main() {
    args_init();
    var which = atoi(argv(1));
    var n = atoi(argv(2));
    var sr = f64_from(44100);
    var hz = f64_from(1000);
    var q = f64_div(f64_from(707), f64_from(1000));
    var r = 0;
    fi_arm(n);
    if (which == 1) { r = noise_new(NOISE_WHITE, 1); }
    if (which == 2) { r = filter_biquad_with_gain(NAAD_FILTER_LOWPASS, sr, hz, q, 0); }
    if (which == 3) { r = filter_svf_new(hz, q, sr); }
    if (which == 4) { r = modulation_lfo_new(MODULATION_LFO_SINE, F64_ONE, sr); }
    if (which == 5) { r = naad_vec_zeros(100); }
    if (which == 6) { r = naad_vec_zeros(0x2000000000000001); }
    var made = fi_disarm();
    fmt_int(r);
    syscall(1, 1, " returned\n", 10);
    return 0;
}
```

```sh
cyrius deps
CYRIUS_SYMS=syms.txt cyrius build repro.cyr build/repro
for c in 1 2 3 4 5; do for n in 0 1 2 3 4 5; do ./build/repro $c $n; echo "case $c N=$n exit=$?"; done; done
```

Observed results follow. Exit 139 is SIGSEGV. N=0 refuses nothing and is the control. The faulting function was resolved against the `CYRIUS_SYMS` file under gdb.

| Case | Call | Refused allocation | Exit | Faults in |
|---|---|---|---:|---|
| 1 | `noise_new(NOISE_WHITE, 1)` | N=1, `rng` | 139 | `noise_new` |
| 1 | | N=2, N=3, `vec_new`'s header and data | 139 | `vec_cap` ← `naad_vec_zeros` |
| 1 | | N=4, the struct | 139 | `noise_new` |
| 2 | `filter_biquad_with_gain(LOWPASS, 44100, 1000, 0.707, 0)` | N=1 | 139 | `filter_biquad_with_gain` |
| 3 | `filter_svf_new(1000, 0.707, 44100)` | N=1 | 139 | `filter_svf_new` |
| 4 | `modulation_lfo_new(SINE, 1, 44100)` | N=1, `rng`; N=2, the struct | 139 | `modulation_lfo_new` |
| 5 | `naad_vec_zeros(100)` | N=1, N=2, `vec_new` | 139 | `vec_cap` ← `naad_vec_zeros` |
| 5 | | N=3, `alloc(800)` | 139 | `naad_vec_zeros` |

Every allocation each function makes faults when it is refused. Each case returns a valid pointer with N=0, and with any N past the function's last allocation.

### 2. A really exhausted heap (no override)

`real.cyr`, in the same project:

```cyrius
# No fault injection: the real stdlib alloc, run under `ulimit -v`.
# Exhaust the heap, then call a naad constructor.
fn main() {
    var p = 1;
    while (p != 0) { p = alloc(1048576); }
    p = 1;
    while (p != 0) { p = alloc(8); }
    syscall(1, 1, "heap exhausted; calling noise_new\n", 34);
    var g = noise_new(NOISE_WHITE, 1);
    fmt_int(g);
    syscall(1, 1, " returned\n", 10);
    return 0;
}
```

```text
$ cyrius build real.cyr build/real
$ (ulimit -v 1048576; ./build/real; echo "exit=$?")
heap exhausted; calling noise_new
exit=139
```

## Expected

Each constructor should return its documented failure value instead of faulting. naad has no allocation error code yet, so I suggest adding one next to the others in `src/error.cyr`:

```cyrius
var NAAD_ERR_ALLOCATION = (0 - 6);   # an allocation was refused
```

- **`noise_new`, `filter_biquad_with_gain` (and therefore `filter_biquad_new`), `filter_svf_new` and `modulation_lfo_new`** should return `NAAD_ERR_ALLOCATION` when any of their allocations is refused. Callers that already test `naad_is_err` or `<= 0` handle this unchanged.
- **`naad_vec_zeros`** should return 0, as `vec_new` does. Its 29 call sites in `src/` then need to check for 0.

The internal callers have to pass the failure on: `noise_new` has 3, `filter_biquad_new` 7, `filter_biquad_with_gain` 2, `filter_svf_new` 1 and `modulation_lfo_new` 1.

## Suggested fix

I checked the patch below by redefining these functions on top of the 2.3.3 bundle, using the same last-definition-wins mechanism, and rerunning repro 1. Every refusal in cases 1-4 now returns `-6`, `naad_vec_zeros` returns 0, and no case faults. The internal callers listed above are not patched or tested.

```cyrius
# ---- proposed fix (overrides the 2.3.3 definitions; last definition wins) ----

var NAAD_ERR_ALLOCATION = (0 - 6);   # an allocation was refused

#must_use
fn naad_vec_zeros(n) {
    if (n < 0) { return 0; }
    if (n > VEC_CAP_MAX) { return 0; }   # bounds n * 8 (2^28 entries, as vec.cyr caps)
    var v = vec_new();
    if (v == 0) { return 0; }
    if (n > vec_cap(v)) {
        var buf = alloc(n * 8);
        if (buf == 0) { return 0; }
        store64(v, buf);
        store64(v + 16, n);
    }
    var data = load64(v);
    var i = 0;
    while (i < n) {
        store64(data + i * 8, 0);
        i = i + 1;
    }
    store64(v + 8, n);
    return v;
}

#must_use
fn noise_new(noise_type, seed) {
    var rng = alloc(8);
    if (rng == 0) { return NAAD_ERR_ALLOCATION; }
    var state = seed;
    if (state == 0) { state = 1; }
    store64(rng, state);

    var pink_octaves = naad_vec_zeros(NOISE_PINK_OCTAVES);
    if (pink_octaves == 0) { return NAAD_ERR_ALLOCATION; }

    var pink_running_sum = 0;
    if (noise_type == NOISE_PINK) {
        var i = 0;
        while (i < NOISE_PINK_OCTAVES) {
            var val = xorshift32_signed_f32(rng);
            vec_set(pink_octaves, i, val);
            pink_running_sum = f64_add(pink_running_sum, val);
            i = i + 1;
        }
    }

    var g = alloc(sizeof(NoiseGenerator));
    if (g == 0) { return NAAD_ERR_ALLOCATION; }
    NoiseGenerator_set_noise_type(g, noise_type);
    NoiseGenerator_set_rng(g, rng);
    NoiseGenerator_set_pink_octaves(g, pink_octaves);
    NoiseGenerator_set_pink_counter(g, 0);
    NoiseGenerator_set_pink_running_sum(g, pink_running_sum);
    NoiseGenerator_set_brown_prev(g, 0);
    return g;
}

#must_use
fn filter_biquad_with_gain(filter_type, sample_rate, frequency, q, gain_db) {
    var e = validate_sample_rate(sample_rate);
    if (naad_is_err(e) == 1) { return e; }
    e = validate_frequency(frequency, sample_rate);
    if (naad_is_err(e) == 1) { return e; }
    if (f64_le(q, 0) == 1) { return NAAD_ERR_INVALID_PARAMETER; }
    if (naad_is_finite(q) == 0) { return NAAD_ERR_INVALID_PARAMETER; }
    if (filter_type < NAAD_FILTER_LOWPASS) { return NAAD_ERR_INVALID_PARAMETER; }
    if (filter_type > NAAD_FILTER_PEAK) { return NAAD_ERR_INVALID_PARAMETER; }

    var self = alloc(sizeof(BiquadFilter));
    if (self == 0) { return NAAD_ERR_ALLOCATION; }
    BiquadFilter_set_b0(self, F64_ONE);
    BiquadFilter_set_b1(self, 0);
    BiquadFilter_set_b2(self, 0);
    BiquadFilter_set_a1(self, 0);
    BiquadFilter_set_a2(self, 0);
    BiquadFilter_set_z1(self, 0);
    BiquadFilter_set_z2(self, 0);
    BiquadFilter_set_filter_type(self, filter_type);
    BiquadFilter_set_sample_rate(self, sample_rate);
    BiquadFilter_set_frequency(self, frequency);
    BiquadFilter_set_q(self, q);
    BiquadFilter_set_gain_db(self, gain_db);
    filter_biquad_compute_coefficients(self);
    return self;
}

#must_use
fn filter_svf_new(frequency, q, sample_rate) {
    var e = validate_sample_rate(sample_rate);
    if (naad_is_err(e) == 1) { return e; }
    e = validate_frequency(frequency, sample_rate);
    if (naad_is_err(e) == 1) { return e; }
    if (f64_le(q, 0) == 1) { return NAAD_ERR_INVALID_PARAMETER; }
    if (naad_is_finite(q) == 0) { return NAAD_ERR_INVALID_PARAMETER; }

    var self = alloc(sizeof(StateVariableFilter));
    if (self == 0) { return NAAD_ERR_ALLOCATION; }
    StateVariableFilter_set_frequency(self, frequency);
    StateVariableFilter_set_q(self, q);
    StateVariableFilter_set_sample_rate(self, sample_rate);
    StateVariableFilter_set_ic1eq(self, 0);
    StateVariableFilter_set_ic2eq(self, 0);
    _filter_svf_compute(self);
    return self;
}

#must_use
fn modulation_lfo_new(shape, frequency, sample_rate) {
    var e = validate_sample_rate(sample_rate);
    if (naad_is_err(e) == 1) { return e; }
    if (f64_lt(frequency, 0) == 1) { return NAAD_ERR_INVALID_PARAMETER; }
    if (naad_is_finite(frequency) == 0) { return NAAD_ERR_INVALID_PARAMETER; }
    if (shape < MODULATION_LFO_SINE) { return NAAD_ERR_INVALID_PARAMETER; }
    if (shape > MODULATION_LFO_SAMPLE_AND_HOLD) { return NAAD_ERR_INVALID_PARAMETER; }

    var rng = alloc(8);
    if (rng == 0) { return NAAD_ERR_ALLOCATION; }
    store64(rng, MODULATION_SH_SEED);
    var initial_sh = xorshift32_signed_f32(rng);

    var self = alloc(sizeof(Lfo));
    if (self == 0) { return NAAD_ERR_ALLOCATION; }
    Lfo_set_shape(self, shape);
    Lfo_set_frequency(self, frequency);
    Lfo_set_sample_rate(self, sample_rate);
    Lfo_set_phase(self, 0);
    Lfo_set_mode(self, MODULATION_LFO_BIPOLAR);
    Lfo_set_depth(self, F64_ONE);
    Lfo_set_sh_value(self, initial_sh);
    Lfo_set_rng(self, rng);
    return self;
}
```

## Also: `naad_vec_zeros` does not bound `n * 8`

I found this while confirming the above; garjan's sweep did not report it. `naad_vec_zeros(0x2000000000000001)` computes `n * 8 = 8`, allocates 8 bytes, and then zero-fills 2^61 slots. Case 6 of the repro (`./build/repro 6 0`) faults in `naad_vec_zeros` with nothing refused. On the real heap it overwrites live allocations first. garjan bounded the same arithmetic in its own helper in 2.6.3. The patch above refuses `n > VEC_CAP_MAX`, the cap `lib/vec.cyr` already applies.

## Wider pattern

A rough grep of the 2.3.3 bundle finds 98 of its 107 `= alloc(` results with no check on the next line. I have not audited them. The five functions above are the ones garjan reaches.

## Context

These are items U1 and U1b in garjan's 2.6.3 audit (`docs/audit/2026-11-07-audit-3.md` §3). On garjan's side, `garjan_noise_new` checks each `noise_new` result, and `tests/naad_failure.tcyr` proves the check at all 21 synth constructors with a failing `noise_new` stand-in.

## Resolution

Resolved in **2.3.4**, as suggested, with one change. Recorded in
[ADR-0010](../../adr/0010-five-constructors-report-a-refused-allocation.md), which narrows
ADR-0006.

- `NAAD_ERR_ALLOCATION = (0 - 6)` is in `src/error.cyr`, named `"allocation refused"`.
- `noise_new`, `filter_biquad_with_gain` (and so `filter_biquad_new`), `filter_svf_new` and
  `modulation_lfo_new` return it for every one of their allocations, singly refused or
  refused from then on. `naad_vec_zeros` returns 0, and refuses any `n` outside
  `[0, VEC_CAP_MAX]`, which closes the `n * 8` wrap reported above.
- Of the internal callers, only `osc_new` needed a change: it stored `noise_new`'s result
  as its generator, and now returns the code. The other eleven already passed on any
  negative result.
- **The change from the suggestion:** naad's 28 other callers of `naad_vec_zeros` do not
  check for 0. They go through `_naad_vec_zeros_or_stop`, which stops in place, as 2.3.3
  did. Passing the 0 on unchecked would have built objects that stop at their first
  sample instead of in the constructor. That was measured on `naad_fdn_matrix_new`.
- `tests/alloc_refused.tcyr` reruns this filing's repro inside the suite, with the same
  last-definition override of `alloc`, and every case returns. Each of the 14 new checks
  was reverted one at a time, and each revert fails it.

garjan needs no change, as the filing said. The other 98 or so unchecked sites stay
fail-stop under ADR-0006.
