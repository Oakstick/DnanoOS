//! The split-sum BRDF lookup (RP14 in the Technical Art and Rendering Plan): for each angle of
//! view (n.v across) and roughness (down), the scale and bias a surface's reflectance head-on
//! takes for the light it reflects from all around (`f0 * a + b`), as Karis's split-sum
//! approximation integrates it with GGX. Worked out once at start-up and kept as a small texture.

use glam::{Vec2, Vec3};

pub const LUT_SIZE: u32 = 64;
const SAMPLES: u32 = 256;

fn hammersley(i: u32, n: u32) -> Vec2 {
    Vec2::new(i as f32 / n as f32, i.reverse_bits() as f32 * 2.328_306_4e-10)
}

/// The scale and bias for view angle cosine `nv` and roughness `rough`.
pub fn integrate(nv: f32, rough: f32) -> (f32, f32) {
    let nv = nv.max(1e-3);
    let v = Vec3::new((1.0 - nv * nv).sqrt(), 0.0, nv);
    let a = (rough * rough).max(1e-4);
    // Smith-Schlick visibility with k for image lighting.
    let k = a / 2.0;
    let g1 = |x: f32| x / (x * (1.0 - k) + k);
    let (mut sa, mut sb) = (0.0, 0.0);
    for i in 0..SAMPLES {
        let xi = hammersley(i, SAMPLES);
        let phi = std::f32::consts::TAU * xi.x;
        let cos_t = ((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y)).sqrt();
        let sin_t = (1.0 - cos_t * cos_t).sqrt();
        let h = Vec3::new(phi.cos() * sin_t, phi.sin() * sin_t, cos_t);
        let l = 2.0 * v.dot(h) * h - v;
        let (nl, nh, vh) = (l.z.max(0.0), h.z.max(0.0), v.dot(h).max(0.0));
        if nl > 0.0 {
            let vis = g1(nl) * g1(nv) * vh / (nh * nv).max(1e-6);
            let fc = (1.0 - vh).powi(5);
            sa += (1.0 - fc) * vis;
            sb += fc * vis;
        }
    }
    (sa / SAMPLES as f32, sb / SAMPLES as f32)
}

/// An f32 as an IEEE half float (rounded to nearest), for texture data.
pub fn f16(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp >= 31 {
        return sign | 0x7c00;
    }
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (man | 0x80_0000) >> (1 - exp);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let h = ((exp as u32) << 10) | (man >> 13);
    sign | (h + ((man >> 12) & 1)) as u16
}

/// The lookup as RGBA half floats (scale, bias, 0, 1), `LUT_SIZE` texels a side: n.v along x
/// (texel centres), roughness down y.
pub fn lut_texels() -> Vec<u16> {
    let n = LUT_SIZE;
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (a, b) = integrate((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32);
            out.extend_from_slice(&[f16(a), f16(b), 0, f16(1.0)]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lookup_reflects_nearly_all_head_on_when_smooth_and_less_when_rough() {
        let (a, b) = integrate(1.0, 0.05);
        assert!(a > 0.95 && b < 0.02, "{a} {b}");
        let (a_rough, _) = integrate(1.0, 1.0);
        assert!(a_rough < a);
        // At a grazing angle the bias (the Fresnel rise) grows.
        let (_, b_grazing) = integrate(0.05, 0.2);
        assert!(b_grazing > b + 0.1, "{b_grazing}");
        // Never more than all of it.
        for (nv, r) in [(0.1, 0.1), (0.5, 0.5), (0.9, 0.9)] {
            let (a, b) = integrate(nv, r);
            assert!(a + b <= 1.01 && a >= 0.0 && b >= 0.0, "{nv} {r}: {a} {b}");
        }
    }

    #[test]
    fn half_floats_hold_the_lookup() {
        let back = |h: u16| {
            let e = ((h >> 10) & 31) as i32;
            let m = (h & 1023) as f32;
            if e == 0 {
                m / 1024.0 * 2f32.powi(-14)
            } else {
                (1.0 + m / 1024.0) * 2f32.powi(e - 15)
            }
        };
        for v in [0.0, 1.0, 0.5, 0.25] {
            assert_eq!(back(f16(v)), v);
        }
        for v in [0.003, 0.04, 0.2, 0.73, 0.999] {
            assert!((back(f16(v)) - v).abs() <= v * 1e-3 + 1e-6, "{v}");
        }
    }
}
