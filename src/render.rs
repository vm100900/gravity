//! Everything that draws the game: the world, the HUD and the menu screens.

use super::*;
use macroquad::prelude::*;

fn draw_arrow(center: Vec2, dir: Dir, size: f32, color: Color) {
    let a = dir.angle();
    let p = |x: f32, y: f32| center + rotate(vec2(x, y) * size, a);
    // Shaft
    draw_triangle(p(-0.18, -0.5), p(0.18, -0.5), p(0.18, 0.1), color);
    draw_triangle(p(-0.18, -0.5), p(0.18, 0.1), p(-0.18, 0.1), color);
    // Head
    draw_triangle(p(-0.45, 0.05), p(0.45, 0.05), p(0.0, 0.55), color);
}

/// Every font size text is drawn at. All of their glyphs are rasterised up
/// front: if macroquad's glyph atlas has to grow mid-game it recreates its
/// texture, which can leave text rendering as solid black boxes.
const FONT_SIZES: [u16; 16] = [16, 20, 22, 24, 26, 28, 30, 32, 34, 36, 40, 56, 60, 64, 80, 110];

pub(crate) fn warm_font_cache() {
    let font = get_default_font();
    let chars: Vec<char> = (32u8..127).map(char::from).collect();
    for size in FONT_SIZES {
        font.populate_font_cache(&chars, size);
    }
}

/// Snap to the nearest pre-cached size so no new glyphs are ever needed.
fn font_size(size: f32) -> u16 {
    *FONT_SIZES.iter().min_by_key(|s| (**s as i32 - size.round() as i32).abs()).unwrap()
}

fn draw_label(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text, x, y, font_size(size) as f32, color);
}

fn text_centered(text: &str, x: f32, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, font_size(size), 1.0);
    draw_label(text, x - dims.width / 2.0, y, size, color);
}

impl Game {
    // -----------------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------------

    pub(crate) fn draw(&self) {
        clear_background(Color::from_rgba(12, 12, 24, 255));
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
                Color::new(1.0, 0.25, 0.3, 1.0),
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
        for s in &self.stars {
            let c = Color::new(0.5, 0.6, 1.0, 0.15 + 0.35 * s.depth);
            let tail = self.gravity.vec() * 10.0 * s.depth;
            let p = vec2(s.pos.x * w, s.pos.y * h);
            draw_line(p.x, p.y, p.x - tail.x, p.y - tail.y, 1.5, c);
        }
    }

    fn draw_world(&self) {
        let half_w = self.view_size().x / 2.0 + 100.0;
        let (left, right) = ((self.cam.x - half_w).max(0.0), self.cam.x + half_w);

        // Interior backdrop and faint grid.
        draw_rectangle(left, 0.0, right - left, WORLD_HEIGHT, Color::from_rgba(20, 22, 40, 255));
        let grid = Color::from_rgba(35, 38, 66, 255);
        let mut x = (left / 100.0).floor() * 100.0;
        while x < right {
            draw_line(x, 0.0, x, WORLD_HEIGHT, 1.0, grid);
            x += 100.0;
        }
        let mut y = 0.0;
        while y < WORLD_HEIGHT {
            draw_line(left, y, right, y, 1.0, grid);
            y += 100.0;
        }

        // Distance markers every 50 m, plus where your best run ended.
        let marker = 50.0 * UNITS_PER_METRE;
        let mut mx = (left / marker).ceil().max(1.0) * marker;
        while mx < right {
            draw_line(mx, 0.0, mx, WORLD_HEIGHT, 3.0, Color::new(0.5, 0.6, 1.0, 0.25));
            text_centered(&format!("{:.0} m", mx / UNITS_PER_METRE), mx, WORLD_HEIGHT / 2.0, 40.0, Color::new(0.6, 0.7, 1.0, 0.35));
            mx += marker;
        }
        let best = self.best_before_run;
        if best > 0.0 && best > left && best < right {
            let pulse = 0.5 + 0.5 * (get_time() as f32 * 3.0).sin();
            draw_line(best, 0.0, best, WORLD_HEIGHT, 4.0, Color::new(1.0, 0.85, 0.3, 0.35 + 0.3 * pulse));
            text_centered("BEST", best, WORLD_HEIGHT / 2.0 - 50.0, 36.0, Color::new(1.0, 0.85, 0.3, 0.8));
        }

        // Start marker.
        draw_label("START", 60.0, WORLD_HEIGHT - 70.0, 30.0, Color::new(1.0, 1.0, 1.0, 0.25));

        for s in &self.world.solids_between(left, right) {
            let r = s.rect;
            let (fill, edge) = match s.kind {
                Kind::Boundary => (Color::from_rgba(50, 52, 78, 255), Color::from_rgba(90, 94, 140, 255)),
                Kind::Bump => (Color::from_rgba(200, 120, 50, 255), Color::from_rgba(255, 180, 90, 255)),
                Kind::Platform if r.w > r.h => (Color::from_rgba(40, 150, 170, 255), Color::from_rgba(120, 230, 240, 255)),
                Kind::Platform => (Color::from_rgba(140, 70, 180, 255), Color::from_rgba(210, 150, 255, 255)),
                Kind::Web => continue,
            };
            draw_rectangle(r.x, r.y, r.w, r.h, fill);
            if s.kind == Kind::Boundary {
                // Only outline the inner face, so neighbouring chunks join seamlessly.
                if r.y >= WORLD_HEIGHT {
                    draw_line(r.x, r.y, r.x + r.w, r.y, 3.0, edge);
                } else if r.w > r.h {
                    draw_line(r.x, r.y + r.h, r.x + r.w, r.y + r.h, 3.0, edge);
                } else {
                    draw_line(r.x + r.w, r.y, r.x + r.w, r.y + r.h, 3.0, edge);
                }
            } else {
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, edge);
            }
        }

