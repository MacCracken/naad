use std::hint::black_box;
#[derive(Clone, Copy)]
struct B<T> { b0: T, b1: T, b2: T, a1: T, a2: T, z1: T, z2: T }
macro_rules! imp { ($t:ty, $step:ident, $fl:ident, $min:expr, $loop_:ident, $bank:ident, $abs:expr) => {
    #[inline(always)]
    fn $fl(x: $t) -> $t { if x.abs() < $min { 0.0 } else { x } }
    #[inline(never)]
    fn $step(s: &mut B<$t>, x: $t) -> $t {
        let y = s.b0 * x + s.z1;
        let n1 = s.b1 * x - s.a1 * y + s.z2;
        let n2 = s.b2 * x - s.a2 * y;
        s.z1 = $fl(n1); s.z2 = $fl(n2);
        y
    }
    #[inline(never)]
    fn $loop_(input: &[$t], n: usize) -> $t {
        let (b0, b1, b2, a1, a2): ($t, $t, $t, $t, $t) = (0.125, 0.25, 0.125, -0.75, 0.25);
        let (mut z1, mut z2, mut h): ($t, $t, $t) = (0.0, 0.0, 0.0);
        for i in 0..n {
            let x = input[i & 1023];
            let y = b0 * x + z1;
            let n1 = b1 * x - a1 * y + z2;
            z2 = b2 * x - a2 * y;
            z1 = n1;
            h = y;
        }
        h
    }
    #[inline(never)]
    fn $bank(b0: &[$t], b1: &[$t], b2: &[$t], a1: &[$t], a2: &[$t], z1: &mut [$t], z2: &mut [$t], y: &mut [$t], x: $t) {
        for i in 0..b0.len() {
            let v = b0[i] * x + z1[i];
            let n1 = b1[i] * x - a1[i] * v + z2[i];
            let n2 = b2[i] * x - a2[i] * v;
            z1[i] = n1; z2[i] = n2; y[i] = v;
        }
    }
} }
imp!(f32, step32, fl32, f32::MIN_POSITIVE, loop32, bank32, 0);
imp!(f64, step64, fl64, f32::MIN_POSITIVE as f64, loop64, bank64, 0);
fn main() {
    let a: Vec<usize> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let (v, n) = (a[0], a[1]);
    let in32: Vec<f32> = (0..1024).map(|i| (((i % 17) as f32) - 8.0) / 16.0).collect();
    let in64: Vec<f64> = in32.iter().map(|&x| x as f64).collect();
    let nb = 64;
    match v {
        0 => { let mut s = B::<f32> { b0: 0.125, b1: 0.25, b2: 0.125, a1: -0.75, a2: 0.25, z1: 0.0, z2: 0.0 };
            let mut h = 0.0; for i in 0..n { h = step32(&mut s, black_box(in32[i & 1023])); } println!("{}", (h as f32).to_bits()); }
        1 => { let mut s = B::<f64> { b0: 0.125, b1: 0.25, b2: 0.125, a1: -0.75, a2: 0.25, z1: 0.0, z2: 0.0 };
            let mut h = 0.0; for i in 0..n { h = step64(&mut s, black_box(in64[i & 1023])); } println!("{}", (h as f64).to_bits()); }
        2 => println!("{}", loop32(black_box(&in32), n).to_bits()),
        3 => println!("{}", loop64(black_box(&in64), n).to_bits()),
        4 => { let (b0, b1, b2, a1, a2) = (vec![0.125f32; nb], vec![0.25f32; nb], vec![0.125f32; nb], vec![-0.75f32; nb], vec![0.25f32; nb]);
            let (mut z1, mut z2, mut y) = (vec![0f32; nb], vec![0f32; nb], vec![0f32; nb]);
            for i in 0..n { bank32(&b0, &b1, &b2, &a1, &a2, &mut z1, &mut z2, &mut y, black_box(in32[i & 1023])); } println!("{}", y[0].to_bits()); }
        5 => { let (b0, b1, b2, a1, a2) = (vec![0.125f64; nb], vec![0.25f64; nb], vec![0.125f64; nb], vec![-0.75f64; nb], vec![0.25f64; nb]);
            let (mut z1, mut z2, mut y) = (vec![0f64; nb], vec![0f64; nb], vec![0f64; nb]);
            for i in 0..n { bank64(&b0, &b1, &b2, &a1, &a2, &mut z1, &mut z2, &mut y, black_box(in64[i & 1023])); } println!("{}", y[0].to_bits()); }
        _ => {}
    }
}
