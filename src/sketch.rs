//! Pencil on paper: a grey palette and sketchy strokes, hatching, scribbles and doodles.
//!
//! Every wobble comes from a seed, so a shape drawn with the same seed looks exactly the same
//! every frame ("still lines") and on every computer.

use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

/// A grey: the look is strictly black and white, so red = green = blue.
pub const fn grey(v: f32) -> Color {
    Color { r: v, g: v, b: v, a: 1.0 }
}

/// The paper itself; also cards, eye whites and the stickman's head.
pub const PAPER: Color = grey(0.95);
/// Outlines, the stickman and main text.
pub const INK: Color = grey(0.13);
/// Hatching on the floor, ceiling and bumps; monsters.
pub const GRAPHITE: Color = grey(0.32);
/// Platform hatching, secondary text, dust.
pub const SHADE: Color = grey(0.58);
/// Graph-paper grid and background flecks.
pub const FAINT: Color = grey(0.84);

/// `color` with its opacity scaled by `a`.
pub fn faded(color: Color, a: f32) -> Color {
    Color { a: color.a * a, ..color }
}

// ---------------------------------------------------------------------------
// Geometry (pure, tested)
// ---------------------------------------------------------------------------

/// A deterministic value in `[-1, 1)` for detail number `i` of the shape with this seed.
pub fn jitter(seed: u64, i: u32) -> f32 {
    let mut z = seed ^ (u64::from(i) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 23) as f32 - 1.0
}

/// A seed for a rectangle (or a point, with zero size): the same rect always wobbles the same way.
pub fn seed_of(r: Rect) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for v in [r.x, r.y, r.w, r.h] {
        h ^= u64::from(v.to_bits());
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Parallel lines `spacing` apart at `angle` (radians), clipped to `r`. Each comes with an id —
/// its position counted across the whole plane — so hatching clipped differently from frame to
/// frame (or continuing into a neighbouring rect) keeps the same lines.
pub fn hatch_lines(r: Rect, spacing: f32, angle: f32) -> Vec<(i64, Vec2, Vec2)> {
    let d = vec2(angle.cos(), angle.sin());
    let n = vec2(-d.y, d.x);
    let corners = [r.point(), vec2(r.x + r.w, r.y), vec2(r.x, r.y + r.h), vec2(r.x + r.w, r.y + r.h)];
    let (lo, hi) = corners
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), c| (lo.min(c.dot(n)), hi.max(c.dot(n))));
    let mut lines = Vec::new();
    let mut id = (lo / spacing).ceil() as i64;
    while (id as f32) * spacing < hi {
        if let Some((a, b)) = clip_line(n * (id as f32 * spacing), d, r) {
            lines.push((id, a, b));
        }
        id += 1;
    }
    lines
}

