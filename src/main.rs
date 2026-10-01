//! GRAVITY — a procedurally generated stickman obby where gravity keeps changing direction.

mod render;
mod seed;
mod sketch;

use macroquad::prelude::*;

// ---------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------

const GRAVITY: f32 = 1900.0;
const MAX_FALL: f32 = 1400.0;
const MOVE_SPEED: f32 = 330.0;
const GROUND_ACCEL: f32 = 3200.0;
const AIR_ACCEL: f32 = 1700.0;
const JUMP_SPEED: f32 = 800.0;
const DOUBLE_JUMP_SPEED: f32 = 720.0;
/// Extra jumps allowed before touching something again.
const AIR_JUMPS: u32 = 1;
const DOUBLE_JUMP_SPIN_TIME: f32 = 0.35;
const COYOTE_TIME: f32 = 0.1;
const JUMP_BUFFER: f32 = 0.12;

/// Landing faster than this hurts.
const SAFE_LANDING_SPEED: f32 = 900.0;
const DAMAGE_PER_SPEED: f32 = 0.11;
const MAX_HEALTH: f32 = 100.0;
const REGEN_PER_SEC: f32 = 4.0;
const REGEN_DELAY: f32 = 2.0;

/// Player hitbox: long along the gravity axis, thin across it.
const HALF_TALL: f32 = 24.0;
const HALF_WIDE: f32 = 10.0;

const WORLD_HEIGHT: f32 = 1100.0;
const BOUNDARY: f32 = 60.0;
/// Clear strip along the floor and the ceiling, so there is always a walkable route.
const LANE: f32 = 170.0;
const BUMP_H: f32 = 36.0;
const CELL_W: f32 = 280.0;
const CELL_H: f32 = 253.0;
const CELL_PAD: f32 = 35.0;
const THICK: f32 = 22.0;

const VIEW_HEIGHT: f32 = 950.0;

/// Seconds without any input before the AFK monsters come.
const AFK_TIME: f32 = 60.0;
/// Start warning this many seconds before they arrive.
const AFK_WARNING: f32 = 10.0;
const AFK_MAX_MONSTERS: usize = 8;
const AFK_SPAWN_EVERY: f32 = 3.5;
const MONSTER_RADIUS: f32 = 22.0;
const MONSTER_DAMAGE: f32 = 15.0;
const MONSTER_KNOCKBACK: f32 = 520.0;
const INVULNERABLE_TIME: f32 = 1.0;

const COIN_RADIUS: f32 = 11.0;
const MAGNET_RANGE: f32 = 280.0;
const MAGNET_PULL: f32 = 650.0;
const TOUGH_BONES_FACTOR: f32 = 0.6;
const WEB_WIDTH: f32 = 150.0;
const WEB_THICK: f32 = 10.0;
const WEB_LIFE: f32 = 5.0;
/// Shorter than WEB_LIFE, so a new web is ready when the one under you vanishes.
const SPIDER_RECHARGE: f32 = 4.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PowerUp {
    Spider,
    Magnet,
    ToughBones,
}

impl PowerUp {
    const ALL: [PowerUp; 3] = [PowerUp::Spider, PowerUp::Magnet, PowerUp::ToughBones];

    fn name(self) -> &'static str {
        match self {
            PowerUp::Spider => "SPIDER",
            PowerUp::Magnet => "MAGNET",
            PowerUp::ToughBones => "TOUGH BONES",
        }
    }

    fn description(self) -> &'static str {
        match self {
            PowerUp::Spider => "Senses when a fall would kill you and spins a cobweb to catch you.",
            PowerUp::Magnet => "Pulls nearby coins towards you.",
            PowerUp::ToughBones => "Take 40% less fall damage.",
        }
    }

    fn cost(self) -> u32 {
        match self {
            PowerUp::Spider => 40,
            PowerUp::Magnet => 15,
            PowerUp::ToughBones => 25,
        }
    }

    fn color(self) -> Color {
        match self {
            PowerUp::Spider => Color::new(0.85, 0.85, 0.95, 1.0),
            PowerUp::Magnet => Color::new(1.0, 0.35, 0.35, 1.0),
            PowerUp::ToughBones => Color::new(1.0, 0.95, 0.75, 1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dir {
    Down,
    Up,
    Left,
    Right,
}

impl Dir {
    const ALL: [Dir; 4] = [Dir::Down, Dir::Up, Dir::Left, Dir::Right];

    fn vec(self) -> Vec2 {
        match self {
            Dir::Down => vec2(0.0, 1.0),
            Dir::Up => vec2(0.0, -1.0),
            Dir::Left => vec2(-1.0, 0.0),
            Dir::Right => vec2(1.0, 0.0),
        }
    }

    /// Rotation that maps local "down" (0, 1) onto this direction.
    fn angle(self) -> f32 {
        use std::f32::consts::{FRAC_PI_2, PI};
        match self {
            Dir::Down => 0.0,
            Dir::Up => PI,
            Dir::Left => FRAC_PI_2,
            Dir::Right => -FRAC_PI_2,
        }
    }

    fn is_vertical(self) -> bool {
        matches!(self, Dir::Down | Dir::Up)
    }

    fn name(self) -> &'static str {
        match self {
            Dir::Down => "DOWN",
            Dir::Up => "UP",
            Dir::Left => "LEFT",
            Dir::Right => "RIGHT",
        }
    }
}

/// xorshift64* — deterministic so a world can be replayed from its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f()
    }
}

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Strict overlap: touching edges do not count.
fn overlaps(a: &Rect, b: &Rect) -> bool {
    const EPS: f32 = 0.01;
    a.x < b.x + b.w - EPS && a.x + a.w > b.x + EPS && a.y < b.y + b.h - EPS && a.y + a.h > b.y + EPS
}

