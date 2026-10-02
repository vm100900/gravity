//! Everything that draws the game: the world, the HUD and the menu screens.

use super::*;
use crate::sketch::*;
use macroquad::prelude::*;
use std::f32::consts::PI;

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

/// A sheet of paper with a sketched outline, for HUD boxes and menu cards.
fn paper_card(r: Rect, seed: u64) {
    draw_rectangle(r.x, r.y, r.w, r.h, faded(PAPER, 0.92));
    pencil_rect(r, 2.0, INK, seed);
}

impl Game {
    // -----------------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------------

    pub(crate) fn draw(&self) {
        clear_background(PAPER);
        self.draw_background();

        let view = self.view_size();
        let shake = vec2(rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0)) * self.shake;
        set_camera(&Camera2D {
            target: self.cam + shake,
            zoom: vec2(2.0 / view.x, 2.0 / view.y),
            ..Default::default()
        });
        self.draw_world();
        set_default_camera();

        self.draw_hud();
        match self.state {
            State::Title => self.draw_title(),
            State::Shop => self.draw_shop(),
            State::EnterSeed => self.draw_seed_entry(),
            State::Dead => self.draw_overlay(
                "YOU DIED",
                &[
                    self.death_cause,
                    &format!("You made it {:.0} m   (best {:.0} m)", self.distance(), self.best / UNITS_PER_METRE),
                    &format!("World {} - give this seed to a friend to race the same world", self.world.code),
                    "",
                    "R / Space - try this world again",
                    "N - brand new world",
                    "S - type a seed",
                    &format!("B - power-up shop   (+{} coins this run, {} total)", self.coins_this_run, self.coins),
                    &if self.used_up.is_empty() {
                        String::new()
                    } else {
                        let names: Vec<&str> = self.used_up.iter().map(|p| p.name()).collect();
                        format!("Used up: {}", names.join(", "))
                    },
                ],
            ),
            State::Playing => {}
        }
    }

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
        scribble_circle(c, r, 1.6, 1.8, body, seed);
        scribble_rect(Rect::new(c.x - r, c.y, r * 2.0, r * 0.8), 1.6, 1.8, body, seed ^ 1);
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
        draw_label(&world_line, card.x + 10.0, 80.0, 16.0, GRAPHITE);

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
            let strength = 0.5 + 0.5 * (0.5 + 0.5 * (t * 10.0).sin()) * (1.0 - self.flip_timer / 3.0 + 0.3);
            let th = 40.0;
            let edge = match self.next_gravity {
                Dir::Down => Rect::new(0.0, sh - th, sw, th),
                Dir::Up => Rect::new(0.0, 0.0, sw, th),
                Dir::Left => Rect::new(0.0, 0.0, th, sh),
                Dir::Right => Rect::new(sw - th, 0.0, th, sh),
            };
            cross_hatch_rect(edge, 5.0, 1.8, faded(GRAPHITE, strength.min(1.0)), 0xED6E);
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
        // On its own scrap of paper, so it stays readable over the floor's hatching.
        let hint_w = measure(hint, 20.0).width;
        paper_card(Rect::new(sw / 2.0 - hint_w / 2.0 - 12.0, sh - 36.0, hint_w + 24.0, 28.0), 0x41E7);
        text_centered(hint, sw / 2.0, sh - 16.0, 20.0, GRAPHITE);
    }

    /// Washes the world out so a menu can sit on top of it.
    fn wash(&self) {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), faded(PAPER, 0.85));
    }

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
                PowerUp::Spider => doodle_spider(icon, 30.0, INK, seed),
                PowerUp::Magnet => doodle_magnet(icon, 30.0, INK, seed),
                PowerUp::ToughBones => doodle_bone(icon, 30.0, INK, seed),
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
}