/// The part of the line through `p` along `d` inside `r`, if it is longer than half a pixel.
fn clip_line(p: Vec2, d: Vec2, r: Rect) -> Option<(Vec2, Vec2)> {
    let (mut t0, mut t1) = (f32::NEG_INFINITY, f32::INFINITY);
    for (pc, dc, lo, hi) in [(p.x, d.x, r.x, r.x + r.w), (p.y, d.y, r.y, r.y + r.h)] {
        if dc.abs() < 1e-6 {
            if pc < lo || pc > hi {
                return None;
            }
        } else {
            let (a, b) = ((lo - pc) / dc, (hi - pc) / dc);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }
    (t1 - t0 > 0.5).then(|| (p + d * t0, p + d * t1))
}

/// A hand-drawn stroke from `a` to `b`: it wobbles off the straight line a little and
/// overshoots both ends, like a quick pencil line.
pub fn stroke_points(a: Vec2, b: Vec2, seed: u64) -> Vec<Vec2> {
    let len = (b - a).length();
    if len < 0.5 {
        return vec![a, b];
    }
    let d = (b - a) / len;
    let n = vec2(-d.y, d.x);
    let over = (len * 0.04).min(4.0);
    let wobble = (len * 0.012).clamp(0.3, 1.5);
    let start = a - d * over * (0.5 + 0.5 * jitter(seed, 0));
    let end = b + d * over * (0.5 + 0.5 * jitter(seed, 1));
    let segments = ((len / 40.0).ceil() as usize).clamp(1, 8);
    (0..=segments)
        .map(|i| {
            let p = start.lerp(end, i as f32 / segments as f32);
            let off = if i == 0 || i == segments { 0.0 } else { wobble * jitter(seed, 2 + i as u32) };
            p + n * off
        })
        .collect()
}

/// Points along an elliptical arc from angle `from` to `to` (radians; y points down), wobbling
/// a few percent in and out.
pub fn arc_points(c: Vec2, rx: f32, ry: f32, from: f32, to: f32, seed: u64) -> Vec<Vec2> {
    let n = (((to - from).abs() * rx.max(ry) / 5.0) as usize).clamp(6, 64);
    (0..=n)
        .map(|i| {
            let t = from + (to - from) * i as f32 / n as f32;
            c + vec2(t.cos() * rx, t.sin() * ry) * (1.0 + 0.035 * jitter(seed, i as u32))
        })
        .collect()
}

/// A back-and-forth scribble filling `r`, one turn every `step` pixels down.
pub fn scribble_rect_points(r: Rect, step: f32, seed: u64) -> Vec<Vec2> {
    let rows = ((r.h / step).ceil() as u32).max(1);
    (0..=rows)
        .map(|i| {
            let y = r.y + r.h * i as f32 / rows as f32;
            let slack = r.w * (0.15 - 0.15 * jitter(seed, i)) * 0.5; // 0..15% of the width
            let x = if i % 2 == 0 { r.x + slack } else { r.x + r.w - slack };
            vec2(x, y)
        })
        .collect()
}

/// A back-and-forth scribble filling a circle.
pub fn scribble_circle_points(c: Vec2, radius: f32, step: f32, seed: u64) -> Vec<Vec2> {
    let rows = ((2.0 * radius / step).ceil() as u32).max(1);
    (0..=rows)
        .map(|i| {
            let y = -radius + 2.0 * radius * i as f32 / rows as f32;
            let half = (radius * radius - y * y).max(0.0).sqrt() * (0.8 + 0.15 * jitter(seed, i));
            c + vec2(if i % 2 == 0 { -half } else { half }, y)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn polyline(points: &[Vec2], width: f32, color: Color) {
    for s in points.windows(2) {
        draw_line(s[0].x, s[0].y, s[1].x, s[1].y, width, color);
    }
}

/// A sketchy pencil line: a firm stroke plus a lighter second pass that doesn't quite match.
pub fn pencil_line(a: Vec2, b: Vec2, width: f32, color: Color, seed: u64) {
    polyline(&stroke_points(a, b, seed), width, faded(color, 0.9));
    polyline(&stroke_points(a, b, seed ^ 0x5EC0_4D5A), width * 0.6, faded(color, 0.5));
}

/// A rectangle outline made of four pencil lines.
pub fn pencil_rect(r: Rect, width: f32, color: Color, seed: u64) {
    let c = [r.point(), vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
    for i in 0..4 {
        pencil_line(c[i], c[(i + 1) % 4], width, color, seed.wrapping_add(i as u64 * 0x9E37));
    }
}

/// Hatching inside `r`. Each line is pulled in from the edges by a varying amount and drawn a
/// little lighter or darker, so it looks done by hand.
pub fn hatch_rect(r: Rect, spacing: f32, angle: f32, width: f32, color: Color, seed: u64) {
    for (id, a, b) in hatch_lines(r, spacing, angle) {
        let k = (id as u32).wrapping_mul(3);
        let len = (b - a).length();
        let d = (b - a) / len;
        let inset = |j: u32| (1.0 + 2.0 * jitter(seed, j).abs()).min(len * 0.3);
        let (a, b) = (a + d * inset(k), b - d * inset(k + 1));
        draw_line(a.x, a.y, b.x, b.y, width, faded(color, 0.75 + 0.25 * jitter(seed, k + 2)));
    }
}

/// Hatching in both diagonal directions.
pub fn cross_hatch_rect(r: Rect, spacing: f32, width: f32, color: Color, seed: u64) {
    hatch_rect(r, spacing, PI / 4.0, width, color, seed);
    hatch_rect(r, spacing, -PI / 4.0, width, color, seed ^ 0xC055);
}

/// A hand-drawn ellipse: a wobbly loop that runs a little past where it started.
pub fn pencil_ellipse(c: Vec2, rx: f32, ry: f32, width: f32, color: Color, seed: u64) {
    let start = PI * jitter(seed, 9999);
    polyline(&arc_points(c, rx, ry, start, start + TAU * 1.08, seed), width, color);
}

pub fn pencil_circle(c: Vec2, radius: f32, width: f32, color: Color, seed: u64) {
    pencil_ellipse(c, radius, radius, width, color, seed);
}

pub fn pencil_arc(c: Vec2, radius: f32, from: f32, to: f32, width: f32, color: Color, seed: u64) {
    polyline(&arc_points(c, radius, radius, from, to, seed), width, color);
}

/// A hand-drawn arrow pointing along `dir` (a unit vector), centred on `c`.
pub fn pencil_arrow(c: Vec2, dir: Vec2, size: f32, width: f32, color: Color, seed: u64) {
    let tip = c + dir * size * 0.5;
    let side = vec2(-dir.y, dir.x) * size * 0.28;
    let back = dir * size * 0.32;
    pencil_line(c - dir * size * 0.5, tip, width, color, seed);
    pencil_line(tip, tip - back + side, width, color, seed ^ 1);
    pencil_line(tip, tip - back - side, width, color, seed ^ 2);
}

pub fn scribble_rect(r: Rect, step: f32, width: f32, color: Color, seed: u64) {
    polyline(&scribble_rect_points(r, step, seed), width, color);
}

pub fn scribble_circle(c: Vec2, radius: f32, step: f32, width: f32, color: Color, seed: u64) {
    polyline(&scribble_circle_points(c, radius, step, seed), width, color);
}

// ---------------------------------------------------------------------------
// Shop doodles
// ---------------------------------------------------------------------------

/// A little spider: scribbled round body, small head, four bent legs a side.
pub fn doodle_spider(c: Vec2, size: f32, color: Color, seed: u64) {
    let body = size * 0.42;
    scribble_circle(c, body, 2.0, 1.4, color, seed);
    pencil_circle(c, body, 2.0, color, seed ^ 1);
    pencil_circle(c + vec2(0.0, -body - size * 0.18), size * 0.2, 1.8, color, seed ^ 2);
    for k in 0..4u64 {
        let lift = -0.75 + k as f32 * 0.5;
        for (s, side) in [(0u64, -1.0f32), (1, 1.0)] {
            let hip = c + vec2(side * body * 0.6, lift * body * 0.8);
            let knee = c + vec2(side * size * 0.8, lift * size * 0.5 - size * 0.25);
            let foot = knee + vec2(side * size * 0.25, size * 0.45);
            pencil_line(hip, knee, 1.6, color, seed ^ (10 + k * 2 + s));
            pencil_line(knee, foot, 1.6, color, seed ^ (30 + k * 2 + s));
        }
    }
}

/// A horseshoe magnet with hatched tips.
pub fn doodle_magnet(c: Vec2, size: f32, color: Color, seed: u64) {
    let (half, top, bottom) = (size * 0.55, c.y - size * 0.7, c.y + size * 0.15);
    for (k, x) in [(0u64, c.x - half), (1, c.x + half)] {
        pencil_line(vec2(x, top), vec2(x, bottom), 2.2, color, seed ^ k);
        let tip = Rect::new(x - size * 0.16, top, size * 0.32, size * 0.32);
        cross_hatch_rect(tip, 3.0, 1.0, color, seed ^ (4 + k));
        pencil_rect(tip, 1.6, color, seed ^ (6 + k));
    }
    pencil_arc(vec2(c.x, bottom), half, 0.0, PI, 2.2, color, seed ^ 8);
}

/// A cartoon bone: a shaft with two round knobs at each end.
pub fn doodle_bone(c: Vec2, size: f32, color: Color, seed: u64) {
    let (half, t) = (size * 0.75, size * 0.14);
    pencil_line(vec2(c.x - half, c.y - t), vec2(c.x + half, c.y - t), 2.0, color, seed);
    pencil_line(vec2(c.x - half, c.y + t), vec2(c.x + half, c.y + t), 2.0, color, seed ^ 1);
    for (k, x) in [(0u64, c.x - half), (1, c.x + half)] {
        for (j, dy) in [(0u64, -t * 1.6), (1, t * 1.6)] {
            let knob = vec2(x, c.y + dy);
            draw_circle(knob.x, knob.y, size * 0.2, PAPER);
            pencil_circle(knob, size * 0.2, 2.0, color, seed ^ (2 + k * 2 + j));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{PI, TAU};

    fn inside(r: Rect, p: Vec2) -> bool {
        const EPS: f32 = 0.01;
        p.x >= r.x - EPS && p.x <= r.x + r.w + EPS && p.y >= r.y - EPS && p.y <= r.y + r.h + EPS
    }

    #[test]
    fn jitter_is_deterministic_and_in_range() {
        for i in 0..1000 {
            let v = jitter(42, i);
            assert!((-1.0..1.0).contains(&v), "{v}");
            assert_eq!(v, jitter(42, i));
        }
        assert_ne!(jitter(1, 0), jitter(2, 0));
    }

    #[test]
    fn jitter_is_not_lopsided() {
        let mean = (0..10_000).map(|i| jitter(7, i)).sum::<f32>() / 10_000.0;
        assert!(mean.abs() < 0.05, "{mean}");
    }

    #[test]
    fn seed_of_depends_on_every_coordinate() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(seed_of(r), seed_of(r));
        for other in [
            Rect::new(11.0, 20.0, 30.0, 40.0),
            Rect::new(10.0, 21.0, 30.0, 40.0),
            Rect::new(10.0, 20.0, 31.0, 40.0),
            Rect::new(10.0, 20.0, 30.0, 41.0),
        ] {
            assert_ne!(seed_of(r), seed_of(other));
        }
    }

    #[test]
    fn hatch_lines_stay_inside_and_follow_the_spacing() {
        let r = Rect::new(100.0, 50.0, 200.0, 22.0);
        // Horizontal lines: every multiple of 10 from the top edge down to 72.
        let flat = hatch_lines(r, 10.0, 0.0);
        let ys: Vec<f32> = flat.iter().map(|(_, a, _)| a.y).collect();
        assert_eq!(ys, vec![50.0, 60.0, 70.0]);
        for (_, a, b) in &flat {
            assert_eq!((a.x, b.x), (100.0, 300.0));
        }
        for angle in [PI / 4.0, -PI / 4.0, PI / 2.0] {
            let lines = hatch_lines(r, 10.0, angle);
            assert!(lines.len() >= 2);
            for (_, a, b) in &lines {
                assert!(inside(r, *a) && inside(r, *b), "{a} {b} outside {r:?}");
            }
            for w in lines.windows(2) {
                assert_eq!(w[1].0, w[0].0 + 1, "one line per spacing step");
            }
        }
    }

    #[test]
    fn hatch_ids_survive_different_clipping() {
        // The same line keeps its id whichever slice of it is visible, so hatching doesn't
        // change as the camera scrolls.
        let angle = PI / 4.0;
        let n = vec2(-angle.sin(), angle.cos());
        let wide = hatch_lines(Rect::new(0.0, 0.0, 1000.0, 60.0), 7.0, angle);
        let slice = hatch_lines(Rect::new(300.0, 0.0, 200.0, 60.0), 7.0, angle);
        for (id, a, _) in &slice {
            let (_, wa, _) = wide.iter().find(|(wid, _, _)| wid == id).expect("same line in the wide rect");
            assert!((a.dot(n) - wa.dot(n)).abs() < 0.01);
        }
    }

    #[test]
    fn strokes_wobble_a_little_and_overshoot_the_ends() {
        let (a, b) = (vec2(10.0, 10.0), vec2(210.0, 10.0));
        let pts = stroke_points(a, b, 99);
        assert_eq!(pts, stroke_points(a, b, 99));
        for p in &pts {
            assert!((p.y - 10.0).abs() <= 1.5 + 1e-4, "{p}");
        }
        let (first, last) = (pts[0], pts[pts.len() - 1]);
        assert!(first.x <= a.x && first.x >= a.x - 4.0, "{first}");
        assert!(last.x >= b.x && last.x <= b.x + 4.0, "{last}");
    }

    #[test]
    fn arcs_stay_near_their_radius() {
        let c = vec2(50.0, 50.0);
        for p in arc_points(c, 20.0, 20.0, 0.0, TAU, 5) {
            let d = (p - c).length() / 20.0;
            assert!((0.96..=1.04).contains(&d), "{d}");
        }
    }

    #[test]
    fn scribbles_stay_inside_their_shape() {
        let r = Rect::new(0.0, 0.0, 44.0, 17.6);
        for p in scribble_rect_points(r, 2.5, 3) {
            assert!(inside(r, p), "{p}");
        }
        for p in scribble_circle_points(Vec2::ZERO, 22.0, 2.5, 3) {
            assert!(p.length() <= 22.0 + 1e-3, "{p}");
        }
    }
}