fn shortest_angle(from: f32, to: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (to - from) % TAU;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    d
}

// ---------------------------------------------------------------------------
// World generation (endless, built chunk by chunk as you advance)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Boundary,
    Bump,
    Platform,
    /// Spider cobweb: catches you without fall damage.
    Web,
}

#[derive(Clone, Copy)]
struct Solid {
    rect: Rect,
    kind: Kind,
}

const CELLS_PER_CHUNK: usize = 4;
const CHUNK_W: f32 = CELL_W * CELLS_PER_CHUNK as f32;
/// Keep this much world generated ahead of the player.
const GENERATE_AHEAD: f32 = 4000.0;
/// No platforms here, so the start is calm.
const SAFE_START: f32 = 380.0;
/// World units per displayed metre.
const UNITS_PER_METRE: f32 = 50.0;

struct Coin {
    home: Vec2,
    pos: Vec2,
    taken: bool,
}

struct Chunk {
    solids: Vec<Solid>,
    coins: Vec<Coin>,
}

struct World {
    chunks: Vec<Chunk>,
    seed: u64,
    /// The shareable code this world is built from (see `seed.rs`).
    code: String,
}

impl World {
    /// The world for a normalized seed code.
    fn new(code: String) -> World {
        let mut w = World { chunks: Vec::new(), seed: seed::world_seed(&code), code };
        w.ensure_generated(0.0);
        w
    }

    fn ensure_generated(&mut self, player_x: f32) {
        while (self.chunks.len() as f32) * CHUNK_W < player_x + GENERATE_AHEAD {
            let chunk = Self::generate_chunk(self.seed, self.chunks.len());
            self.chunks.push(chunk);
        }
    }

