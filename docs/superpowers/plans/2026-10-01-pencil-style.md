# Pencil-on-Paper Look Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redraw the whole game as graphite pencil on white paper — strictly black and white, sketchy but still lines, handwritten text.

**Architecture:** A new `src/sketch.rs` holds the grey palette and pencil drawing built on small pure geometry functions (seeded wobble, clipped hatching, arcs, scribbles) that are unit-tested. All drawing code moves from `src/main.rs` into a new `src/render.rs` (pure move first), then the font, the world, the HUD and the menu screens are redrawn there one task at a time, each checked with screenshots of the real web build.

**Tech Stack:** Rust 2024, macroquad (git rev 8d602d3), Patrick Hand TTF (SIL OFL 1.1), `cargo test`, headless Chromium + playwright-core + pngjs in `/tmp/gravity-fs-test` (not committed).

Spec: `docs/superpowers/specs/2026-10-01-pencil-style-design.md`

## Global Constraints

- Palette (all `r = g = b`): `PAPER` 0.95, `INK` 0.13, `GRAPHITE` 0.32, `SHADE` 0.58, `FAINT` 0.84. No other colours anywhere on screen; only alpha varies. (`WHITE` may be used only as a texture tint, where it means "no tint".)
- Still lines: every stroke's wobble comes from a seed that is the same every frame — `seed_of(rect)` for world shapes, the chunk index for per-chunk lines, fixed constants for HUD and menus. Never use `rand::gen_range` or time to shape a stroke.
- Font: Patrick Hand Regular and its `OFL.txt` from `https://raw.githubusercontent.com/google/fonts/main/ofl/patrickhand/`, stored in `assets/` and embedded with `include_bytes!`.
- Glyph warm-up uses `(size as f32 * screen_dpi_scale()).ceil() as u16`, the size macroquad looks glyphs up at.
- The working tree has unrelated uncommitted changes (`web/index.html`, `.gitignore`); every commit adds only the files its task names.
- Out of scope: line boil, colour, sound, gameplay changes, layout changes beyond what the font requires.

## File Structure

- **Create `src/sketch.rs`** — palette, pure geometry (`jitter`, `seed_of`, `hatch_lines`, `stroke_points`, `arc_points`, `scribble_rect_points`, `scribble_circle_points`), drawing (`pencil_line`, `pencil_rect`, `hatch_rect`, `cross_hatch_rect`, `pencil_ellipse`, `pencil_circle`, `pencil_arc`, `pencil_arrow`, `scribble_rect`, `scribble_circle`) and shop doodles; unit tests.
- **Create `src/render.rs`** — everything that draws: `impl Game` drawing methods, text helpers, font, paper grain.
- **Create `assets/PatrickHand-Regular.ttf`, `assets/OFL.txt`.**
- **Modify `src/main.rs`** — `mod sketch; mod render;`, drawing code removed, `Popup.color` → `Popup.big`, `PowerUp::color` removed.

## Test harness (outside the repo)

`/tmp/gravity-fs-test` already has `run.sh` (serves `web/` on `127.0.0.1:8792`, then runs `node "${1:-test.mjs}"`), `seed-check.mjs`, `test.mjs` (fullscreen), `texsize-probe.mjs`, and Chromium at `$HOME/Library/Caches/ms-playwright/chromium-1234`. Task 2 adds `shots.mjs` and `grey-check.mjs`.

---

### Task 1: Pencil geometry and drawing (`src/sketch.rs`)

**Files:**
- Create: `src/sketch.rs`
- Modify: `src/main.rs:3` (add `mod sketch;`)

**Interfaces:**
- Produces (all `pub`): `grey(v: f32) -> Color`; consts `PAPER, INK, GRAPHITE, SHADE, FAINT: Color`; `faded(color: Color, a: f32) -> Color`; `jitter(seed: u64, i: u32) -> f32`; `seed_of(r: Rect) -> u64`; `hatch_lines(r: Rect, spacing: f32, angle: f32) -> Vec<(i64, Vec2, Vec2)>`; `stroke_points(a: Vec2, b: Vec2, seed: u64) -> Vec<Vec2>`; `arc_points(c: Vec2, rx: f32, ry: f32, from: f32, to: f32, seed: u64) -> Vec<Vec2>`; `scribble_rect_points(r: Rect, step: f32, seed: u64) -> Vec<Vec2>`; `scribble_circle_points(c: Vec2, radius: f32, step: f32, seed: u64) -> Vec<Vec2>`; `pencil_line(a: Vec2, b: Vec2, width: f32, color: Color, seed: u64)`; `pencil_rect(r: Rect, width: f32, color: Color, seed: u64)`; `hatch_rect(r: Rect, spacing: f32, angle: f32, width: f32, color: Color, seed: u64)`; `cross_hatch_rect(r: Rect, spacing: f32, width: f32, color: Color, seed: u64)`; `pencil_ellipse(c: Vec2, rx: f32, ry: f32, width: f32, color: Color, seed: u64)`; `pencil_circle(c: Vec2, radius: f32, width: f32, color: Color, seed: u64)`; `pencil_arc(c: Vec2, radius: f32, from: f32, to: f32, width: f32, color: Color, seed: u64)`; `pencil_arrow(c: Vec2, dir: Vec2, size: f32, width: f32, color: Color, seed: u64)`; `scribble_rect(r: Rect, step: f32, width: f32, color: Color, seed: u64)`; `scribble_circle(c: Vec2, radius: f32, step: f32, width: f32, color: Color, seed: u64)`; `doodle_spider/doodle_magnet/doodle_bone(c: Vec2, size: f32, color: Color, seed: u64)`.

- [ ] **Step 1: Write the failing tests**

Create `src/sketch.rs`:

```rust
//! Pencil on paper: a grey palette and sketchy strokes, hatching, scribbles and doodles.
//!
//! Every wobble comes from a seed, so a shape drawn with the same seed looks exactly the same
//! every frame ("still lines") and on every computer.

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
```

In `src/main.rs`, change line 3 from `mod seed;` to:

```rust
mod seed;
mod sketch;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test sketch::`
Expected: compile errors — cannot find `Rect`, `vec2`, `jitter`, `seed_of`, `hatch_lines`, `stroke_points`, `arc_points`, `scribble_rect_points`, `scribble_circle_points`.

- [ ] **Step 3: Implement**

Insert between the doc comment and `#[cfg(test)]` in `src/sketch.rs`:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: `17 passed; 0 failed` (9 existing + 8 new). Dead-code warnings for `sketch` functions are expected until Task 4 uses them.

- [ ] **Step 5: Commit**

```bash
git add src/sketch.rs src/main.rs
git commit -m "Add pencil drawing helpers: seeded wobble, clipped hatching, scribbles, doodles"
```

---

### Task 2: Move all drawing into `src/render.rs` (no visual change)

**Files:**
- Create: `src/render.rs`
- Modify: `src/main.rs` — remove lines 205–240 (`draw_arrow` … `text_centered`) and 1282–1789 (the `// Drawing` section of `impl Game`); add `mod render;`; call `render::warm_font_cache()`.
- Test (not committed): `/tmp/gravity-fs-test/shots.mjs`, `/tmp/gravity-fs-test/grey-check.mjs`

**Interfaces:**
- Produces: `render::warm_font_cache()` and `Game::draw(&self)`, both `pub(crate)`; private in `render.rs`: `font_size`, `draw_label`, `text_centered`, `draw_arrow` and the `Game::draw_*` methods.

