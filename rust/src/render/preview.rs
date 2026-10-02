//! A model sheet for sculpting without the game (tests only): a `Collected` drawn from the
//! front, the side, the back and above, and from two three-quarter views, each voxel a few
//! pixels across, lit from the upper left and outlined where the depth jumps, on a grid of
//! 8-voxel squares with the floor line drawn. Saved as a PNG beside the others in
//! `VH_PREVIEW_DIR` (or the temp folder's `vh-preview`).
//!
//! The sheet runs: front, right side (the model's own left), back; top (front down), the
//! front-left three-quarter, the back-right three-quarter.

use super::voxels::Collected;
use glam::Vec3;

/// The largest a view is drawn, in pixels.
const VIEW: f32 = 340.0;
const BG: [u8; 3] = [58, 62, 70];
const GRID: [u8; 3] = [70, 75, 84];
const FLOOR: [u8; 3] = [120, 92, 60];

struct Grid {
    w: i32,
    h: i32,
    d: i32,
    cells: Vec<u32>,
}

impl Grid {
    fn of(c: &Collected) -> Self {
        let mut cells = vec![0; (c.width * c.height * c.depth) as usize];
        for v in &c.voxels {
            if v.x >= 0 && v.y >= 0 && v.z >= 0 && v.x < c.width && v.y < c.height && v.z < c.depth {
                // (The colour with its material byte, and the top bit so that black is not empty.)
                cells[((v.y * c.depth + v.z) * c.width + v.x) as usize] = v.c | 1 << 31;
            }
        }
        Self { w: c.width, h: c.height, d: c.depth, cells }
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u32 {
        if x < 0 || y < 0 || z < 0 || x >= self.w || y >= self.h || z >= self.d {
            return 0;
        }
        self.cells[((y * self.d + z) * self.w + x) as usize]
    }

    /// The first voxel a ray from `o` along `dir` meets: its colour, the face it came through, and how far.
    fn cast(&self, o: Vec3, dir: Vec3) -> Option<(u32, Vec3, f32)> {
        // Into the grid's box first.
        let (lo, hi) = (Vec3::splat(-0.5), Vec3::new(self.w as f32, self.h as f32, self.d as f32) - 0.5);
        let inv = Vec3::ONE / dir;
        let (t0, t1) = ((lo - o) * inv, (hi - o) * inv);
        let (near, far) = (t0.min(t1).max_element(), t0.max(t1).min_element());
        if far < near.max(0.0) {
            return None;
        }
        let mut t = near.max(0.0) + 1e-4;
        let p = o + dir * t;
        let mut cell = (p + 0.5).floor();
        let step = dir.signum();
        let next = |c: f32, s: f32| c - 0.5 + if s > 0.0 { 1.0 } else { 0.0 };
        let mut tmax = Vec3::new(
            if dir.x != 0.0 { (next(cell.x, step.x) - o.x) * inv.x } else { f32::INFINITY },
            if dir.y != 0.0 { (next(cell.y, step.y) - o.y) * inv.y } else { f32::INFINITY },
            if dir.z != 0.0 { (next(cell.z, step.z) - o.z) * inv.z } else { f32::INFINITY },
        );
        let delta = inv.abs();
        let mut face = Vec3::ZERO;
        for _ in 0..(self.w + self.h + self.d) * 2 {
            let c = self.get(cell.x as i32, cell.y as i32, cell.z as i32);
            if c != 0 {
                return Some((c, face, t));
            }
            if tmax.x < tmax.y && tmax.x < tmax.z {
                t = tmax.x;
                cell.x += step.x;
                tmax.x += delta.x;
                face = Vec3::new(-step.x, 0.0, 0.0);
            } else if tmax.y < tmax.z {
                t = tmax.y;
                cell.y += step.y;
                tmax.y += delta.y;
                face = Vec3::new(0.0, -step.y, 0.0);
            } else {
                t = tmax.z;
                cell.z += step.z;
                tmax.z += delta.z;
                face = Vec3::new(0.0, 0.0, -step.z);
            }
            if t > far {
                return None;
            }
        }
        None
    }
}

/// One view: looking along `dir` with `up` up, every pixel its voxel's shaded colour.
fn view(g: &Grid, dir: Vec3, up: Vec3, scale: f32) -> (usize, usize, Vec<[u8; 3]>) {
    let dir = dir.normalize();
    let right = dir.cross(up).normalize();
    let up = right.cross(dir);
    let centre = Vec3::new(g.w as f32 - 1.0, g.h as f32 - 1.0, g.d as f32 - 1.0) / 2.0;
    let half = Vec3::new(g.w as f32, g.h as f32, g.d as f32) / 2.0;
    let (mut u0, mut u1, mut v0, mut v1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for k in 0..8 {
        let corner = centre + half * Vec3::new([-1.0, 1.0][k & 1], [-1.0, 1.0][k >> 1 & 1], [-1.0, 1.0][k >> 2 & 1]);
        let (u, v) = ((corner - centre).dot(right), (corner - centre).dot(up));
        (u0, u1, v0, v1) = (u0.min(u), u1.max(u), v0.min(v), v1.max(v));
    }
    let (pw, ph) = (((u1 - u0) * scale).ceil() as usize, ((v1 - v0) * scale).ceil() as usize);
    let back = centre - dir * (half.length() + 2.0);
    let light = Vec3::new(-0.45, 0.8, 0.55).normalize();
    // (The grid only behind the views straight along an axis.)
    let square = dir.abs().max_element() > 0.99;
    let mut depth = vec![f32::INFINITY; pw * ph];
    let mut px = vec![BG; pw * ph];
    for j in 0..ph {
        for i in 0..pw {
            let (u, v) = (u0 + (i as f32 + 0.5) / scale, v1 - (j as f32 + 0.5) / scale);
            let o = back + right * u + up * v;
            // The 8-voxel grid on the background, and the floor's line, in model units.
            let w = o + dir * (half.length() + 2.0);
            let on_grid = |a: f32| (a + 0.5).rem_euclid(8.0) < 1.0 / scale;
            if dir.y.abs() < 0.5 && (w.y + 0.5).abs() < 1.0 / scale {
                px[j * pw + i] = FLOOR;
            } else if square
                && (on_grid(w.x) && dir.x.abs() < 0.5 || on_grid(w.y) && dir.y.abs() < 0.5 || on_grid(w.z) && dir.z.abs() < 0.5)
            {
                px[j * pw + i] = GRID;
            }
            let Some((c, face, t)) = g.cast(o, dir) else { continue };
            let c = c & 0x7fff_ffff;
            let rgb = Vec3::new(((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32) / 255.0;
            let glow = c >> 24 == 11;
            let lit = if glow { 1.15 } else { 0.42 + 0.58 * face.dot(light).max(0.0) + 0.12 * face.y.max(0.0) };
            let out = (rgb * lit).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
            px[j * pw + i] = [out.x as u8, out.y as u8, out.z as u8];
            depth[j * pw + i] = t;
        }
    }
    // Outlines where the depth jumps, so the silhouette and the overlaps read.
    let src = px.clone();
    for j in 0..ph {
        for i in 0..pw {
            let t = depth[j * pw + i];
            if !t.is_finite() {
                continue;
            }
            let edge = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|&(di, dj)| {
                let (a, b) = (i as i32 + di, j as i32 + dj);
                a < 0 || b < 0 || a >= pw as i32 || b >= ph as i32 || depth[b as usize * pw + a as usize] > t + 2.5
            });
            if edge {
                let s = src[j * pw + i];
                px[j * pw + i] = [s[0] / 3, s[1] / 3, s[2] / 3];
            }
        }
    }
    (pw, ph, px)
}

/// Draws `c`'s sheet and saves it as `<name>.png`; returns the path.
pub fn sheet(c: &Collected, name: &str) -> String {
    let g = Grid::of(c);
    let biggest = g.w.max(g.h).max(g.d) as f32;
    let scale = (VIEW / biggest).clamp(1.0, 8.0);
    let iso = |yaw: f32| Vec3::new(yaw.sin() * 0.82, -0.52, -yaw.cos() * 0.82);
    let views = [
        view(&g, Vec3::NEG_Z, Vec3::Y, scale),
        view(&g, Vec3::NEG_X, Vec3::Y, scale),
        view(&g, Vec3::Z, Vec3::Y, scale),
        view(&g, Vec3::NEG_Y, Vec3::NEG_Z, scale),
        view(&g, iso(0.6), Vec3::Y, scale),
        view(&g, iso(0.6 + std::f32::consts::PI), Vec3::Y, scale),
    ];
    let gap = 8;
    let row_h = |r: usize| views[r * 3..r * 3 + 3].iter().map(|v| v.1).max().unwrap_or(0);
    let row_w = |r: usize| views[r * 3..r * 3 + 3].iter().map(|v| v.0 + gap).sum::<usize>() + gap;
    let (w, h) = (row_w(0).max(row_w(1)), row_h(0) + row_h(1) + gap * 3);
    let mut img = vec![[24u8, 26, 30]; w * h];
    let mut y = gap;
    for r in 0..2 {
        let mut x = gap;
        for (vw, vh, px) in &views[r * 3..r * 3 + 3] {
            for j in 0..*vh {
                for i in 0..*vw {
                    img[(y + j) * w + x + i] = px[j * vw + i];
                }
            }
            x += vw + gap;
        }
        y += row_h(r) + gap;
    }
    let dir = std::env::var("VH_PREVIEW_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("vh-preview"));
    std::fs::create_dir_all(&dir).expect("the preview folder");
    let path = dir.join(format!("{name}.png"));
    std::fs::write(&path, png(w as u32, h as u32, &img)).expect("the preview");
    path.display().to_string()
}

/// A PNG of `rgb` rows, stored without compression (a preview is small and short-lived).
fn png(w: u32, h: u32, rgb: &[[u8; 3]]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w as usize * 3 + 1) * h as usize);
    for row in rgb.chunks(w as usize) {
        raw.push(0);
        raw.extend(row.iter().flatten());
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (k, b) in blocks.iter().enumerate() {
        z.push(u8::from(k + 1 == blocks.len()));
        let n = b.len() as u16;
        z.extend(n.to_le_bytes());
        z.extend((!n).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend((b << 16 | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    for (kind, data) in [(b"IHDR", ihdr), (b"IDAT", z), (b"IEND", Vec::new())] {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(&data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}