    /// Each chunk is seeded from the world seed and its index, so a world
    /// always regenerates identically when you retry it.
    fn generate_chunk(seed: u64, index: usize) -> Chunk {
        let mut rng = Rng::new(seed ^ (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let x0 = index as f32 * CHUNK_W;
        let h = WORLD_HEIGHT;
        let mut solids = Vec::new();
        let mut push = |x: f32, y: f32, w: f32, hh: f32, kind: Kind| {
            solids.push(Solid { rect: Rect::new(x, y, w, hh), kind });
        };

        // Floor, ceiling, and a wall behind the start.
        push(x0, h, CHUNK_W, BOUNDARY, Kind::Boundary);
        push(x0, -BOUNDARY, CHUNK_W, BOUNDARY, Kind::Boundary);
        if index == 0 {
            push(-BOUNDARY, -BOUNDARY, BOUNDARY, h + 2.0 * BOUNDARY, Kind::Boundary);
        }

        // Low bumps in the floor and ceiling lanes: jumpable, and they stop
        // you sliding for ever when gravity goes sideways.
        for ceiling in [false, true] {
            let mut x = x0 + rng.range(40.0, 400.0);
            while x < x0 + CHUNK_W - 40.0 {
                if x > SAFE_START {
                    let y = if ceiling { 0.0 } else { h - BUMP_H };
                    push(x, y, 28.0, BUMP_H, Kind::Bump);
                }
                x += rng.range(330.0, 520.0);
            }
        }

        // Grid of platforms between the lanes, in every orientation. The
        // padding keeps a gap wider than the player between any two shapes.
        let rows = ((h - 2.0 * LANE) / CELL_H).floor() as i32;
        for c in 0..CELLS_PER_CHUNK {
            let cell_x = x0 + c as f32 * CELL_W;
            if cell_x < SAFE_START {
                continue;
            }
            for r in 0..rows {
                let cx = cell_x + CELL_PAD;
                let cy = LANE + r as f32 * CELL_H + CELL_PAD;
                let iw = CELL_W - 2.0 * CELL_PAD;
                let ih = CELL_H - 2.0 * CELL_PAD;
                let roll = rng.f();
                if roll < 0.12 {
                    continue; // empty cell
                }
                let slab = |rng: &mut Rng| {
                    let w = rng.range(110.0, iw);
                    let x = cx + rng.range(0.0, iw - w);
                    let y = cy + rng.range(0.0, ih - THICK);
                    Rect::new(x, y, w, THICK)
                };
                let pillar = |rng: &mut Rng| {
                    let hh = rng.range(100.0, ih);
                    let x = cx + rng.range(0.0, iw - THICK);
                    let y = cy + rng.range(0.0, ih - hh);
                    Rect::new(x, y, THICK, hh)
                };
                let mut shapes: Vec<Rect> = Vec::new();
                if roll < 0.45 {
                    shapes.push(slab(&mut rng));
                } else if roll < 0.75 {
                    shapes.push(pillar(&mut rng));
                } else if roll < 0.88 {
                    // Cross / plus
                    let w = rng.range(120.0, iw);
                    let hh = rng.range(110.0, ih);
                    let mx = cx + iw / 2.0;
                    let my = cy + ih / 2.0;
                    shapes.push(Rect::new(mx - w / 2.0, my - THICK / 2.0, w, THICK));
                    shapes.push(Rect::new(mx - THICK / 2.0, my - hh / 2.0, THICK, hh));
                } else {
                    // L / corner shape, randomly mirrored
                    let w = rng.range(110.0, iw);
                    let hh = rng.range(100.0, ih);
                    let flip_x = rng.f() < 0.5;
                    let flip_y = rng.f() < 0.5;
                    let x = cx + rng.range(0.0, iw - w);
                    let y = cy + rng.range(0.0, ih - hh);
                    let slab_y = if flip_y { y } else { y + hh - THICK };
                    let pillar_x = if flip_x { x + w - THICK } else { x };
                    shapes.push(Rect::new(x, slab_y, w, THICK));
                    shapes.push(Rect::new(pillar_x, y, THICK, hh));
                }
                for s in shapes {
                    push(s.x, s.y, s.w, s.h, Kind::Platform);
                }
            }
        }

        // Coins: short rows along the lanes, plus some floating between platforms.
        let mut coins = Vec::new();
        let mut place = |p: Vec2, solids: &[Solid]| {
            let r = COIN_RADIUS + 4.0;
            let area = Rect::new(p.x - r, p.y - r, r * 2.0, r * 2.0);
            if p.x > SAFE_START && !solids.iter().any(|s| overlaps(&area, &s.rect)) {
                coins.push(Coin { home: p, pos: p, taken: false });
            }
        };
        for ceiling in [false, true] {
            if rng.f() < 0.6 {
                let n = 3 + (rng.f() * 3.0) as i32;
                let start = x0 + rng.range(60.0, CHUNK_W - 60.0 - n as f32 * 45.0);
                let y = if ceiling { 60.0 } else { h - 60.0 };
                for k in 0..n {
                    place(vec2(start + k as f32 * 45.0, y), &solids);
                }
            }
        }
        for c in 0..CELLS_PER_CHUNK {
            for r in 0..rows {
                if rng.f() < 0.35 {
                    let p = vec2(
                        x0 + c as f32 * CELL_W + rng.range(30.0, CELL_W - 30.0),
                        LANE + r as f32 * CELL_H + rng.range(30.0, CELL_H - 30.0),
                    );
                    place(p, &solids);
                }
            }
        }

        Chunk { solids, coins }
    }

    fn chunk_range(&self, min_x: f32, max_x: f32) -> std::ops::RangeInclusive<usize> {
        let first = ((min_x / CHUNK_W).floor().max(0.0)) as usize;
        let last = ((max_x / CHUNK_W).floor().max(0.0) as usize).min(self.chunks.len().saturating_sub(1));
        first..=last
    }

    /// Put every coin back, for retrying the same world.
    fn reset_coins(&mut self) {
        for chunk in &mut self.chunks {
            for c in &mut chunk.coins {
                c.taken = false;
                c.pos = c.home;
            }
        }
    }

    /// All solids in chunks overlapping [min_x, max_x].
    fn solids_between(&self, min_x: f32, max_x: f32) -> Vec<Solid> {
        self.chunk_range(min_x, max_x).flat_map(|i| self.chunks[i].solids.iter().copied()).collect()
    }

    /// Flips come faster the further you get.
    fn flip_interval(distance: f32) -> f32 {
        (9.0 - distance / 3000.0).max(3.5)
    }
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

struct Player {
    pos: Vec2,
    vel: Vec2,
    half: Vec2,
    health: f32,
    grounded: bool,
    coyote: f32,
    jump_buffer: f32,
    since_damage: f32,
    hurt_flash: f32,
    invulnerable: f32,
    draw_angle: f32,
    walk_phase: f32,
    air_jumps: u32,
    /// Somersault animation after a double jump, counts down to 0.
    flip_spin: f32,
    facing: f32,
}

impl Player {
    fn new(pos: Vec2) -> Player {
        Player {
            pos,
            vel: Vec2::ZERO,
            half: vec2(HALF_WIDE, HALF_TALL),
            health: MAX_HEALTH,
            grounded: false,
            coyote: 0.0,
            jump_buffer: 0.0,
            since_damage: 10.0,
            hurt_flash: 0.0,
            invulnerable: 0.0,
            draw_angle: 0.0,
            walk_phase: 0.0,
            air_jumps: AIR_JUMPS,
            flip_spin: 0.0,
            facing: 1.0,
        }
    }

    fn rect(&self) -> Rect {
        Rect::new(self.pos.x - self.half.x, self.pos.y - self.half.y, self.half.x * 2.0, self.half.y * 2.0)
    }

    /// Reorient the hitbox for a new gravity and push out of anything it now overlaps.
    fn set_gravity(&mut self, g: Dir, solids: &[Solid]) {
        self.half = if g.is_vertical() { vec2(HALF_WIDE, HALF_TALL) } else { vec2(HALF_TALL, HALF_WIDE) };
        for _ in 0..8 {
            let mut moved = false;
            for s in solids {
                let r = self.rect();
                let b = &s.rect;
                if !overlaps(&r, b) {
                    continue;
                }
                let pushes = [
                    (vec2(-(r.x + r.w - b.x), 0.0)),
                    (vec2(b.x + b.w - r.x, 0.0)),
                    (vec2(0.0, -(r.y + r.h - b.y))),
                    (vec2(0.0, b.y + b.h - r.y)),
                ];
                let best = pushes.iter().min_by(|a, b| a.length().total_cmp(&b.length())).unwrap();
                self.pos += *best;
                moved = true;
            }
            if !moved {
                break;
            }
        }
        self.grounded = false;
    }

    /// Moves along one axis and returns the signed velocity at impact, if any.
    fn move_axis(&mut self, solids: &[Solid], axis: usize, delta: f32) -> Option<(f32, Kind)> {
        if delta == 0.0 {
            return None;
        }
        self.pos[axis] += delta;
        let mut impact = None;
        for s in solids {
            let r = self.rect();
            if !overlaps(&r, &s.rect) {
                continue;
            }
            let (lo, size) = if axis == 0 { (s.rect.x, s.rect.w) } else { (s.rect.y, s.rect.h) };
            self.pos[axis] = if delta > 0.0 { lo - self.half[axis] } else { lo + size + self.half[axis] };
            impact = Some((self.vel[axis], s.kind));
            self.vel[axis] = 0.0;
        }
        impact
    }
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

struct Monster {
    pos: Vec2,
    vel: Vec2,
    /// Animation offset so they don't all wobble in sync.
    phase: f32,
    /// Set once you're back at the keyboard: they flee and fade out.
    fleeing: bool,
    alpha: f32,
}

struct Popup {
    pos: Vec2,
    text: String,
    life: f32,
    color: Color,
}

struct Dust {
    pos: Vec2,
    vel: Vec2,
    life: f32,
}

struct Star {
    pos: Vec2, // normalised screen coords 0..1
    depth: f32,
}

// ---------------------------------------------------------------------------
// Game
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
enum State {
    Title,
    Shop,
    Playing,
    Dead,
    /// Typing a seed code.
    EnterSeed,
}

struct Game {
    state: State,
    world: World,
    player: Player,
    gravity: Dir,
    next_gravity: Dir,
    flip_timer: f32,
    flip_flash: f32,
    rng: Rng,
    /// Gravity flip directions: restarted from the world seed every run, so a seed always
    /// flips the same way, for you and for your friend.
    flip_rng: Rng,
    cam: Vec2,
    shake: f32,
    popups: Vec<Popup>,
    dust: Vec<Dust>,
    stars: Vec<Star>,
    deaths: u32,
    /// Furthest x reached this run, and the best run so far.
    furthest: f32,
    best: f32,
    /// Best distance before this run started, marked in the world as a target.
    best_before_run: f32,
    /// Seconds since the last input.
    idle: f32,
    monsters: Vec<Monster>,
    monster_spawn: f32,
    death_cause: &'static str,
    /// Wallet, kept across runs.
    coins: u32,
    coins_this_run: u32,
    /// Power-ups bought for the current/next round; used up when the round ends.
    owned: Vec<PowerUp>,
    /// Power-ups that just got used up, to mention on the death screen.
    used_up: Vec<PowerUp>,
    webs: Vec<(Solid, f32)>,
    spider_cooldown: f32,
    /// Where to go back to when leaving the shop.
    shop_return: State,
    shop_message: (String, f32),
    /// What has been typed in the seed box, and where Esc goes back to.
    seed_input: String,
    seed_return: State,
}

/// Mixed into the world seed for the gravity-flip sequence, so it doesn't mirror world generation.
const FLIP_SALT: u64 = 0xF11F_5EED_D1B5_4A33;

fn spawn_point() -> Vec2 {
    vec2(120.0, WORLD_HEIGHT - HALF_TALL)
}

impl Game {
    fn new(seed: u64) -> Game {
        let mut rng = Rng::new(seed ^ 0x9E37_79B9_7F4A_7C15);
        let world = World::new(seed::random_code(rng.next_u64()));
        let stars = (0..90)
            .map(|_| Star { pos: vec2(rng.f(), rng.f()), depth: rng.range(0.2, 1.0) })
            .collect();
        let mut g = Game {
            state: State::Title,
            player: Player::new(spawn_point()),
            gravity: Dir::Down,
            next_gravity: Dir::Up,
            flip_timer: World::flip_interval(0.0),
            flip_flash: 0.0,
            world,
            rng,
            flip_rng: Rng::new(1),
            cam: spawn_point(),
            shake: 0.0,
            popups: Vec::new(),
            dust: Vec::new(),
            stars,
            deaths: 0,
            furthest: 0.0,
            best: 0.0,
            best_before_run: 0.0,
            idle: 0.0,
            monsters: Vec::new(),
            monster_spawn: 0.0,
            death_cause: "",
            coins: 0,
            coins_this_run: 0,
            owned: Vec::new(),
            used_up: Vec::new(),
            webs: Vec::new(),
            spider_cooldown: 0.0,
            shop_return: State::Title,
            shop_message: (String::new(), 0.0),
            seed_input: String::new(),
            seed_return: State::Title,
        };
        g.restart_run();
        g
    }

    fn pick_next(&mut self) -> Dir {
        let options: Vec<Dir> = Dir::ALL.iter().copied().filter(|d| *d != self.gravity).collect();
        options[(self.flip_rng.next_u64() % options.len() as u64) as usize]
    }

    fn nearby_solids(&self) -> Vec<Solid> {
        let x = self.player.pos.x;
        let mut solids = self.world.solids_between(x - 200.0, x + 200.0);
        solids.extend(self.webs.iter().map(|(w, _)| *w));
        solids
    }

    fn has(&self, p: PowerUp) -> bool {
        self.owned.contains(&p)
    }

    fn fall_damage(&self, speed: f32) -> f32 {
        let mult = if self.has(PowerUp::ToughBones) { TOUGH_BONES_FACTOR } else { 1.0 };
        (speed - SAFE_LANDING_SPEED).max(0.0) * DAMAGE_PER_SPEED * mult
    }

    /// Distance from the player's "feet" to the first solid below them, along gravity.
    fn drop_distance(&self, solids: &[Solid]) -> f32 {
        let r = self.player.rect();
        let mut best = f32::INFINITY;
        for s in solids {
            let b = &s.rect;
            let (lined_up, gap) = match self.gravity {
                Dir::Down => (b.x < r.x + r.w && b.x + b.w > r.x, b.y - (r.y + r.h)),
                Dir::Up => (b.x < r.x + r.w && b.x + b.w > r.x, r.y - (b.y + b.h)),
                Dir::Right => (b.y < r.y + r.h && b.y + b.h > r.y, b.x - (r.x + r.w)),
                Dir::Left => (b.y < r.y + r.h && b.y + b.h > r.y, r.x - (b.x + b.w)),
            };
            if lined_up && gap >= -0.5 {
                best = best.min(gap.max(0.0));
            }
        }
        best
    }

    /// Spider: if the landing ahead would kill you, spin a web in the way.
    fn spider_check(&mut self) {
        if !self.has(PowerUp::Spider) || self.spider_cooldown > 0.0 || self.player.grounded {
            return;
        }
        // Look wide enough to see the floor even when it's in another chunk.
        let x = self.player.pos.x;
        let mut solids = self.world.solids_between(x - WORLD_HEIGHT, x + WORLD_HEIGHT);
        solids.extend(self.webs.iter().map(|(w, _)| *w));
        let dist = self.drop_distance(&solids);
        if !dist.is_finite() {
            return;
        }
        let g = self.gravity.vec();
        let fall = self.player.vel.dot(g);
        let impact = (fall * fall + 2.0 * GRAVITY * dist).sqrt().min(MAX_FALL);
        if self.fall_damage(impact) < self.player.health {
            return;
        }
        // Not enough room to fit a web.
        if dist < WEB_THICK + 6.0 {
            return;
        }
        let p = &self.player;
        let feet = p.pos + g * p.half.dot(g.abs());
        let gap = (dist - WEB_THICK - 4.0).min(30.0);
        let near = feet + g * gap;
        let far = near + g * WEB_THICK;
        let center = (near + far) / 2.0;
        let size = if self.gravity.is_vertical() { vec2(WEB_WIDTH, WEB_THICK) } else { vec2(WEB_THICK, WEB_WIDTH) };
        let rect = Rect::new(center.x - size.x / 2.0, center.y - size.y / 2.0, size.x, size.y);
        self.webs.push((Solid { rect, kind: Kind::Web }, WEB_LIFE));
        self.spider_cooldown = SPIDER_RECHARGE;
        self.popups.push(Popup {
            pos: center - g * 60.0,
            text: "SPIDER WEB!".to_owned(),
            life: 1.2,
            color: PowerUp::Spider.color(),
        });
    }

    fn update_coins(&mut self, dt: f32) {
        let pos = self.player.pos;
        let magnet = self.has(PowerUp::Magnet);
        let range = self.world.chunk_range(pos.x - MAGNET_RANGE, pos.x + MAGNET_RANGE);
        let mut got = 0;
        for i in range {
            for c in &mut self.world.chunks[i].coins {
                if c.taken {
                    continue;
                }
                let to = pos - c.pos;
                if magnet && to.length() < MAGNET_RANGE {
                    c.pos += to.normalize_or_zero() * (MAGNET_PULL * dt).min(to.length());
                }
                if (pos - c.pos).length() < COIN_RADIUS + HALF_TALL * 0.8 {
                    c.taken = true;
                    got += 1;
                    self.popups.push(Popup {
                        pos: c.pos,
                        text: "+1".to_owned(),
                        life: 0.6,
                        color: Color::new(1.0, 0.85, 0.2, 1.0),
                    });
                }
            }
        }
        self.coins += got;
        self.coins_this_run += got;
    }

    fn open_shop(&mut self, from: State) {
        self.shop_return = from;
        self.shop_message = (String::new(), 0.0);
        self.state = State::Shop;
    }

    fn try_buy(&mut self, p: PowerUp) {
        let msg = if self.has(p) {
            format!("{} is already ready for this round.", p.name())
        } else if self.coins < p.cost() {
            format!("Not enough coins for {} - need {} more.", p.name(), p.cost() - self.coins)
        } else {
            self.coins -= p.cost();
            self.owned.push(p);
            format!("{} is ready - it lasts for your next round.", p.name())
        };
        self.shop_message = (msg, 2.5);
    }

    /// Card layout shared by drawing and mouse clicks.
    fn shop_cards() -> Vec<(PowerUp, Rect)> {
        let (w, h) = (screen_width(), screen_height());
        let card_w = ((w - 120.0) / 3.0).min(320.0);
        let card_h = 260.0;
        let total = card_w * 3.0 + 40.0;
        PowerUp::ALL
            .iter()
            .enumerate()
            .map(|(i, p)| (*p, Rect::new(w / 2.0 - total / 2.0 + i as f32 * (card_w + 20.0), h / 2.0 - card_h / 2.0, card_w, card_h)))
            .collect()
    }

    fn distance(&self) -> f32 {
        self.furthest / UNITS_PER_METRE
    }

    /// Power-ups only last one round.
    fn end_round(&mut self) {
        self.used_up = std::mem::take(&mut self.owned);
    }

    fn restart_run(&mut self) {
        self.flip_rng = Rng::new(self.world.seed ^ FLIP_SALT);
        self.player = Player::new(spawn_point());
        self.gravity = Dir::Down;
        let solids = self.nearby_solids();
        self.player.set_gravity(Dir::Down, &solids);
        self.next_gravity = self.pick_next();
        self.flip_timer = World::flip_interval(0.0);
        self.furthest = 0.0;
        self.best_before_run = self.best;
        self.cam = self.player.pos;
        self.popups.clear();
        self.dust.clear();
        self.monsters.clear();
        self.webs.clear();
        self.spider_cooldown = 0.0;
        self.coins_this_run = 0;
        self.world.reset_coins();
        self.idle = 0.0;
    }

    fn update_shop(&mut self) {
        let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
        for (i, k) in keys.iter().enumerate() {
            if is_key_pressed(*k) {
                self.try_buy(PowerUp::ALL[i]);
            }
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let m: Vec2 = mouse_position().into();
            if let Some((p, _)) = Self::shop_cards().into_iter().find(|(_, r)| r.contains(m)) {
                self.try_buy(p);
            }
        }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::B) || is_key_pressed(KeyCode::Space) {
            self.state = if self.shop_return == State::Dead { State::Dead } else { State::Title };
        }
    }

    fn open_seed_entry(&mut self, from: State) {
        self.seed_return = from;
        self.seed_input.clear();
        self.state = State::EnterSeed;
    }

    fn update_seed_entry(&mut self) {
        while let Some(c) = get_char_pressed() {
            self.seed_input = seed::normalize(&format!("{}{c}", self.seed_input));
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.seed_input.pop();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.state = if self.seed_return == State::Dead { State::Dead } else { State::Title };
        } else if is_key_pressed(KeyCode::Enter) {
            let code = if self.seed_input.is_empty() {
                seed::random_code(self.rng.next_u64())
            } else {
                self.seed_input.clone()
            };
            self.world = World::new(code);
            self.state = State::Playing;
            self.restart_run();
        }
    }

    fn new_world(&mut self) {
        self.world = World::new(seed::random_code(self.rng.next_u64()));
        self.restart_run();
    }

    fn flip(&mut self) {
        self.gravity = self.next_gravity;
        self.next_gravity = self.pick_next();
        self.flip_timer = World::flip_interval(self.furthest);
        self.flip_flash = 1.0;
        self.shake = self.shake.max(6.0);
        let solids = self.nearby_solids();
        self.player.set_gravity(self.gravity, &solids);
    }

    fn input_axis(&self) -> f32 {
        let (neg, pos) = if self.gravity.is_vertical() {
            (
                is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
                is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            )
        } else {
            (
                is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
                is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
            )
        };
        (pos as i32 - neg as i32) as f32
    }

    fn jump_pressed(&self) -> bool {
        // Space always jumps; so does the key pointing away from the current "floor".
        let away = match self.gravity {
            Dir::Down => [KeyCode::Up, KeyCode::W],
            Dir::Up => [KeyCode::Down, KeyCode::S],
            Dir::Left => [KeyCode::Right, KeyCode::D],
            Dir::Right => [KeyCode::Left, KeyCode::A],
        };
        is_key_pressed(KeyCode::Space) || away.iter().any(|k| is_key_pressed(*k))
    }

    fn update(&mut self, dt: f32) {
        self.update_effects(dt);
        // macroquad keeps every typed character until it is read: outside the seed box, throw
        // them away so they don't pile up or spill into the box when it opens (the S included).
        if self.state != State::EnterSeed {
            while get_char_pressed().is_some() {}
        }
        // Debug-build cheat for testing the shop: C gives 50 coins.
        if cfg!(debug_assertions) && self.state != State::EnterSeed && is_key_pressed(KeyCode::C) {
            self.coins += 50;
            self.shop_message = ("Cheat: +50 coins".to_owned(), 2.0);
            self.popups.push(Popup {
                pos: self.player.pos - vec2(0.0, 60.0),
                text: "+50 (cheat)".to_owned(),
                life: 1.0,
                color: Color::new(1.0, 0.85, 0.2, 1.0),
            });
        }
        match self.state {
            State::Title => {
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Title);
                } else if is_key_pressed(KeyCode::S) {
                    self.open_seed_entry(State::Title);
                } else if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                    self.state = State::Playing;
                    self.restart_run();
                }
            }
            State::Shop => self.update_shop(),
            State::EnterSeed => self.update_seed_entry(),
            State::Dead => {
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Dead);
                } else if is_key_pressed(KeyCode::S) {
                    self.open_seed_entry(State::Dead);
                } else if is_key_pressed(KeyCode::R) || is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    self.state = State::Playing;
                    self.restart_run();
                } else if is_key_pressed(KeyCode::N) {
                    self.state = State::Playing;
                    self.new_world();
                }
            }
            State::Playing => {
                if is_key_pressed(KeyCode::R) {
                    self.end_round();
                    self.restart_run();
                    return;
                }
                self.update_playing(dt);
            }
        }
        self.update_camera(dt);
    }

    fn any_input() -> bool {
        !get_keys_down().is_empty() || is_mouse_button_down(MouseButton::Left)
    }

    /// Applies damage and reports whether it killed you.
    fn hurt(&mut self, damage: f32, cause: &'static str) -> bool {
        let g = self.gravity.vec();
        let p = &mut self.player;
        p.health -= damage;
        p.since_damage = 0.0;
        p.hurt_flash = 1.0;
        self.shake = self.shake.max(4.0 + damage * 0.35);
        self.popups.push(Popup {
            pos: p.pos - g * 40.0,
            text: format!("-{}", damage.ceil() as i32),
            life: 1.0,
            color: Color::new(1.0, 0.3, 0.3, 1.0),
        });
        if p.health <= 0.0 {
            p.health = 0.0;
            self.deaths += 1;
            self.death_cause = cause;
            self.state = State::Dead;
            self.end_round();
            return true;
        }
        false
    }

    fn update_monsters(&mut self, dt: f32) -> bool {
        if Self::any_input() {
            self.idle = 0.0;
        } else {
            self.idle += dt;
        }

        let afk = self.idle >= AFK_TIME;
        if afk {
            self.monster_spawn -= dt;
            let active = self.monsters.iter().filter(|m| !m.fleeing).count();
            if self.monster_spawn <= 0.0 && active < AFK_MAX_MONSTERS {
                self.monster_spawn = AFK_SPAWN_EVERY;
                // Appear just off-screen, from a random direction.
                let angle = rand::gen_range(0.0, std::f32::consts::TAU);
                let reach = self.view_size().length() / 2.0 + 80.0;
                self.monsters.push(Monster {
                    pos: self.player.pos + rotate(vec2(reach, 0.0), angle),
                    vel: Vec2::ZERO,
                    phase: rand::gen_range(0.0, 10.0),
                    fleeing: false,
                    alpha: 1.0,
                });
            }
        } else {
            self.monster_spawn = 0.0;
            for m in &mut self.monsters {
                m.fleeing = true;
            }
        }

        // They get faster the longer you stay away.
        let speed = (170.0 + 12.0 * (self.idle - AFK_TIME).max(0.0)).min(420.0);
        let target = self.player.pos;
        let mut hit_from = None;
        for m in &mut self.monsters {
            let to_player = target - m.pos;
            let dir = to_player.normalize_or_zero();
            if m.fleeing {
                m.vel += -dir * 900.0 * dt;
                m.alpha -= dt * 1.2;
            } else {
                m.vel += (dir * speed - m.vel) * (2.5 * dt).min(1.0);
                if to_player.length() < MONSTER_RADIUS + HALF_TALL * 0.7 && hit_from.is_none() {
                    hit_from = Some(dir);
                    // Bounce off after landing a hit.
                    m.vel = -dir * 300.0;
                }
            }
            m.pos += m.vel * dt;
        }
        self.monsters.retain(|m| m.alpha > 0.0);

        if let Some(dir) = hit_from {
            if self.player.invulnerable <= 0.0 {
                self.player.invulnerable = INVULNERABLE_TIME;
                // Shove away from the monster and off the floor.
                self.player.vel += dir * MONSTER_KNOCKBACK - self.gravity.vec() * MONSTER_KNOCKBACK * 0.8;
                return self.hurt(MONSTER_DAMAGE, "The AFK monsters got you!");
            }
        }
        false
    }

    fn update_playing(&mut self, dt: f32) {
        if self.update_monsters(dt) {
            return;
        }
        self.flip_timer -= dt;
        if self.flip_timer <= 0.0 {
            self.flip();
        }

        let g = self.gravity.vec();
        let perp = vec2(g.y.abs(), g.x.abs());
        let input = self.input_axis();
        let jump = self.jump_pressed();

        self.spider_cooldown = (self.spider_cooldown - dt).max(0.0);
        for (_, life) in &mut self.webs {
            *life -= dt;
        }
        self.webs.retain(|(_, life)| *life > 0.0);
        self.spider_check();
        let solids = self.nearby_solids();

        let p = &mut self.player;
        p.since_damage += dt;
        p.hurt_flash = (p.hurt_flash - dt * 3.0).max(0.0);
        p.invulnerable = (p.invulnerable - dt).max(0.0);
        p.coyote = if p.grounded { COYOTE_TIME } else { (p.coyote - dt).max(0.0) };
        p.jump_buffer = if jump { JUMP_BUFFER } else { (p.jump_buffer - dt).max(0.0) };

        // Split velocity into "along the floor" and "falling".
        let mut along = p.vel.dot(perp);
        let mut fall = p.vel.dot(g);
        let accel = if p.grounded { GROUND_ACCEL } else { AIR_ACCEL };
        let target = input * MOVE_SPEED;
        along += (target - along).clamp(-accel * dt, accel * dt);
        fall = (fall + GRAVITY * dt).min(MAX_FALL);
        if p.grounded {
            p.air_jumps = AIR_JUMPS;
        }
        p.flip_spin = (p.flip_spin - dt).max(0.0);
        let mut double_jumped = false;
        if p.jump_buffer > 0.0 && p.coyote > 0.0 {
            fall = -JUMP_SPEED;
            p.jump_buffer = 0.0;
            p.coyote = 0.0;
        } else if jump && p.air_jumps > 0 {
            fall = -DOUBLE_JUMP_SPEED;
            p.air_jumps -= 1;
            p.jump_buffer = 0.0;
            p.flip_spin = DOUBLE_JUMP_SPIN_TIME;
            double_jumped = true;
        }
        p.vel = perp * along + g * fall;

        // Local "right" for the stickman sprite, used for facing.
        let local_right = rotate(vec2(1.0, 0.0), self.gravity.angle());
        let side_speed = p.vel.dot(local_right);
        if side_speed.abs() > 20.0 {
            p.facing = side_speed.signum();
        }

        // Sub-stepped movement so fast falls never tunnel through thin platforms.
        let travel = p.vel * dt;
        let steps = (travel.abs().max_element() / 8.0).ceil().max(1.0) as i32;
        let mut landing_speed: f32 = 0.0;
        let mut caught_by_web = false;
        let mut grounded = false;
        let gravity_axis = if self.gravity.is_vertical() { 1 } else { 0 };
        for _ in 0..steps {
            for axis in 0..2 {
                let delta = p.vel[axis] * dt / steps as f32;
                if let Some((v, kind)) = p.move_axis(&solids, axis, delta) {
                    // Only count hits on the side gravity is pulling toward.
                    if axis == gravity_axis && v.signum() == g[axis].signum() {
                        grounded = true;
                        if kind == Kind::Web {
                            caught_by_web |= v.abs() > SAFE_LANDING_SPEED;
                        } else {
                            landing_speed = landing_speed.max(v.abs());
                        }
                    }
                }
            }
        }

        if double_jumped {
            // Puff of air where you kicked off.
            let feet = p.pos + g * p.half.dot(g.abs());
            for i in 0..10 {
                let a = i as f32 / 10.0 * std::f32::consts::TAU;
                self.dust.push(Dust { pos: feet, vel: vec2(a.cos(), a.sin()) * 120.0 + g * 60.0, life: 0.4 });
            }
        }

        let was_grounded = p.grounded;
        p.grounded = grounded;
        if grounded {
            p.walk_phase += along.abs() * dt * 0.045;
        }

        if grounded && !was_grounded && landing_speed > 300.0 {
            let feet = p.pos + g * p.half.dot(g.abs());
            for i in 0..8 {
                let spread = (i as f32 - 3.5) / 3.5;
                self.dust.push(Dust {
                    pos: feet,
                    vel: perp * spread * 140.0 - g * rand::gen_range(20.0, 90.0),
                    life: 0.5,
                });
            }
        }

        let x = p.pos.x;
        let regen = p.grounded && p.since_damage > REGEN_DELAY;
        if caught_by_web {
            self.shake = self.shake.max(5.0);
        }
        if landing_speed > SAFE_LANDING_SPEED {
            let damage = self.fall_damage(landing_speed);
            if self.hurt(damage, "Too much fall damage!") {
                return;
            }
        } else if regen {
            let p = &mut self.player;
            p.health = (p.health + REGEN_PER_SEC * dt).min(MAX_HEALTH);
        }
        self.update_coins(dt);

        self.furthest = self.furthest.max(x);
        self.best = self.best.max(self.furthest);
        self.world.ensure_generated(x);
    }

    fn update_effects(&mut self, dt: f32) {
        let p = &mut self.player;
        let target = self.gravity.angle();
        let d = shortest_angle(p.draw_angle, target);
        p.draw_angle += d * (1.0 - (-14.0 * dt).exp());

        self.shake = (self.shake - dt * 30.0).max(0.0);
        self.flip_flash = (self.flip_flash - dt * 2.5).max(0.0);
        self.shop_message.1 -= dt;

        for pp in &mut self.popups {
            pp.life -= dt;
            pp.pos.y -= 40.0 * dt;
        }
        self.popups.retain(|pp| pp.life > 0.0);

        let g = self.gravity.vec();
        for d in &mut self.dust {
            d.life -= dt;
            d.vel += g * 400.0 * dt;
            d.pos += d.vel * dt;
        }
        self.dust.retain(|d| d.life > 0.0);

        // Background dust drifts with gravity so you can always feel which way is down.
        for s in &mut self.stars {
            s.pos += g * 0.05 * s.depth * dt;
            s.pos.x = s.pos.x.rem_euclid(1.0);
            s.pos.y = s.pos.y.rem_euclid(1.0);
        }
    }

    fn view_size(&self) -> Vec2 {
        vec2(VIEW_HEIGHT * screen_width() / screen_height(), VIEW_HEIGHT)
    }

    fn update_camera(&mut self, dt: f32) {
        let view = self.view_size();
        let mut target = self.player.pos + self.gravity.vec() * -60.0;
        target.x = target.x.max(view.x / 2.0 - BOUNDARY);
        let (min_y, max_y) = (view.y / 2.0 - BOUNDARY, WORLD_HEIGHT - view.y / 2.0 + BOUNDARY);
        target.y = if min_y < max_y { target.y.clamp(min_y, max_y) } else { WORLD_HEIGHT / 2.0 };
        self.cam += (target - self.cam) * (1.0 - (-6.0 * dt).exp());
    }

}