        for d in &self.dust {
            draw_circle(d.pos.x, d.pos.y, 3.0 * d.life * 2.0, Color::new(0.8, 0.8, 0.9, d.life * 1.6));
        }

        let t = get_time() as f32;
        for i in self.world.chunk_range(left, right) {
            for c in self.world.chunks[i].coins.iter().filter(|c| !c.taken) {
                // Spinning coin: squash its width over time.
                let spin = (t * 3.0 + c.home.x * 0.01).cos().abs().max(0.15);
                draw_circle(c.pos.x, c.pos.y, COIN_RADIUS * 1.6, Color::new(1.0, 0.8, 0.2, 0.12));
                draw_ellipse(c.pos.x, c.pos.y, COIN_RADIUS * spin, COIN_RADIUS, 0.0, Color::new(1.0, 0.78, 0.15, 1.0));
                draw_ellipse(c.pos.x, c.pos.y, COIN_RADIUS * spin * 0.6, COIN_RADIUS * 0.6, 0.0, Color::new(1.0, 0.93, 0.5, 1.0));
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
            let mut c = pp.color;
            c.a = pp.life.min(1.0);
            text_centered(&pp.text, pp.pos.x, pp.pos.y, 36.0, c);
        }
    }

    fn draw_web(&self, r: &Rect, life: f32) {
        let a = life.min(1.0);
        let color = Color::new(0.92, 0.92, 1.0, 0.85 * a);
        let faint = Color::new(0.92, 0.92, 1.0, 0.45 * a);
        let horizontal = r.w > r.h;
        // Anchor strands run along the web's length, sagging lines connect them.
        let (len, start, dir, across) = if horizontal {
            (r.w, vec2(r.x, r.y + r.h / 2.0), vec2(1.0, 0.0), vec2(0.0, 1.0))
        } else {
            (r.h, vec2(r.x + r.w / 2.0, r.y), vec2(0.0, 1.0), vec2(1.0, 0.0))
        };
        let end = start + dir * len;
        draw_line(start.x, start.y, end.x, end.y, 2.0, color);
        let center = start + dir * len / 2.0;
        let n = 8;
        for i in 0..=n {
            let p = start + dir * len * i as f32 / n as f32;
            let off = across * (if i % 2 == 0 { -1.0 } else { 1.0 }) * r.w.min(r.h) * 1.2;
            draw_line(center.x, center.y, p.x + off.x, p.y + off.y, 1.2, faint);
            if i < n {
                let q = start + dir * len * (i + 1) as f32 / n as f32;
                draw_line(p.x + off.x, p.y + off.y, q.x - off.x, q.y - off.y, 1.2, faint);
            }
        }
        // Little spider hanging at the middle.
        draw_circle(center.x, center.y, 5.0, Color::new(0.1, 0.1, 0.12, a));
        for k in 0..4 {
            let ang = k as f32 * 0.5 - 0.75;
            for side in [-1.0f32, 1.0] {
                let leg = rotate(vec2(side * 9.0, 0.0), ang * side);
                draw_line(center.x, center.y, center.x + leg.x, center.y + leg.y, 1.2, Color::new(0.1, 0.1, 0.12, a));
            }
        }
    }