- [ ] **Step 1: Add the screenshot harness and take the "before" set**

Create `/tmp/gravity-fs-test/shots.mjs`:

```js
// Screenshots of every screen of the web build. Usage: node shots.mjs <outdir> [dpr]
import { chromium } from "playwright-core";
import { mkdirSync } from "node:fs";

const CHROME = `${process.env.HOME}/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;
const [out = "shots", dpr = "1"] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({
    executablePath: CHROME,
    headless: true,
    args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const open = async () => {
    const page = await browser.newPage({ viewport: { width: 1280, height: 760 }, deviceScaleFactor: Number(dpr) });
    await page.goto("http://127.0.0.1:8792/index.html");
    await page.waitForFunction(() => typeof wasm_exports !== "undefined" && wasm_exports, null, { timeout: 20000 });
    await page.evaluate(() => canvas.focus());
    await page.waitForTimeout(500);
    return page;
};
const shot = (page, name) => page.screenshot({ path: `${out}/${name}.png` });

let page = await open();
await shot(page, "1-title");
await page.keyboard.press("KeyS");
await page.waitForTimeout(200);
await page.keyboard.type("pencil");
await page.waitForTimeout(300);
await shot(page, "2-seed-box");
await page.keyboard.press("Escape");
await page.waitForTimeout(200);
await page.keyboard.press("KeyB");
await page.waitForTimeout(300);
await shot(page, "3-shop");
await page.close();

// Gameplay in a fixed world ("PENCIL"), so every run shows the same level.
page = await open();
await page.keyboard.press("KeyS");
await page.waitForTimeout(200);
await page.keyboard.type("pencil");
await page.keyboard.press("Enter");
await page.waitForTimeout(800);
await shot(page, "4-play-start");
await page.keyboard.down("KeyD");
await page.waitForTimeout(1500);
await page.keyboard.up("KeyD");
await shot(page, "5-play-moved");
// Sample once a second through the urgent countdown and the first gravity flip.
for (let s = 0; s < 9; s++) {
    await page.waitForTimeout(1000);
    await shot(page, `6-play-${s}`);
}
await browser.close();
console.log(`wrote ${out}/*.png`);
```

Create `/tmp/gravity-fs-test/grey-check.mjs`:

```js
// Counts pixels that are not grey (any channel more than 6 apart). Usage: node grey-check.mjs a.png [b.png ...]
import { PNG } from "pngjs";
import { readFileSync } from "node:fs";

let bad = 0;
for (const file of process.argv.slice(2)) {
    const png = PNG.sync.read(readFileSync(file));
    let coloured = 0;
    for (let i = 0; i < png.data.length; i += 4) {
        const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
        if (Math.max(r, g, b) - Math.min(r, g, b) > 6) coloured++;
    }
    console.log(`${coloured === 0 ? "GREY " : "COLOUR"}  ${file}  (${coloured} coloured pixels)`);
    if (coloured) bad++;
}
process.exit(bad ? 1 : 0);
```

`run.sh` ends with `node "${1:-test.mjs}"`, which passes "script + arguments" as one word. Let it split, then install the PNG reader and take the "before" set:

```bash
sed -i '' 's|^node "${1:-test.mjs}"$|node ${1:-test.mjs}|' /tmp/gravity-fs-test/run.sh && tail -1 /tmp/gravity-fs-test/run.sh
cd /tmp/gravity-fs-test && npm install --silent --no-audit --no-fund pngjs
cd /Users/virenvijaymane/Rust_Projects/gravity && ./build_web.sh
/tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/before"
```

Expected: the `tail` prints `node ${1:-test.mjs}`; then `wrote /tmp/gravity-fs-test/before/*.png` (14 files).

- [ ] **Step 2: Move the code**

Line numbers shifted by one in Task 1, so find the two blocks by content and print their edges before cutting:

```bash
cd /Users/virenvijaymane/Rust_Projects/gravity
s1=$(grep -n '^fn draw_arrow' src/main.rs | cut -d: -f1)
e1=$(( $(grep -n '^fn text_centered' src/main.rs | cut -d: -f1) + 3 ))
s2=$(( $(grep -n '^    // Drawing$' src/main.rs | cut -d: -f1) - 1 ))
e2=$(( $(grep -n '^fn window_conf' src/main.rs | cut -d: -f1) - 3 ))
for n in $s1 $e1 $s2 $e2; do printf '%5d: %s\n' "$n" "$(sed -n "${n}p" src/main.rs)"; done
```

Expected output, in order: `fn draw_arrow(...) {`, `}` (end of `text_centered`), `    // ----…` (the Drawing banner's first line) and `    }` (end of `draw_overlay`). Only then:

```bash
{
  printf '%s\n' '//! Everything that draws the game: the world, the HUD and the menu screens.' '' 'use super::*;' 'use macroquad::prelude::*;' ''
  sed -n "${s1},${e1}p" src/main.rs
  printf '\n%s\n' 'impl Game {'
  sed -n "${s2},${e2}p" src/main.rs
  printf '%s\n' '}'
} > src/render.rs
sed -i '' -e "${s2},${e2}d" -e "${s1},${e1}d" src/main.rs
```

Then in `src/main.rs`:
- change `mod sketch;` to:

```rust
mod render;
mod sketch;
```

- in `main()`, change `warm_font_cache();` to `render::warm_font_cache();`

In `src/render.rs`:
- change `fn warm_font_cache() {` to `pub(crate) fn warm_font_cache() {`
- change `    fn draw(&self) {` to `    pub(crate) fn draw(&self) {`

- [ ] **Step 3: Build and test**

Run: `cargo test && cargo build --release && ./build_web.sh`
Expected: `17 passed`; both builds succeed. (If `render.rs` can't see `seed::MAX_LEN`, write it as `crate::seed::MAX_LEN`.)

- [ ] **Step 4: Take the "after" set and compare**

Run: `/tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/after-move"`
View `before/1-title.png`, `before/3-shop.png`, `before/4-play-start.png` and the same files in `after-move/`.
Expected: identical layouts and colours (only animation — coin spin, title wobble, blinking — may differ).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/render.rs
git commit -m "Move all drawing code into render.rs"
```

---

### Task 3: Handwritten font

**Files:**
- Create: `assets/PatrickHand-Regular.ttf`, `assets/OFL.txt`
- Modify: `src/render.rs` (text section and every `measure_text`/`draw_text_ex` call)

**Interfaces:**
- Produces (private in `render.rs`): `with_font<R>(f: impl FnOnce(&Font) -> R) -> R`, `measure(text: &str, size: f32) -> TextDimensions`; `draw_label` / `text_centered` keep their signatures.

- [ ] **Step 1: Record the failing check**

Run: `/tmp/gravity-fs-test/run.sh texsize-probe.mjs` — note the atlas sizes; view `after-move/1-title.png`.
Expected (current, failing the goal): blocky pixel font.

- [ ] **Step 2: Add the font**

```bash
mkdir -p assets
curl -sL -o assets/PatrickHand-Regular.ttf https://raw.githubusercontent.com/google/fonts/main/ofl/patrickhand/PatrickHand-Regular.ttf
curl -sL -o assets/OFL.txt https://raw.githubusercontent.com/google/fonts/main/ofl/patrickhand/OFL.txt
ls -l assets   # expect ~214772 bytes and ~4376 bytes
```

- [ ] **Step 3: Replace the text section**

In `src/render.rs`, replace everything from the `/// Every font size text is drawn at.` doc comment through the end of `fn text_centered` with:

```rust
// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Every font size text is drawn at. Their glyphs are rasterised up front so the atlas doesn't
/// have to grow mid-game.
const FONT_SIZES: [u16; 16] = [16, 20, 22, 24, 26, 28, 30, 32, 34, 36, 40, 56, 60, 64, 80, 110];
/// From this size up, text is headings and numbers, so only these characters are pre-rasterised.
const BIG_TEXT: u16 = 56;
const BIG_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -:._!?+";

thread_local! {
    /// Patrick Hand by Patrick Wagesreiter, SIL Open Font License 1.1 (assets/OFL.txt).
    static FONT: Font = load_ttf_font_from_bytes(include_bytes!("../assets/PatrickHand-Regular.ttf"))
        .expect("the bundled font loads");
}

fn with_font<R>(f: impl FnOnce(&Font) -> R) -> R {
    FONT.with(f)
}

pub(crate) fn warm_font_cache() {
    // macroquad looks glyphs up at the size scaled for the screen's pixel density.
    let dpi = screen_dpi_scale();
    let all: Vec<char> = (32u8..127).map(char::from).collect();
    let big: Vec<char> = BIG_CHARS.chars().collect();
    with_font(|font| {
        for size in FONT_SIZES {
            let chars = if size >= BIG_TEXT { &big } else { &all };
            font.populate_font_cache(chars, (size as f32 * dpi).ceil() as u16);
        }
    });
}

/// Snap to the nearest pre-cached size so no new glyphs are needed.
fn font_size(size: f32) -> u16 {
    *FONT_SIZES.iter().min_by_key(|s| (**s as i32 - size.round() as i32).abs()).unwrap()
}

fn measure(text: &str, size: f32) -> TextDimensions {
    with_font(|font| measure_text(text, Some(font), font_size(size), 1.0))
}

fn draw_label(text: &str, x: f32, y: f32, size: f32, color: Color) {
    with_font(|font| {
        draw_text_ex(text, x, y, TextParams { font: Some(font), font_size: font_size(size), color, ..Default::default() });
    });
}

fn text_centered(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_label(text, x - measure(text, size).width / 2.0, y, size, color);
}
```

- [ ] **Step 4: Route every measurement and the title through the font**

In `src/render.rs` make these exact replacements:
- `measure_text(&world_line, None, font_size(16.0), 1.0).width` → `measure(&world_line, 16.0).width`
- `let dims = measure_text(text, None, font_size(28.0), 1.0);` → `let dims = measure(text, 28.0);`
- `let dims = measure_text(title, None, font_size(size), 1.0);` → `let dims = measure(title, size);`
- `measure_text(&candidate, None, font_size(20.0), 1.0).width` → `measure(&candidate, 20.0).width`
- `measure_text(&self.seed_input, None, font_size(56.0), 1.0).width` → `measure(&self.seed_input, 56.0).width`
- the title's draw call becomes:

```rust
        with_font(|font| {
            draw_text_ex(
                title,
                w / 2.0 - dims.width / 2.0,
                h / 2.0 - 90.0,
                TextParams { font: Some(font), font_size: font_size(size), rotation: wobble, color: WHITE, ..Default::default() },
            );
        });
```

Check: `grep -n 'measure_text\|draw_text(' src/render.rs` shows only the uses inside `measure` and nothing else.

- [ ] **Step 5: Build, test, look**

Run: `cargo test && ./build_web.sh && /tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/font"`, then view `font/1-title.png`, `font/3-shop.png`, `font/4-play-start.png`.
Expected: all text handwritten; nothing overflows its box (the top-right box, shop cards, AFK banner); no black boxes.

- [ ] **Step 6: Atlas size at 1× and 2×**

Run (servers via `run.sh` are per-run, so start one for both probes):

```bash
cd /tmp/gravity-fs-test && python3 -m http.server 8792 --bind 127.0.0.1 --directory /Users/virenvijaymane/Rust_Projects/gravity/web >/dev/null 2>&1 & S=$!
curl -s -o /dev/null --retry 10 --retry-connrefused --retry-delay 1 http://127.0.0.1:8792/index.html
cd /tmp/gravity-fs-test && BASE=http://127.0.0.1:8792 node texsize-probe.mjs "font @1x" && DPR=2 BASE=http://127.0.0.1:8792 node texsize-probe.mjs "font @2x"; kill $S
```

Expected: the largest `texImage2D` size is at most `4096x4096` in both.

- [ ] **Step 7: Commit**

```bash
git add assets/PatrickHand-Regular.ttf assets/OFL.txt src/render.rs
git commit -m "Draw all text in the Patrick Hand handwriting font"
```

---

### Task 4: The world in pencil

**Files:**
- Modify: `src/render.rs` — `draw`, `draw_background`, `draw_world`, `draw_web`, `draw_monster`, `draw_stickman`; add `draw_boundaries`, paper grain.
- Modify: `src/main.rs` — `struct Popup` field `color: Color` → `big: bool`; the four `Popup { .. }` literals.

**Interfaces:**
- Consumes: everything from Task 1; `with_font`, `measure`, `draw_label`, `text_centered` (Task 3).
- Produces: `Popup.big: bool` (damage popups are `true`).

- [ ] **Step 1: The failing check**

Run: `node /tmp/gravity-fs-test/grey-check.mjs /tmp/gravity-fs-test/font/4-play-start.png`
Expected: `COLOUR` (thousands of coloured pixels).

- [ ] **Step 2: Popups carry "big" instead of a colour**

In `src/main.rs`, in `struct Popup`, replace `    color: Color,` with:

```rust
    /// Damage numbers: drawn larger, pressed hard.
    big: bool,
```

Then replace the `color: ...,` line in each popup literal:
- spider web (`text: "SPIDER WEB!"`): `color: PowerUp::Spider.color(),` → `big: false,`
- coin (`text: "+1"`): `color: Color::new(1.0, 0.85, 0.2, 1.0),` → `big: false,`
- cheat (`text: "+50 (cheat)"`): `color: Color::new(1.0, 0.85, 0.2, 1.0),` → `big: false,`
- damage (in `hurt`): `color: Color::new(1.0, 0.3, 0.3, 1.0),` → `big: true,`

- [ ] **Step 3: Imports and paper grain**

At the top of `src/render.rs`, after `use super::*;`, add:

```rust
use crate::sketch::*;
use std::f32::consts::PI;
```

Add after the text section:

```rust
// ---------------------------------------------------------------------------
// Paper
// ---------------------------------------------------------------------------

const GRAIN_PX: u16 = 128;
/// World units covered by one tile of grain.
const GRAIN_TILE: f32 = 256.0;

thread_local! {
    /// Faint paper grain, made once and tiled across the world.
    static GRAIN: Texture2D = make_grain();
}

fn make_grain() -> Texture2D {
    let mut img = Image::gen_image_color(GRAIN_PX, GRAIN_PX, Color::new(0.0, 0.0, 0.0, 0.0));
    let n = u32::from(GRAIN_PX);
    for y in 0..n {
        for x in 0..n {
            let v = jitter(0x6EA1_9A1D, y * n + x);
            // Mostly a faint tooth, with a few darker specks.
            let a = if v > 0.97 { 0.09 } else { 0.025 * (v + 1.0) };
            img.set_pixel(x, y, Color { a, ..INK });
        }
    }
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Linear);
    tex
}

/// Tiles the grain over the part of the world in view. It is fixed to the world, so the paper
/// scrolls with the level.
fn draw_paper_grain(left: f32, right: f32) {
    GRAIN.with(|tex| {
        let mut x = (left / GRAIN_TILE).floor() * GRAIN_TILE;
        while x < right {
            let mut y = (-BOUNDARY / GRAIN_TILE).floor() * GRAIN_TILE;
            while y < WORLD_HEIGHT + BOUNDARY {
                let params = DrawTextureParams { dest_size: Some(vec2(GRAIN_TILE, GRAIN_TILE)), ..Default::default() };
                draw_texture_ex(tex, x, y, WHITE, params);
                y += GRAIN_TILE;
            }
            x += GRAIN_TILE;
        }
    });
}
```

- [ ] **Step 4: Redraw the world**

In `draw`, change `clear_background(Color::from_rgba(12, 12, 24, 255));` to `clear_background(PAPER);`.

Replace `draw_background`, `draw_world`, `draw_web`, `draw_monster` and `draw_stickman` with:

```rust
    fn draw_background(&self) {
        let (w, h) = (screen_width(), screen_height());
        // Graphite flecks drift with gravity so you can always feel which way is down.
        let tail_dir = self.gravity.vec();
        for s in &self.stars {
            let p = vec2(s.pos.x * w, s.pos.y * h);
            let tail = tail_dir * 6.0 * s.depth;
            draw_line(p.x, p.y, p.x - tail.x, p.y - tail.y, 1.2, faded(FAINT, 0.35 + 0.4 * s.depth));
        }
    }

    fn draw_world(&self) {
        let half_w = self.view_size().x / 2.0 + 100.0;
        let (left, right) = ((self.cam.x - half_w).max(-BOUNDARY), self.cam.x + half_w);

        draw_paper_grain(left, right);

        // Graph paper is printed, so its lines are straight.
        let mut x = (left / 100.0).floor() * 100.0;
        while x < right {
            draw_line(x, 0.0, x, WORLD_HEIGHT, 1.0, FAINT);
            x += 100.0;
        }
        let mut y = 100.0;
        while y < WORLD_HEIGHT {
            draw_line(left, y, right, y, 1.0, FAINT);
            y += 100.0;
        }

        // Distance markers every 50 m, plus where your best run ended.
        let marker = 50.0 * UNITS_PER_METRE;
        let mut mx = (left / marker).ceil().max(1.0) * marker;
        while mx < right {
            let seed = seed_of(Rect::new(mx, 0.0, 0.0, 0.0));
            pencil_line(vec2(mx, 0.0), vec2(mx, WORLD_HEIGHT), 2.0, faded(SHADE, 0.7), seed);
            text_centered(&format!("{:.0} m", mx / UNITS_PER_METRE), mx, WORLD_HEIGHT / 2.0, 40.0, SHADE);
            mx += marker;
        }
        let best = self.best_before_run;
        if best > 0.0 && best > left && best < right {
            let pulse = 0.5 + 0.5 * (get_time() as f32 * 3.0).sin();
            let mut y = 0.0;
            while y < WORLD_HEIGHT {
                let seed = seed_of(Rect::new(best, y, 0.0, 30.0));
                pencil_line(vec2(best, y), vec2(best, y + 30.0), 2.5, faded(INK, 0.5 + 0.4 * pulse), seed);
                y += 50.0;
            }
            text_centered("BEST", best, WORLD_HEIGHT / 2.0 - 50.0, 36.0, INK);
        }

        draw_label("START", 60.0, WORLD_HEIGHT - 70.0, 30.0, SHADE);

        self.draw_boundaries(left, right);
        for s in &self.world.solids_between(left, right) {
            let r = s.rect;
            match s.kind {
                // Floor, ceiling and start wall are drawn as continuous bands; webs below.
                Kind::Boundary | Kind::Web => {}
                Kind::Bump => {
                    cross_hatch_rect(r, 5.0, 1.2, GRAPHITE, seed_of(r));
                    pencil_rect(r, 2.2, INK, seed_of(r));
                }
                Kind::Platform => {
                    hatch_rect(r, 9.0, PI / 4.0, 1.2, SHADE, seed_of(r));
                    pencil_rect(r, 2.2, INK, seed_of(r));
                }
            }
        }

        for d in &self.dust {
            draw_circle(d.pos.x, d.pos.y, 1.5 + 2.0 * d.life, faded(SHADE, (d.life * 1.6).min(1.0)));
        }

        let t = get_time() as f32;
        for i in self.world.chunk_range(left, right) {
            for c in self.world.chunks[i].coins.iter().filter(|c| !c.taken) {
                // Spinning coin: squash its width over time.
                let spin = (t * 3.0 + c.home.x * 0.01).cos().abs().max(0.15);
                let seed = seed_of(Rect::new(c.home.x, c.home.y, 0.0, 0.0));
                let rx = COIN_RADIUS * spin;
                draw_ellipse(c.pos.x, c.pos.y, rx, COIN_RADIUS, 0.0, PAPER);
                pencil_ellipse(c.pos, rx, COIN_RADIUS, 2.0, INK, seed);
                pencil_ellipse(c.pos, rx * 0.55, COIN_RADIUS * 0.55, 1.2, GRAPHITE, seed ^ 1);
                // Two little shine ticks.
                let s = c.pos + vec2(COIN_RADIUS + 2.0, -COIN_RADIUS - 2.0);
                draw_line(s.x, s.y, s.x + 4.0, s.y - 4.0, 1.2, GRAPHITE);
                draw_line(s.x + 2.0, s.y + 4.0, s.x + 6.5, s.y + 2.0, 1.2, GRAPHITE);
            }
        }

        for (web, life) in &self.webs {
            self.draw_web(&web.rect, *life);
        }

        self.draw_stickman();
        for m in &self.monsters {
            self.draw_monster(m);
        }

        for pp in &self.popups {
            let c = faded(INK, pp.life.min(1.0));
            if pp.big {
                // Pressed hard: drawn twice, a pixel apart.
                text_centered(&pp.text, pp.pos.x, pp.pos.y, 56.0, c);
                text_centered(&pp.text, pp.pos.x + 1.0, pp.pos.y, 56.0, c);
            } else {
                text_centered(&pp.text, pp.pos.x, pp.pos.y, 36.0, c);
            }
        }
    }

    /// Floor, ceiling and the wall behind the start: dense cross-hatching with a firm inner edge.
    /// The hatching is one band across the view (not per chunk), so it has no seams.
    fn draw_boundaries(&self, left: f32, right: f32) {
        let band = |r: Rect, seed: u64| cross_hatch_rect(r, 7.0, 1.3, GRAPHITE, seed);
        band(Rect::new(left, WORLD_HEIGHT, right - left, BOUNDARY), 0xF100);
        band(Rect::new(left, -BOUNDARY, right - left, BOUNDARY), 0xCE11);
        if left < 0.0 {
            band(Rect::new(-BOUNDARY, -BOUNDARY, BOUNDARY, WORLD_HEIGHT + 2.0 * BOUNDARY), 0x3A11);
            pencil_line(vec2(0.0, 0.0), vec2(0.0, WORLD_HEIGHT), 2.6, INK, 0x3A12);
        }
        // Inner faces: one pencil line per chunk, so the lines never move.
        for i in self.world.chunk_range(left, right) {
            let x0 = i as f32 * CHUNK_W;
            pencil_line(vec2(x0, WORLD_HEIGHT), vec2(x0 + CHUNK_W, WORLD_HEIGHT), 2.6, INK, 0xF1_0000 + i as u64);
            pencil_line(vec2(x0, 0.0), vec2(x0 + CHUNK_W, 0.0), 2.6, INK, 0xCE_0000 + i as u64);
        }
    }

    fn draw_web(&self, r: &Rect, life: f32) {
        let a = life.min(1.0);
        let strand = faded(GRAPHITE, 0.85 * a);
        let thread = faded(SHADE, 0.7 * a);
        let horizontal = r.w > r.h;
        // Anchor strands run along the web's length, sagging lines connect them.
        let (len, start, dir, across) = if horizontal {
            (r.w, vec2(r.x, r.y + r.h / 2.0), vec2(1.0, 0.0), vec2(0.0, 1.0))
        } else {
            (r.h, vec2(r.x + r.w / 2.0, r.y), vec2(0.0, 1.0), vec2(1.0, 0.0))
        };
        let end = start + dir * len;
        draw_line(start.x, start.y, end.x, end.y, 2.0, strand);
        let center = start + dir * len / 2.0;
        let n = 8;
        for i in 0..=n {
            let p = start + dir * len * i as f32 / n as f32;
            let off = across * (if i % 2 == 0 { -1.0 } else { 1.0 }) * r.w.min(r.h) * 1.2;
            draw_line(center.x, center.y, p.x + off.x, p.y + off.y, 1.2, thread);
            if i < n {
                let q = start + dir * len * (i + 1) as f32 / n as f32;
                draw_line(p.x + off.x, p.y + off.y, q.x - off.x, q.y - off.y, 1.2, thread);
            }
        }
        // Little spider hanging at the middle.
        let ink = faded(INK, a);
        draw_circle(center.x, center.y, 5.0, ink);
        for k in 0..4 {
            let ang = k as f32 * 0.5 - 0.75;
            for side in [-1.0f32, 1.0] {
                let leg = rotate(vec2(side * 9.0, 0.0), ang * side);
                draw_line(center.x, center.y, center.x + leg.x, center.y + leg.y, 1.2, ink);
            }
        }
    }

    fn draw_monster(&self, m: &Monster) {
        let t = get_time() as f32 + m.phase;
        let a = m.alpha.clamp(0.0, 1.0);
        let r = MONSTER_RADIUS;
        let c = m.pos + vec2(0.0, (t * 3.0).sin() * 4.0);
        let seed = u64::from(m.phase.to_bits());
        let body = faded(GRAPHITE, 0.9 * a);

        // A scribbled ghost: round top, square middle, wavy skirt...
        scribble_circle(c, r, 2.5, 1.6, body, seed);
        scribble_rect(Rect::new(c.x - r, c.y, r * 2.0, r * 0.8), 2.5, 1.6, body, seed ^ 1);
        for i in 0..4 {
            let x = c.x - r + i as f32 * r * 0.5;
            let dip = r * (1.1 + 0.25 * (t * 8.0 + i as f32).sin());
            draw_triangle(vec2(x, c.y + r * 0.8), vec2(x + r * 0.5, c.y + r * 0.8), vec2(x + r * 0.25, c.y + dip), body);
        }
        // ...outlined in ink: the top half of the circle and both sides.
        let ink = faded(INK, a);
        pencil_arc(c, r, PI, 2.0 * PI, 2.0, ink, seed ^ 2);
        pencil_line(vec2(c.x - r, c.y), vec2(c.x - r, c.y + r * 0.8), 2.0, ink, seed ^ 3);
        pencil_line(vec2(c.x + r, c.y), vec2(c.x + r, c.y + r * 0.8), 2.0, ink, seed ^ 4);
        // Paper-white eyes that track you, under angry brows.
        let look = (self.player.pos - c).normalize_or_zero() * 3.0;
        for side in [-1.0f32, 1.0] {
            let e = c + vec2(side * r * 0.38, -r * 0.15);
            draw_circle(e.x, e.y, 5.5, faded(PAPER, a));
            draw_circle(e.x + look.x, e.y + look.y, 2.6, ink);
            draw_line(e.x - side * 7.0, e.y - 10.0, e.x + side * 5.0, e.y - 6.0, 2.5, ink);
        }
        text_centered("AFK", c.x, c.y + r * 0.75, 16.0, faded(PAPER, 0.9 * a));
    }

    fn draw_stickman(&self) {
        let p = &self.player;
        if p.invulnerable > 0.0 && (get_time() * 20.0).sin() > 0.0 {
            return;
        }
        // Somersault forward during a double jump.
        let spin = (1.0 - p.flip_spin / DOUBLE_JUMP_SPIN_TIME) * std::f32::consts::TAU;
        let a = p.draw_angle + if p.flip_spin > 0.0 { spin * p.facing } else { 0.0 };
        // Hurt or low on health: the whole figure shakes (its strokes keep their shape).
        let t = get_time() as f32;
        let shake_amount = 3.0 * p.hurt_flash + if p.health < 30.0 { 1.2 } else { 0.0 };
        let pos = p.pos + vec2((t * 47.0).sin(), (t * 39.0).cos()) * shake_amount;
        let to_world = |v: Vec2| pos + rotate(vec2(v.x * p.facing, v.y), a);
        let width = 3.2 + 1.5 * p.hurt_flash;
        let line = |from: Vec2, to: Vec2, k: u64| pencil_line(to_world(from), to_world(to), width, INK, 0x5717_C4A9 + k);

        let moving = p.grounded && p.vel.length() > 30.0;
        let swing = if moving { p.walk_phase.sin() } else { 0.0 };
        let head = vec2(0.0, -15.0);
        let neck = vec2(0.0, -7.0);
        let hip = vec2(0.0, 7.0);
        let shoulder = vec2(0.0, -3.0);

        // Legs
        if p.grounded {
            line(hip, vec2(6.0 + swing * 8.0, 24.0), 1);
            line(hip, vec2(-6.0 - swing * 8.0, 24.0), 2);
        } else {
            line(hip, vec2(6.0, 21.0), 1);
            line(hip, vec2(-5.0, 18.0), 2);
        }
        // Body
        line(neck, hip, 3);
        // Arms
        if p.grounded {
            line(shoulder, vec2(-7.0 - swing * 7.0, 9.0), 4);
            line(shoulder, vec2(7.0 + swing * 7.0, 9.0), 5);
        } else {
            line(shoulder, vec2(-11.0, -14.0), 4);
            line(shoulder, vec2(11.0, -14.0), 5);
        }
        // Head
        let h = to_world(head);
        draw_circle(h.x, h.y, 8.0, PAPER);
        pencil_circle(h, 8.0, 2.6, INK, 0x4EAD);
        let eye = to_world(vec2(3.5, -16.0));
        draw_circle(eye.x, eye.y, 1.6, INK);
    }
```

- [ ] **Step 5: Build, test, look**

Run: `cargo test && ./build_web.sh && /tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/world"`; view `world/4-play-start.png`, `world/5-play-moved.png` and a few `world/6-play-*.png`.
Expected:
- Paper is near-white with faint graph lines; the world has no dark areas.
- Platforms: double pencil outline + diagonal hatching; bumps darker and cross-hatched; floor and ceiling densely cross-hatched with a firm inner edge and no seams.
- Coins are pencil circles; the stickman is a dark pencil figure with a white head.
- The HUD may still be in colour (Task 5).

If something reads poorly, the only knobs are the spacing/width/alpha numbers in the code above (hatch spacing 5/7/9, widths 1.2–2.6, `faded` alphas); change them and re-shoot.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/render.rs
git commit -m "Draw the world in pencil on paper"
```

---

### Task 5: The HUD in pencil

**Files:**
- Modify: `src/render.rs` — `draw_hud`; add `paper_card`.

**Interfaces:**
- Produces (private in `render.rs`): `paper_card(r: Rect, seed: u64)`.

- [ ] **Step 1: The failing check**

Crop check: `node /tmp/gravity-fs-test/grey-check.mjs /tmp/gravity-fs-test/world/6-play-6.png`
Expected: `COLOUR` (the HUD's green/yellow/red/gold).

- [ ] **Step 2: Add `paper_card`** (top level in `render.rs`, after `draw_paper_grain`):

```rust
/// A sheet of paper with a sketched outline, for HUD boxes and menu cards.
fn paper_card(r: Rect, seed: u64) {
    draw_rectangle(r.x, r.y, r.w, r.h, faded(PAPER, 0.92));
    pencil_rect(r, 2.0, INK, seed);
}
```

- [ ] **Step 3: Replace `draw_hud`**

```rust
    fn draw_hud(&self) {
        let (sw, sh) = (screen_width(), screen_height());
        let t = get_time() as f32;

        // Health: a pencil box hatched up to the current health.
        let hp = (self.player.health / MAX_HEALTH).clamp(0.0, 1.0);
        draw_label("HEALTH", 20.0, 30.0, 24.0, INK);
        let bar = Rect::new(20.0, 38.0, 220.0, 20.0);
        draw_rectangle(bar.x, bar.y, bar.w, bar.h, PAPER);
        if hp > 0.0 {
            cross_hatch_rect(Rect::new(bar.x, bar.y, bar.w * hp, bar.h), 4.0, 1.3, INK, 0x4EA1);
        }
        pencil_rect(bar, 2.0, INK, 0x4EA2);

        // Distance, and the world's seed code (the card widens for long codes).
        let world_line = format!("world {}  deaths {}", self.world.code, self.deaths);
        let box_w = (measure(&world_line, 16.0).width + 20.0).max(222.0);
        let card = Rect::new(sw - box_w - 8.0, 8.0, box_w, 80.0);
        paper_card(card, 0xD157);
        draw_label(&format!("{:.0} m", self.distance()), card.x + 10.0, 38.0, 36.0, INK);
        draw_label(&format!("BEST {:.0} m", self.best / UNITS_PER_METRE), card.x + 10.0, 62.0, 22.0, GRAPHITE);
        draw_label(&world_line, card.x + 10.0, 80.0, 16.0, SHADE);

        if matches!(self.state, State::Title | State::Shop | State::EnterSeed) {
            return;
        }

        // Flip countdown. Under 3 seconds the number and arrow shake.
        let cx = sw / 2.0;
        let urgent = self.flip_timer < 3.0;
        let jolt = if urgent { vec2((t * 41.0).sin(), (t * 37.0).cos()) * 2.0 } else { Vec2::ZERO };
        paper_card(Rect::new(cx - 150.0, 8.0, 300.0, 92.0), 0xF11B);
        text_centered("NEXT FLIP IN", cx, 30.0, 22.0, GRAPHITE);
        text_centered(&format!("{:.1}", self.flip_timer.max(0.0)), cx - 30.0 + jolt.x, 80.0 + jolt.y, 60.0, INK);
        pencil_arrow(vec2(cx + 70.0, 58.0) + jolt, self.next_gravity.vec(), 40.0, 3.0, INK, 0xA770);
        text_centered(self.next_gravity.name(), cx + 70.0, 96.0, 16.0, INK);

        // Current gravity.
        draw_label("GRAVITY", 20.0, 90.0, 20.0, GRAPHITE);
        pencil_arrow(vec2(120.0, 84.0), self.gravity.vec(), 28.0, 2.6, INK, 0x96A7);

        // Coins and active power-ups.
        let coin = vec2(32.0, 128.0);
        draw_circle(coin.x, coin.y, 11.0, PAPER);
        pencil_circle(coin, 11.0, 2.0, INK, 0xC014);
        pencil_circle(coin, 6.0, 1.2, GRAPHITE, 0xC015);
        draw_label(&format!("{}", self.coins), 52.0, 136.0, 28.0, INK);
        let mut y = 168.0;
        for p in &self.owned {
            let label = if *p == PowerUp::Spider && self.spider_cooldown > 0.0 {
                format!("{}  {:.1}s", p.name(), self.spider_cooldown)
            } else {
                p.name().to_owned()
            };
            draw_label(&label, 20.0, y, 20.0, INK);
            y += 22.0;
        }

        // Hatching builds up on the screen edge gravity is about to point at.
        if urgent {
            let strength = 0.25 + 0.35 * (0.5 + 0.5 * (t * 10.0).sin()) * (1.0 - self.flip_timer / 3.0 + 0.3);
            let th = 34.0;
            let edge = match self.next_gravity {
                Dir::Down => Rect::new(0.0, sh - th, sw, th),
                Dir::Up => Rect::new(0.0, 0.0, sw, th),
                Dir::Left => Rect::new(0.0, 0.0, th, sh),
                Dir::Right => Rect::new(sw - th, 0.0, th, sh),
            };
            cross_hatch_rect(edge, 6.0, 1.6, faded(GRAPHITE, strength.min(1.0)), 0xED6E);
        }

        let afk_in = AFK_TIME - self.idle;
        let afk_banner = |text: &str, color: Color| {
            let width = measure(text, 28.0).width;
            paper_card(Rect::new(sw / 2.0 - width / 2.0 - 16.0, 110.0, width + 32.0, 42.0), 0xAF0B);
            text_centered(text, sw / 2.0, 140.0, 28.0, color);
        };
        if afk_in <= 0.0 {
            let pulse = 0.6 + 0.4 * (t * 6.0).sin();
            afk_banner("AFK MONSTERS!  Press anything to scare them off!", faded(INK, pulse));
        } else if afk_in <= AFK_WARNING {
            afk_banner(&format!("Are you there? AFK monsters in {:.0}...", afk_in.ceil()), INK);
        }

        // A gravity flip smudges the page (a white flash wouldn't show on paper).
        if self.flip_flash > 0.0 {
            draw_rectangle(0.0, 0.0, sw, sh, faded(SHADE, 0.3 * self.flip_flash));
            text_centered(
                &format!("GRAVITY: {}", self.gravity.name()),
                sw / 2.0,
                sh / 2.0 - 120.0,
                56.0,
                faded(INK, self.flip_flash),
            );
        }

        let hint = if self.gravity.is_vertical() {
            "A/D or Left/Right: move   Space: jump / double jump   R: restart"
        } else {
            "W/S or Up/Down: move   Space: jump / double jump   R: restart"
        };
        text_centered(hint, sw / 2.0, sh - 16.0, 20.0, SHADE);
    }
```

- [ ] **Step 4: Remove the old arrow**

Delete `fn draw_arrow` from `render.rs` (nothing uses it now). Run `grep -n draw_arrow src/*.rs` — expect no matches.

- [ ] **Step 5: Build, test, look, grey-check**

Run: `cargo test && ./build_web.sh && /tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/hud"`, then `node /tmp/gravity-fs-test/grey-check.mjs /tmp/gravity-fs-test/hud/4-play-start.png /tmp/gravity-fs-test/hud/5-play-moved.png /tmp/gravity-fs-test/hud/6-play-*.png`.
Expected: all `GREY`. View `hud/6-play-*.png`: at least one shows the urgent countdown with hatching on one screen edge, and one shows the grey smudge with "GRAVITY: X".

- [ ] **Step 6: Commit**

```bash
git add src/render.rs
git commit -m "Draw the HUD in pencil"
```

---

### Task 6: Menu screens in pencil

**Files:**
- Modify: `src/render.rs` — `draw` (death screen call), `draw_title`, `draw_shop`, `draw_seed_entry`, `draw_overlay`; add `wash`.
- Modify: `src/main.rs` — delete `PowerUp::color`.

**Interfaces:**
- Produces: `draw_overlay(&self, title: &str, lines: &[&str])` (the colour parameter is gone).

- [ ] **Step 1: The failing check**

Run: `node /tmp/gravity-fs-test/grey-check.mjs /tmp/gravity-fs-test/hud/1-title.png /tmp/gravity-fs-test/hud/2-seed-box.png /tmp/gravity-fs-test/hud/3-shop.png`
Expected: `COLOUR` for all three.

- [ ] **Step 2: Replace the screens**

Add to `impl Game` in `render.rs`:

```rust
    /// Washes the world out so a menu can sit on top of it.
    fn wash(&self) {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), faded(PAPER, 0.85));
    }
```

In `draw`, replace the `State::Dead => self.draw_overlay(` call's first two arguments `"YOU DIED",` and `Color::new(1.0, 0.25, 0.3, 1.0),` with just `"YOU DIED",`.

Replace `draw_title`, `draw_shop`, `draw_seed_entry` and `draw_overlay` with:

```rust
    fn draw_title(&self) {
        let (w, h) = (screen_width(), screen_height());
        self.wash();
        let card_w = (w - 40.0).min(1000.0);
        paper_card(Rect::new(w / 2.0 - card_w / 2.0, h / 2.0 - 190.0, card_w, 470.0), 0x7171);
        let t = get_time() as f32;
        let wobble = (t * 1.5).sin() * 0.08;
        let title = "GRAVITY";
        let size = 110.0;
        let dims = measure(title, size);
        with_font(|font| {
            draw_text_ex(
                title,
                w / 2.0 - dims.width / 2.0,
                h / 2.0 - 90.0,
                TextParams { font: Some(font), font_size: font_size(size), rotation: wobble, color: INK, ..Default::default() },
            );
        });
        let lines = [
            "Go right as far as you can - the obby never ends.",
            "Gravity changes direction when the countdown hits zero.",
            "Land too hard and you take fall damage - lose it all and you die.",
            &format!("Don't go AFK for {:.0} seconds... the AFK monsters are watching.", AFK_TIME),
            "Grab coins and spend them on power-ups like SPIDER - each lasts one round.",
            "",
            "Move: A/D (or W/S when gravity is sideways)    Jump: Space (again in mid-air to double jump)",
            "",
            "S - type a seed to play the same world as a friend",
            &format!("Press SPACE or click to start      B - shop ({} coins)", self.coins),
        ];
        for (i, l) in lines.iter().enumerate() {
            let c = if i == lines.len() - 1 { faded(INK, 0.6 + 0.4 * (t * 4.0).sin().abs()) } else { GRAPHITE };
            text_centered(l, w / 2.0, h / 2.0 - 30.0 + i as f32 * 30.0, 26.0, c);
        }
    }

    fn draw_shop(&self) {
        let (w, h) = (screen_width(), screen_height());
        self.wash();
        text_centered("POWER-UP SHOP", w / 2.0, h / 2.0 - 190.0, 64.0, INK);
        text_centered(
            &format!("Coins: {}      (power-ups last one round)", self.coins),
            w / 2.0,
            h / 2.0 - 150.0,
            28.0,
            GRAPHITE,
        );

        let mouse: Vec2 = mouse_position().into();
        for (i, (p, r)) in Self::shop_cards().iter().enumerate() {
            let owned = self.has(*p);
            let affordable = self.coins >= p.cost();
            let seed = 0x5409 + i as u64;
            paper_card(*r, seed);
            if owned {
                hatch_rect(*r, 12.0, PI / 4.0, 1.0, faded(SHADE, 0.5), seed);
            }
            if r.contains(mouse) {
                // Hovered: a second, looser outline.
                pencil_rect(Rect::new(r.x - 4.0, r.y - 4.0, r.w + 8.0, r.h + 8.0), 1.6, GRAPHITE, seed ^ 0xF0);
            }
            let cx = r.x + r.w / 2.0;
            draw_label(&format!("[{}]", i + 1), r.x + 12.0, r.y + 28.0, 22.0, SHADE);
            let icon = vec2(cx, r.y + 50.0);
            match p {
                PowerUp::Spider => doodle_spider(icon, 22.0, INK, seed),
                PowerUp::Magnet => doodle_magnet(icon, 22.0, INK, seed),
                PowerUp::ToughBones => doodle_bone(icon, 22.0, INK, seed),
            }
            text_centered(p.name(), cx, r.y + 104.0, 34.0, INK);
            // Word-wrap the description.
            let mut line = String::new();
            let mut ly = r.y + 134.0;
            for word in p.description().split(' ') {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if measure(&candidate, 20.0).width > r.w - 30.0 {
                    text_centered(&line, cx, ly, 20.0, GRAPHITE);
                    ly += 24.0;
                    line = word.to_owned();
                } else {
                    line = candidate;
                }
            }
            text_centered(&line, cx, ly, 20.0, GRAPHITE);
            let (label, color) = if owned {
                ("READY FOR NEXT ROUND".to_owned(), INK)
            } else if affordable {
                (format!("BUY - {} coins", p.cost()), INK)
            } else {
                (format!("{} coins", p.cost()), SHADE)
            };
            text_centered(&label, cx, r.y + r.h - 25.0, 26.0, color);
        }

        let (msg, time) = &self.shop_message;
        if *time > 0.0 {
            text_centered(msg, w / 2.0, h / 2.0 + 175.0, 28.0, faded(INK, time.min(1.0)));
        }
        text_centered("1/2/3 or click to buy    Esc/B/Space - back", w / 2.0, h / 2.0 + 215.0, 22.0, SHADE);
        if cfg!(debug_assertions) {
            text_centered("debug build: press C for +50 coins", w / 2.0, h / 2.0 + 245.0, 20.0, SHADE);
        }
    }

    fn draw_seed_entry(&self) {
        let (w, h) = (screen_width(), screen_height());
        self.wash();
        text_centered("ENTER A SEED", w / 2.0, h / 2.0 - 120.0, 64.0, INK);
        let bx = Rect::new(w / 2.0 - 280.0, h / 2.0 - 75.0, 560.0, 90.0);
        paper_card(bx, 0x5EED);
        pencil_rect(Rect::new(bx.x - 5.0, bx.y - 5.0, bx.w + 10.0, bx.h + 10.0), 1.4, GRAPHITE, 0x5EEE);
        // The code so far, centred, with a blinking pencil cursor after it.
        let width = measure(&self.seed_input, 56.0).width;
        let x = w / 2.0 - width / 2.0;
        draw_label(&self.seed_input, x, bx.y + 65.0, 56.0, INK);
        if get_time() % 1.0 < 0.5 {
            pencil_line(vec2(x + width + 8.0, bx.y + 70.0), vec2(x + width + 34.0, bx.y + 70.0), 3.0, INK, 0xC0A5);
        }
        text_centered(
            &format!("Letters and numbers, up to {}. Leave it blank for a random world.", seed::MAX_LEN),
            w / 2.0,
            bx.y + bx.h + 45.0,
            24.0,
            INK,
        );
        text_centered("Enter - play    Backspace - delete    Esc - back", w / 2.0, bx.y + bx.h + 80.0, 22.0, SHADE);
    }

    fn draw_overlay(&self, title: &str, lines: &[&str]) {
        let (w, h) = (screen_width(), screen_height());
        self.wash();
        let card_w = (w - 40.0).min(900.0);
        paper_card(Rect::new(w / 2.0 - card_w / 2.0, h / 2.0 - 120.0, card_w, 150.0 + lines.len() as f32 * 30.0), 0xDEAD);
        text_centered(title, w / 2.0, h / 2.0 - 40.0, 80.0, INK);
        // A scribbled underline.
        let tw = measure(title, 80.0).width;
        pencil_line(vec2(w / 2.0 - tw / 2.0, h / 2.0 - 22.0), vec2(w / 2.0 + tw / 2.0, h / 2.0 - 26.0), 3.0, INK, 0x0D1E);
        pencil_line(vec2(w / 2.0 - tw / 2.0 + 10.0, h / 2.0 - 16.0), vec2(w / 2.0 + tw / 2.0 - 6.0, h / 2.0 - 18.0), 2.0, GRAPHITE, 0x0D1F);
        for (i, l) in lines.iter().enumerate() {
            text_centered(l, w / 2.0, h / 2.0 + 10.0 + i as f32 * 30.0, 28.0, INK);
        }
    }
```

- [ ] **Step 3: Drop the power-up colours**

In `src/main.rs`, delete the whole `fn color(self) -> Color { ... }` method from `impl PowerUp`. Run `grep -n '\.color()' src/*.rs` — expect no matches.

- [ ] **Step 4: Build, test, look, grey-check everything**

Run: `cargo test && cargo build --release && ./build_web.sh && /tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/screens"`, then `node /tmp/gravity-fs-test/grey-check.mjs /tmp/gravity-fs-test/screens/*.png`.
Expected: `17 passed`, no warnings, every screenshot `GREY`. View `screens/1-title.png`, `2-seed-box.png`, `3-shop.png`: paper cards with sketched outlines, handwritten text, spider/magnet/bone doodles on the shop cards, nothing overflowing.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/render.rs
git commit -m "Draw the title, shop, seed box and death screen in pencil"
```

---

### Task 7: Final verification

**Files:** none committed (harness files in `/tmp/gravity-fs-test`).

- [ ] **Step 1: Regression checks**

Run: `cargo test` (expect `17 passed`), `cargo build --release` (the native `GRAVITY_SHOT` hook still compiles; running it opens a game window, so leave that to the user), `/tmp/gravity-fs-test/run.sh` (fullscreen: `all checks passed`), `/tmp/gravity-fs-test/run.sh seed-check.mjs` and view `seed-1-typed.png` (the box shows `BANANA` in handwriting) and `seed-2-hud-banana.png` (`world BANANA`).

- [ ] **Step 2: 2× screenshots and atlas**

Run: `/tmp/gravity-fs-test/run.sh "shots.mjs /tmp/gravity-fs-test/final-2x 2"`; grey-check them (all `GREY`); view `final-2x/1-title.png` (crisp handwriting, no black boxes). Run the 1×/2× atlas probes from Task 3 Step 6 — largest at most `4096x4096`.

- [ ] **Step 3: Death screen and AFK monsters**

Create `/tmp/gravity-fs-test/afk.mjs`:

```js
// Idles in a fixed world until the AFK monsters come (60 s of game time), shooting every 5 s.
import { chromium } from "playwright-core";
const CHROME = `${process.env.HOME}/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;
const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1280, height: 760 } });
await page.goto("http://127.0.0.1:8792/index.html");
await page.waitForFunction(() => typeof wasm_exports !== "undefined" && wasm_exports, null, { timeout: 20000 });
await page.evaluate(() => canvas.focus());
await page.keyboard.press("KeyS");
await page.keyboard.type("pencil");
await page.keyboard.press("Enter");
for (let s = 0; s < 22; s++) {
    await page.waitForTimeout(5000);
    await page.screenshot({ path: `afk-${String(s).padStart(2, "0")}.png` });
}
await browser.close();
```

Run: `/tmp/gravity-fs-test/run.sh afk.mjs` (takes ~2 minutes); view the last few `afk-*.png`.
Expected: scribbled ghost monsters with white eyes and a paper "AFK" banner; once health runs out, the death card with "YOU DIED" and its underline. Grey-check them.

- [ ] **Step 4: Rough performance check**

Build the pre-pencil commit (the commit before Task 1) for comparison and measure animation frames per second in both:

```bash
BASE_COMMIT=$(git log --format=%h --grep='Add design spec for the pencil-on-paper look' -1)
rm -rf /tmp/gravity-prepencil && mkdir -p /tmp/gravity-prepencil && git archive "$BASE_COMMIT" | tar -x -C /tmp/gravity-prepencil
CARGO_TARGET_DIR=/tmp/gravity-prepencil-target cargo build --release --target wasm32-unknown-unknown --manifest-path /tmp/gravity-prepencil/Cargo.toml
mkdir -p /tmp/gravity-prepencil-web && cp web/index.html web/mq_js_bundle.js /tmp/gravity-prepencil-web/ && cp /tmp/gravity-prepencil-target/wasm32-unknown-unknown/release/gravity.wasm /tmp/gravity-prepencil-web/
```

Create `/tmp/gravity-fs-test/fps.mjs`:

```js
// Frames per second while playing, at BASE (headless, software rendering: compare, don't trust absolutes).
import { chromium } from "playwright-core";
const CHROME = `${process.env.HOME}/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;
const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1280, height: 760 } });
await page.goto(`${process.env.BASE}/index.html`);
await page.waitForFunction(() => typeof wasm_exports !== "undefined" && wasm_exports, null, { timeout: 20000 });
await page.evaluate(() => canvas.focus());
await page.keyboard.press("Space");
await page.keyboard.down("KeyD");
await page.waitForTimeout(1000);
const fps = await page.evaluate(() => new Promise((done) => {
    let frames = 0;
    const t0 = performance.now();
    const tick = () => { frames++; performance.now() - t0 < 4000 ? requestAnimationFrame(tick) : done(frames / 4); };
    requestAnimationFrame(tick);
}));
console.log(`${process.argv[2]}: ${fps.toFixed(1)} fps`);
await browser.close();
```

Run:

```bash
cd /tmp/gravity-fs-test
python3 -m http.server 8792 --bind 127.0.0.1 --directory /Users/virenvijaymane/Rust_Projects/gravity/web >/dev/null 2>&1 & A=$!
python3 -m http.server 8793 --bind 127.0.0.1 --directory /tmp/gravity-prepencil-web >/dev/null 2>&1 & B=$!
curl -s -o /dev/null --retry 10 --retry-connrefused --retry-delay 1 http://127.0.0.1:8792/index.html
curl -s -o /dev/null --retry 10 --retry-connrefused --retry-delay 1 http://127.0.0.1:8793/index.html
BASE=http://127.0.0.1:8793 node fps.mjs before; BASE=http://127.0.0.1:8792 node fps.mjs pencil; kill $A $B
```

Expected: `pencil` within ~25% of `before`. If much slower, raise the hatch spacings (Task 4) and re-measure.

- [ ] **Step 5: Show the user**

Share `screens/1-title.png`, `screens/3-shop.png`, a gameplay shot with the urgent countdown, and the death card. Nothing is pushed or deployed without the user's say-so.