fn window_conf() -> Conf {
    Conf {
        window_title: "Gravity".to_owned(),
        window_width: 1280,
        window_height: 760,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = (miniquad::date::now() * 1000.0) as u64;
    rand::srand(seed);
    render::warm_font_cache();
    let mut game = Game::new(seed);

    // Native-only debug hook: GRAVITY_SHOT=out.png grabs a screenshot mid-game and exits.
    #[cfg(not(target_arch = "wasm32"))]
    let shot = std::env::var("GRAVITY_SHOT").ok();
    #[cfg(not(target_arch = "wasm32"))]
    let shot_frame: u32 = std::env::var("GRAVITY_SHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(150);
    #[cfg(not(target_arch = "wasm32"))]
    if shot.is_some() {
        game.state = State::Playing;
    }
    let mut frame = 0u32;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);
        game.update(dt);
        game.draw();

        frame += 1;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &shot {
            if frame == 90 {
                game.flip_timer = 2.0; // show the urgent countdown state
            }
            if frame == shot_frame {
                get_screen_data().export_png(path);
                break;
            }
        }
        let _ = frame;
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every solid and coin position in the first few chunks of the world for `code`.
    fn layout(code: &str) -> (Vec<Rect>, Vec<Vec2>) {
        let mut world = World::new(code.to_owned());
        world.ensure_generated(3.0 * CHUNK_W);
        let solids = world.chunks.iter().flat_map(|c| c.solids.iter().map(|s| s.rect)).collect();
        let coins = world.chunks.iter().flat_map(|c| c.coins.iter().map(|c| c.home)).collect();
        (solids, coins)
    }

    /// The first `n` gravity directions of a fresh run in the world for `code`.
    fn flips(game: &mut Game, code: &str, n: usize) -> Vec<Dir> {
        game.world = World::new(code.to_owned());
        game.restart_run();
        let mut seen = vec![game.next_gravity];
        for _ in 1..n {
            game.flip();
            seen.push(game.next_gravity);
        }
        seen
    }

    #[test]
    fn same_code_builds_the_same_world() {
        assert_eq!(layout("BANANA"), layout("BANANA"));
    }

    #[test]
    fn different_codes_build_different_worlds() {
        assert_ne!(layout("BANANA").0, layout("K7Q2X").0);
    }

    #[test]
    fn same_code_flips_gravity_the_same_way() {
        // Two games with different internal seeds stand in for two friends' computers.
        let first = flips(&mut Game::new(1), "BANANA", 12);
        assert_eq!(first, flips(&mut Game::new(987_654_321), "BANANA", 12));
    }

    #[test]
    fn retrying_replays_the_same_flips() {
        let mut game = Game::new(5);
        let first = flips(&mut game, "BANANA", 12);
        game.flip();
        game.flip();
        game.restart_run(); // what R does
        let mut again = vec![game.next_gravity];
        for _ in 1..12 {
            game.flip();
            again.push(game.next_gravity);
        }
        assert_eq!(first, again);
    }
}