    fn draw_monster(&self, m: &Monster) {
        let t = get_time() as f32 + m.phase;
        let a = m.alpha.clamp(0.0, 1.0);
        let r = MONSTER_RADIUS;
        let c = m.pos + vec2(0.0, (t * 3.0).sin() * 4.0);
        let body = Color::new(0.35, 0.1, 0.45, 0.9 * a);
        let glow = Color::new(0.8, 0.2, 1.0, 0.18 * a);

        draw_circle(c.x, c.y, r * 1.6, glow);
        draw_circle(c.x, c.y, r, body);
        draw_rectangle(c.x - r, c.y, r * 2.0, r * 0.8, body);
        // Wavy ghost skirt.
        for i in 0..4 {
            let x = c.x - r + i as f32 * r * 0.5;
            let dip = r * (1.1 + 0.25 * (t * 8.0 + i as f32).sin());
            draw_triangle(vec2(x, c.y + r * 0.8), vec2(x + r * 0.5, c.y + r * 0.8), vec2(x + r * 0.25, c.y + dip), body);
        }
        // Angry red eyes that track you.
        let look = (self.player.pos - c).normalize_or_zero() * 3.0;
        let eye = Color::new(1.0, 0.15, 0.2, a);
        for side in [-1.0, 1.0] {
            let e = c + vec2(side * r * 0.38, -r * 0.15);
            draw_circle(e.x, e.y, 5.0, Color::new(1.0, 0.9, 0.9, a));
            draw_circle(e.x + look.x, e.y + look.y, 3.0, eye);
            draw_line(e.x - side * 7.0, e.y - 10.0, e.x + side * 5.0, e.y - 6.0, 2.5, eye);
        }
        text_centered("AFK", c.x, c.y + r * 0.75, 16.0, Color::new(1.0, 0.8, 1.0, 0.8 * a));
    }

    fn draw_stickman(&self) {
        let p = &self.player;
        if p.invulnerable > 0.0 && (get_time() * 20.0).sin() > 0.0 {
            return;
        }
        // Somersault forward during a double jump.
        let spin = (1.0 - p.flip_spin / DOUBLE_JUMP_SPIN_TIME) * std::f32::consts::TAU;
        let a = p.draw_angle + if p.flip_spin > 0.0 { spin * p.facing } else { 0.0 };
        let to_world = |v: Vec2| p.pos + rotate(vec2(v.x * p.facing, v.y), a);
        let hurt = p.hurt_flash;
        let low_hp = p.health < 30.0 && (get_time() * 8.0).sin() > 0.0;
        let color = if hurt > 0.0 || low_hp {
            Color::new(1.0, 1.0 - 0.7 * hurt.max(0.5), 1.0 - 0.7 * hurt.max(0.5), 1.0)
        } else {
            WHITE
        };
        let line = |a: Vec2, b: Vec2| {
            let (a, b) = (to_world(a), to_world(b));
            draw_line(a.x, a.y, b.x, b.y, 3.5, color);
        };

        let moving = p.grounded && p.vel.length() > 30.0;
        let swing = if moving { p.walk_phase.sin() } else { 0.0 };
        let head = vec2(0.0, -15.0);
        let neck = vec2(0.0, -7.0);
        let hip = vec2(0.0, 7.0);
        let shoulder = vec2(0.0, -3.0);

        // Legs
        if p.grounded {
            line(hip, vec2(6.0 + swing * 8.0, 24.0));
            line(hip, vec2(-6.0 - swing * 8.0, 24.0));
        } else {
            line(hip, vec2(6.0, 21.0));
            line(hip, vec2(-5.0, 18.0));
        }
        // Body
        line(neck, hip);
        // Arms
        if p.grounded {
            line(shoulder, vec2(-7.0 - swing * 7.0, 9.0));
            line(shoulder, vec2(7.0 + swing * 7.0, 9.0));
        } else {
            line(shoulder, vec2(-11.0, -14.0));
            line(shoulder, vec2(11.0, -14.0));
        }
        // Head
        let h = to_world(head);
        draw_circle(h.x, h.y, 8.0, Color::from_rgba(20, 22, 40, 255));
        draw_circle_lines(h.x, h.y, 8.0, 3.0, color);
        let eye = to_world(vec2(3.5, -16.0));
        draw_circle(eye.x, eye.y, 1.6, color);
    }

    fn draw_hud(&self) {
        let (sw, _sh) = (screen_width(), screen_height());
        let t = get_time() as f32;

        // Health bar.
        let hp = self.player.health / MAX_HEALTH;
        draw_label("HEALTH", 20.0, 30.0, 24.0, WHITE);
        draw_rectangle(20.0, 38.0, 220.0, 20.0, Color::from_rgba(40, 40, 60, 255));
        let hp_color = if hp > 0.5 {
            Color::new(0.3, 0.9, 0.4, 1.0)
        } else if hp > 0.25 {
            Color::new(1.0, 0.8, 0.2, 1.0)
        } else {
            Color::new(1.0, 0.25, 0.25, 1.0)
        };
        draw_rectangle(20.0, 38.0, 220.0 * hp, 20.0, hp_color);
        draw_rectangle_lines(20.0, 38.0, 220.0, 20.0, 2.0, WHITE);

        // Distance, and the world's seed code (the box widens for long codes).
        let world_line = format!("world {}  deaths {}", self.world.code, self.deaths);
        let box_w = (measure_text(&world_line, None, font_size(16.0), 1.0).width + 20.0).max(222.0);
        let x = sw - box_w - 8.0;
        draw_rectangle(x, 8.0, box_w, 80.0, Color::new(0.0, 0.0, 0.0, 0.45));
        draw_label(&format!("{:.0} m", self.distance()), x + 10.0, 38.0, 36.0, WHITE);
        draw_label(&format!("BEST {:.0} m", self.best / UNITS_PER_METRE), x + 10.0, 62.0, 22.0, Color::new(1.0, 0.85, 0.3, 1.0));
        draw_label(&world_line, x + 10.0, 80.0, 16.0, LIGHTGRAY);

        if matches!(self.state, State::Title | State::Shop | State::EnterSeed) {
            return;
        }

        // Flip countdown.
        let cx = sw / 2.0;
        let urgent = self.flip_timer < 3.0;
        let blink = urgent && (t * 8.0).sin() > 0.0;
        let color = if urgent {
            if blink { Color::new(1.0, 0.3, 0.3, 1.0) } else { Color::new(1.0, 0.7, 0.3, 1.0) }
        } else {
            WHITE
        };
        draw_rectangle(cx - 150.0, 8.0, 300.0, 92.0, Color::new(0.0, 0.0, 0.0, 0.45));
        text_centered("NEXT FLIP IN", cx, 30.0, 22.0, Color::new(1.0, 1.0, 1.0, 0.7));
        text_centered(&format!("{:.1}", self.flip_timer.max(0.0)), cx - 30.0, 80.0, 60.0, color);
        draw_arrow(vec2(cx + 70.0, 62.0), self.next_gravity, 44.0, color);
        text_centered(self.next_gravity.name(), cx + 70.0, 96.0, 16.0, color);

        // Current gravity indicator.
        draw_label("GRAVITY", 20.0, 90.0, 20.0, Color::new(1.0, 1.0, 1.0, 0.7));
        draw_arrow(vec2(120.0, 84.0), self.gravity, 30.0, WHITE);

        // Coins and active power-ups.
        draw_circle(32.0, 128.0, 11.0, Color::new(1.0, 0.78, 0.15, 1.0));
        draw_circle(32.0, 128.0, 6.0, Color::new(1.0, 0.93, 0.5, 1.0));
        draw_label(&format!("{}", self.coins), 52.0, 136.0, 28.0, Color::new(1.0, 0.85, 0.3, 1.0));
        let mut y = 168.0;
        for p in &self.owned {
            let label = if *p == PowerUp::Spider && self.spider_cooldown > 0.0 {
                format!("{}  {:.1}s", p.name(), self.spider_cooldown)
            } else {
                p.name().to_owned()
            };
            draw_label(&label, 20.0, y, 20.0, p.color());
            y += 22.0;
        }

        // Screen-edge warning glow toward where gravity is about to point.
        if urgent {
            let a = 0.15 + 0.25 * (0.5 + 0.5 * (t * 10.0).sin()) * (1.0 - self.flip_timer / 3.0 + 0.3);
            let c = Color::new(1.0, 0.3, 0.2, a);
            let (w, h) = (screen_width(), screen_height());
            let th = 30.0;
            match self.next_gravity {
                Dir::Down => draw_rectangle(0.0, h - th, w, th, c),
                Dir::Up => draw_rectangle(0.0, 0.0, w, th, c),
                Dir::Left => draw_rectangle(0.0, 0.0, th, h, c),
                Dir::Right => draw_rectangle(w - th, 0.0, th, h, c),
            }
        }

        let afk_in = AFK_TIME - self.idle;
        let w = screen_width();
        let afk_banner = |text: &str, color: Color| {
            let dims = measure_text(text, None, font_size(28.0), 1.0);
            draw_rectangle(w / 2.0 - dims.width / 2.0 - 16.0, 110.0, dims.width + 32.0, 42.0, Color::new(0.1, 0.0, 0.15, 0.75));
            text_centered(text, w / 2.0, 140.0, 28.0, color);
        };
        if afk_in <= 0.0 {
            let pulse = 0.6 + 0.4 * (t * 6.0).sin();
            afk_banner("AFK MONSTERS!  Press anything to scare them off!", Color::new(0.95, 0.5, 1.0, pulse));
        } else if afk_in <= AFK_WARNING {
            afk_banner(&format!("Are you there? AFK monsters in {:.0}...", afk_in.ceil()), Color::new(0.9, 0.7, 1.0, 1.0));
        }

        if self.flip_flash > 0.0 {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 1.0, 1.0, 0.25 * self.flip_flash));
            text_centered(
                &format!("GRAVITY: {}", self.gravity.name()),
                w / 2.0,
                h / 2.0 - 120.0,
                56.0,
                Color::new(1.0, 1.0, 1.0, self.flip_flash),
            );
        }

        let hint = if self.gravity.is_vertical() {
            "A/D or Left/Right: move   Space: jump / double jump   R: restart"
        } else {
            "W/S or Up/Down: move   Space: jump / double jump   R: restart"
        };
        text_centered(hint, sw / 2.0, screen_height() - 16.0, 20.0, Color::new(1.0, 1.0, 1.0, 0.5));
    }

    fn draw_title(&self) {
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.05, 0.75));
        let t = get_time() as f32;
        let wobble = (t * 1.5).sin() * 0.08;
        let title = "GRAVITY";
        let size = 110.0;
        let dims = measure_text(title, None, font_size(size), 1.0);
        draw_text_ex(
            title,
            w / 2.0 - dims.width / 2.0,
            h / 2.0 - 90.0,
            TextParams { font_size: font_size(size), rotation: wobble, color: WHITE, ..Default::default() },
        );
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
            let c = if i == lines.len() - 1 { Color::new(1.0, 0.9, 0.4, 0.6 + 0.4 * (t * 4.0).sin().abs()) } else { WHITE };
            text_centered(l, w / 2.0, h / 2.0 - 30.0 + i as f32 * 30.0, 26.0, c);
        }
    }

    fn draw_shop(&self) {
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.02, 0.08, 0.92));
        text_centered("POWER-UP SHOP", w / 2.0, h / 2.0 - 190.0, 64.0, WHITE);
        text_centered(
            &format!("Coins: {}      (power-ups last one round)", self.coins),
            w / 2.0,
            h / 2.0 - 150.0,
            28.0,
            Color::new(1.0, 0.85, 0.3, 1.0),
        );

        let mouse: Vec2 = mouse_position().into();
        for (i, (p, r)) in Self::shop_cards().iter().enumerate() {
            let owned = self.has(*p);
            let affordable = self.coins >= p.cost();
            let hover = r.contains(mouse);
            let bg = if owned {
                Color::new(0.1, 0.3, 0.15, 0.9)
            } else if hover {
                Color::new(0.2, 0.2, 0.35, 0.95)
            } else {
                Color::new(0.12, 0.12, 0.22, 0.9)
            };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, p.color());
            let cx = r.x + r.w / 2.0;
            text_centered(&format!("[{}]", i + 1), cx, r.y + 30.0, 22.0, GRAY);
            text_centered(p.name(), cx, r.y + 70.0, 34.0, p.color());
            // Word-wrap the description.
            let mut line = String::new();
            let mut ly = r.y + 110.0;
            for word in p.description().split(' ') {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if measure_text(&candidate, None, font_size(20.0), 1.0).width > r.w - 30.0 {
                    text_centered(&line, cx, ly, 20.0, WHITE);
                    ly += 24.0;
                    line = word.to_owned();
                } else {
                    line = candidate;
                }
            }
            text_centered(&line, cx, ly, 20.0, WHITE);
            let (label, color) = if owned {
                ("READY FOR NEXT ROUND".to_owned(), Color::new(0.4, 1.0, 0.5, 1.0))
            } else if affordable {
                (format!("BUY - {} coins", p.cost()), Color::new(1.0, 0.85, 0.3, 1.0))
            } else {
                (format!("{} coins", p.cost()), Color::new(0.6, 0.6, 0.6, 1.0))
            };
            text_centered(&label, cx, r.y + r.h - 25.0, 26.0, color);
        }

        let (msg, time) = &self.shop_message;
        if *time > 0.0 {
            text_centered(msg, w / 2.0, h / 2.0 + 175.0, 28.0, Color::new(1.0, 1.0, 1.0, time.min(1.0)));
        }
        text_centered("1/2/3 or click to buy    Esc/B/Space - back", w / 2.0, h / 2.0 + 215.0, 22.0, GRAY);
        if cfg!(debug_assertions) {
            text_centered("debug build: press C for +50 coins", w / 2.0, h / 2.0 + 245.0, 20.0, Color::new(1.0, 0.85, 0.3, 0.6));
        }
    }

    fn draw_seed_entry(&self) {
        let (w, h) = (screen_width(), screen_height());
        let gold = Color::new(1.0, 0.85, 0.3, 1.0);
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.02, 0.08, 0.92));
        text_centered("ENTER A SEED", w / 2.0, h / 2.0 - 120.0, 64.0, WHITE);
        let (bw, bh) = (560.0, 90.0);
        let (bx, by) = (w / 2.0 - bw / 2.0, h / 2.0 - 75.0);
        draw_rectangle(bx, by, bw, bh, Color::new(0.12, 0.12, 0.22, 0.95));
        draw_rectangle_lines(bx, by, bw, bh, 3.0, gold);
        // The code so far, centred, with a blinking cursor after it.
        let width = measure_text(&self.seed_input, None, font_size(56.0), 1.0).width;
        let x = w / 2.0 - width / 2.0;
        draw_label(&self.seed_input, x, by + 65.0, 56.0, WHITE);
        if get_time() % 1.0 < 0.5 {
            draw_label("_", x + width + 4.0, by + 65.0, 56.0, gold);
        }
        text_centered(
            &format!("Letters and numbers, up to {}. Leave it blank for a random world.", seed::MAX_LEN),
            w / 2.0,
            by + bh + 45.0,
            24.0,
            WHITE,
        );
        text_centered("Enter - play    Backspace - delete    Esc - back", w / 2.0, by + bh + 80.0, 22.0, GRAY);
    }

    fn draw_overlay(&self, title: &str, color: Color, lines: &[&str]) {
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.6));
        text_centered(title, w / 2.0, h / 2.0 - 40.0, 80.0, color);
        for (i, l) in lines.iter().enumerate() {
            text_centered(l, w / 2.0, h / 2.0 + 10.0 + i as f32 * 30.0, 28.0, WHITE);
        }
    }
}
