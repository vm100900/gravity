# Shareable World Seeds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every world gets a short shareable seed code (or any typed word); typing a code plays that exact world, with the same platforms, coins and gravity-flip order.

**Architecture:** A new pure module `src/seed.rs` turns text into codes and codes into the 64-bit world seed. `World` is built from a code instead of a raw number, gravity flips get their own RNG restarted from the world seed every run, and a new `State::EnterSeed` screen (opened with S on the title and death screens) lets players type a code.

**Tech Stack:** Rust 2024, macroquad 0.4.16 (`get_char_pressed`, `is_key_pressed`, `measure_text`), `cargo test`, headless Chromium + playwright-core for the browser check (harness in `/tmp/gravity-fs-test`, not committed).

Spec: `docs/superpowers/specs/2026-09-30-shareable-seeds-design.md`

## Global Constraints

- Random codes: exactly 5 characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ` (31 symbols; no 0/O, 1/I/L).
- Typed codes: upper-cased, ASCII letters and digits only, at most 12 characters (`MAX_LEN = 12`).
- `world_seed`: FNV-1a 64-bit over the code's bytes, then the splitmix64 finalizer. Golden values (computed independently in Python): `BANANA` → `0x98FF_0611_5810_37FD`, `K7Q2X` → `0x4FD0_000E_75F4_D364`.
- Keys: **S** opens the seed box (title and death screens only); **Enter** plays (blank → random world); **Backspace** deletes; **Esc** returns to where you came from.
- Exact text: HUD `world {code}  deaths {n}`; death screen `World {code} - give this seed to a friend to race the same world` and `S - type a seed`; title `S - type a seed to play the same world as a friend`; the title's AFK line must use `AFK_TIME` (60 s), not "30".
- Out of scope: share links / URL parameters, clipboard, best distance per seed, randomness of AFK monsters, dust and stars.
- The working tree already has unrelated uncommitted changes (`web/index.html`, `.gitignore`). Every commit adds only the files its task names.

## File Structure

- **Create `src/seed.rs`** — pure seed functions (`normalize`, `random_code`, `world_seed`) and their unit tests. No macroquad, no game state.
- **Modify `src/main.rs`** — `mod seed;`; `World` built from a code; `Game.flip_rng`; seed-entry state, input, drawing; HUD/title/death text; a `#[cfg(test)] mod tests` for world/flip determinism.

---

### Task 1: Seed codes (`src/seed.rs`)

**Files:**
- Create: `src/seed.rs`
- Modify: `src/main.rs:1-3` (add `mod seed;`)

**Interfaces:**
- Produces: `seed::normalize(text: &str) -> String`, `seed::random_code(bits: u64) -> String`, `seed::world_seed(code: &str) -> u64`, `seed::CODE_LEN: usize = 5`, `seed::MAX_LEN: usize = 12`.

- [ ] **Step 1: Write the failing tests**

Create `src/seed.rs` containing only the doc comment and tests:

```rust
//! Shareable world seeds: a short code such as "K7Q2X" (or any word) picks a world, so friends
//! can play the same one.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_upper_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalize(" k7q-2x! "), "K7Q2X");
        assert_eq!(normalize("banana"), "BANANA");
        assert_eq!(normalize("é ü 😀"), "");
    }

    #[test]
    fn normalize_stops_at_max_len() {
        assert_eq!(normalize("abcdefghijklmnop"), "ABCDEFGHIJKL");
        assert_eq!(normalize("abcdefghijklmnop").len(), MAX_LEN);
    }

    #[test]
    fn random_code_is_short_and_unambiguous() {
        assert_eq!(random_code(0), "22222");
        assert_eq!(random_code(1), "32222");
        assert_eq!(random_code(31), "23222");
        assert_eq!(random_code(123_456_789), "425QB");
        for bits in [0, 7, 1 << 40, u64::MAX] {
            let code = random_code(bits);
            assert_eq!(code.len(), CODE_LEN);
            assert!(code.bytes().all(|b| ALPHABET.contains(&b)), "{code}");
            assert_eq!(normalize(&code), code, "random codes are already normalized");
        }
    }

    #[test]
    fn world_seed_is_pinned() {
        // Golden values computed independently. If these change, every shared code breaks.
        assert_eq!(world_seed("BANANA"), 0x98FF_0611_5810_37FD);
        assert_eq!(world_seed("K7Q2X"), 0x4FD0_000E_75F4_D364);
    }

    #[test]
    fn similar_codes_give_different_seeds() {
        assert_ne!(world_seed("K7Q2X"), world_seed("K7Q2Y"));
        assert_ne!(world_seed("A"), world_seed("B"));
    }
}
```

In `src/main.rs`, replace the first three lines:

```rust
//! GRAVITY — a procedurally generated stickman obby where gravity keeps changing direction.

use macroquad::prelude::*;
```

with:

```rust
//! GRAVITY — a procedurally generated stickman obby where gravity keeps changing direction.

mod seed;

use macroquad::prelude::*;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test seed::`
Expected: compile errors — `cannot find function normalize`, `random_code`, `world_seed`, values `MAX_LEN`, `CODE_LEN`, `ALPHABET`.

- [ ] **Step 3: Implement**

Insert between the doc comment and `#[cfg(test)]` in `src/seed.rs`:

```rust
/// Random codes only use characters that can't be mistaken for each other (no 0/O or 1/I/L).
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
/// Length of a random code.
pub const CODE_LEN: usize = 5;
/// Longest code a player can type.
pub const MAX_LEN: usize = 12;

/// Upper-cases, keeps only ASCII letters and digits, and stops at `MAX_LEN` characters.
pub fn normalize(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .take(MAX_LEN)
        .collect()
}

/// A random code, built from `bits`.
pub fn random_code(mut bits: u64) -> String {
    let base = ALPHABET.len() as u64;
    (0..CODE_LEN)
        .map(|_| {
            let c = ALPHABET[(bits % base) as usize] as char;
            bits /= base;
            c
        })
        .collect()
}

/// The world-generation seed for a normalized code. Integer-only, so every platform builds the
/// same world. Changing this changes every world and breaks codes people have already shared.
pub fn world_seed(code: &str) -> u64 {
    // FNV-1a...
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in code.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    // ...then the splitmix64 finalizer, so similar codes give unrelated worlds.
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test seed::`
Expected: `5 passed; 0 failed`. (Dead-code warnings for the `seed` functions are expected until Task 2 uses them.)

- [ ] **Step 5: Commit**

```bash
git add src/seed.rs src/main.rs
git commit -m "Add shareable seed codes (normalize, random codes, world seed hash)"
```

---

### Task 2: Worlds from codes, and the same flips for the same code

**Files:**
- Modify: `src/main.rs` — `World` (struct ~line 281, `new` ~line 287), `Game` struct (~line 594), `Game::new` (~line 638), `pick_next` (~line 679), `restart_run` (~line 831), `new_world` (~line 869); append a `#[cfg(test)] mod tests` at the end of the file.

**Interfaces:**
- Consumes: `seed::random_code(bits: u64) -> String`, `seed::world_seed(code: &str) -> u64` (Task 1).
- Produces: `World::new(code: String) -> World`, field `World.code: String`, field `Game.flip_rng: Rng`, const `FLIP_SALT: u64`.

- [ ] **Step 1: Write the failing tests**

Append to the end of `src/main.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test tests::`
Expected: compile error — `World::new` expects `u64`, found `String` (`mismatched types`).

- [ ] **Step 3: Build worlds from codes**

In `src/main.rs`, replace:

```rust
struct World {
    chunks: Vec<Chunk>,
    seed: u64,
}

impl World {
    fn new(seed: u64) -> World {
        let mut w = World { chunks: Vec::new(), seed };
        w.ensure_generated(0.0);
        w
    }
```

with:

```rust
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
```

In `Game::new`, replace `let world = World::new(rng.next_u64());` with:

```rust
        let world = World::new(seed::random_code(rng.next_u64()));
```

In `new_world`, replace `self.world = World::new(self.rng.next_u64());` with:

```rust
        self.world = World::new(seed::random_code(self.rng.next_u64()));
```

- [ ] **Step 4: Give gravity flips their own RNG**

Directly above `fn spawn_point()` add:

```rust
/// Mixed into the world seed for the gravity-flip sequence, so it doesn't mirror world generation.
const FLIP_SALT: u64 = 0xF11F_5EED_D1B5_4A33;
```

In `struct Game`, after the line `rng: Rng,` add:

```rust
    /// Gravity flip directions: restarted from the world seed every run, so a seed always
    /// flips the same way, for you and for your friend.
    flip_rng: Rng,
```

In the `Game { ... }` literal inside `Game::new`, after `rng,` add `flip_rng: Rng::new(1),` (a placeholder; `restart_run()` at the end of `Game::new` replaces it).

In `pick_next`, replace `self.rng.next_u64()` with `self.flip_rng.next_u64()`:

```rust
    fn pick_next(&mut self) -> Dir {
        let options: Vec<Dir> = Dir::ALL.iter().copied().filter(|d| *d != self.gravity).collect();
        options[(self.flip_rng.next_u64() % options.len() as u64) as usize]
    }
```

Make the first line of `restart_run`:

```rust
    fn restart_run(&mut self) {
        self.flip_rng = Rng::new(self.world.seed ^ FLIP_SALT);
        self.player = Player::new(spawn_point());
```

(The rest of `restart_run` is unchanged; it already calls `self.pick_next()` after this point.)

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: `9 passed; 0 failed` (5 seed tests + 4 world/flip tests).

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "Build worlds from seed codes; same code gives the same gravity flips"
```

---

### Task 3: Seed box, HUD code, and title/death text

**Files:**
- Modify: `src/main.rs` — `enum State`, `struct Game` + `Game::new`, `update`, new `open_seed_entry` / `update_seed_entry` / `draw_seed_entry`, `draw` (death overlay lines + match), `draw_hud` (top-right box, early return), `draw_title` (lines).
- Test (not committed): `/tmp/gravity-fs-test/seed-check.mjs`, run with `/tmp/gravity-fs-test/run.sh seed-check.mjs` (it serves `web/` on `127.0.0.1:8792`).

**Interfaces:**
- Consumes: `seed::normalize`, `seed::random_code`, `seed::MAX_LEN` (Task 1); `World::new(code: String)`, `World.code` (Task 2).
- Produces: `State::EnterSeed`, `Game.seed_input: String`, `Game.seed_return: State`, `Game::open_seed_entry(&mut self, from: State)`, `Game::update_seed_entry(&mut self)`, `Game::draw_seed_entry(&self)`.

- [ ] **Step 1: Write the failing end-to-end check**

Create `/tmp/gravity-fs-test/seed-check.mjs`:

```js
// Seed box end to end in the real web build. Screenshots are checked by eye.
import { chromium } from "playwright-core";

const CHROME = `${process.env.HOME}/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;
const browser = await chromium.launch({
    executablePath: CHROME,
    headless: true,
    args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const errors = [];
const open = async () => {
    const page = await browser.newPage({ viewport: { width: 1280, height: 760 } });
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto("http://127.0.0.1:8792/index.html");
    await page.waitForFunction(() => typeof wasm_exports !== "undefined" && wasm_exports, null, { timeout: 20000 });
    await page.evaluate(() => canvas.focus()); // no click: a click on the title starts the game
    await page.waitForTimeout(300);
    return page;
};
const hud = { x: 880, y: 0, width: 400, height: 100 };

// 1. S, type "banana", Enter: the box shows BANANA (not SBANANA), then the HUD names the world.
let page = await open();
await page.keyboard.type("wad"); // typed on the title (no S/B, which are title keys): must not leak into the box
await page.keyboard.press("KeyS");
await page.waitForTimeout(200);
await page.keyboard.type("banana");
await page.waitForTimeout(300);
await page.screenshot({ path: "seed-1-typed.png" });
await page.keyboard.press("Enter");
await page.waitForTimeout(800);
await page.screenshot({ path: "seed-2-hud-banana.png", clip: hud });

// 2. Esc goes back to the title; a blank Enter plays a random 5-character world.
page = await open();
await page.keyboard.press("KeyS");
await page.waitForTimeout(200);
await page.keyboard.press("Escape");
await page.waitForTimeout(300);
await page.screenshot({ path: "seed-3-back-to-title.png" });
await page.keyboard.press("KeyS");
await page.waitForTimeout(200);
await page.keyboard.press("Enter");
await page.waitForTimeout(800);
await page.screenshot({ path: "seed-4-hud-random.png", clip: hud });

console.log("wrote seed-1..4 PNGs in /tmp/gravity-fs-test; page errors:", JSON.stringify(errors));
await browser.close();
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/virenvijaymane/Rust_Projects/gravity && ./build_web.sh && /tmp/gravity-fs-test/run.sh seed-check.mjs`
Then view `/tmp/gravity-fs-test/seed-1-typed.png`.
Expected (failure): no seed box — the title screen is still showing (S does nothing yet), and `seed-2-hud-banana.png` shows a hex `world ...` value, not `BANANA`.

- [ ] **Step 3: Add the state and fields**

In `enum State`, add after `Dead,`:

```rust
    /// Typing a seed code.
    EnterSeed,
```

In `struct Game`, after `shop_message: (String, f32),` add:

```rust
    /// What has been typed in the seed box, and where Esc goes back to.
    seed_input: String,
    seed_return: State,
```

In the `Game { ... }` literal in `Game::new`, after `shop_message: (String::new(), 0.0),` add:

```rust
            seed_input: String::new(),
            seed_return: State::Title,
```

- [ ] **Step 4: Handle input**

Add these methods to `impl Game`, directly after `update_shop`:

```rust
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
```

In `update`, replace the start of the function:

```rust
    fn update(&mut self, dt: f32) {
        self.update_effects(dt);
        // Debug-build cheat for testing the shop: C gives 50 coins.
        if cfg!(debug_assertions) && is_key_pressed(KeyCode::C) {
```

with:

```rust
    fn update(&mut self, dt: f32) {
        self.update_effects(dt);
        // macroquad keeps every typed character until it is read: outside the seed box, throw
        // them away so they don't pile up or spill into the box when it opens (the S included).
        if self.state != State::EnterSeed {
            while get_char_pressed().is_some() {}
        }
        // Debug-build cheat for testing the shop: C gives 50 coins.
        if cfg!(debug_assertions) && self.state != State::EnterSeed && is_key_pressed(KeyCode::C) {
```

In the `State::Title` arm, replace:

```rust
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Title);
                } else if is_key_pressed(KeyCode::Space)
```

with:

```rust
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Title);
                } else if is_key_pressed(KeyCode::S) {
                    self.open_seed_entry(State::Title);
                } else if is_key_pressed(KeyCode::Space)
```

In the `State::Dead` arm, replace:

```rust
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Dead);
                } else if is_key_pressed(KeyCode::R)
```

with:

```rust
                if is_key_pressed(KeyCode::B) {
                    self.open_shop(State::Dead);
                } else if is_key_pressed(KeyCode::S) {
                    self.open_seed_entry(State::Dead);
                } else if is_key_pressed(KeyCode::R)
```

Directly after the line `State::Shop => self.update_shop(),` add:

```rust
            State::EnterSeed => self.update_seed_entry(),
```

- [ ] **Step 5: Draw the seed box**

Add to `impl Game`, directly after `draw_shop`:

```rust
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
```

In `draw`, directly after the line `State::Shop => self.draw_shop(),` add:

```rust
            State::EnterSeed => self.draw_seed_entry(),
```

- [ ] **Step 6: Show the code (HUD, death screen, title)**

In `draw_hud`, replace:

```rust
        // Distance.
        draw_rectangle(sw - 230.0, 8.0, 222.0, 80.0, Color::new(0.0, 0.0, 0.0, 0.45));
        draw_label(&format!("{:.0} m", self.distance()), sw - 220.0, 38.0, 36.0, WHITE);
        draw_label(&format!("BEST {:.0} m", self.best / UNITS_PER_METRE), sw - 220.0, 62.0, 22.0, Color::new(1.0, 0.85, 0.3, 1.0));
        draw_label(
            &format!("world {:x}  deaths {}", self.world.seed & 0xFFFF, self.deaths),
            sw - 220.0,
            80.0,
            16.0,
            GRAY,
        );

        if matches!(self.state, State::Title | State::Shop) {
            return;
        }
```

with:

```rust
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
```

In `draw`, in the `State::Dead => self.draw_overlay(` lines array, replace:

```rust
                    &format!("You made it {:.0} m   (best {:.0} m)", self.distance(), self.best / UNITS_PER_METRE),
                    "",
                    "R / Space - try this world again",
                    "N - brand new world",
```

with:

```rust
                    &format!("You made it {:.0} m   (best {:.0} m)", self.distance(), self.best / UNITS_PER_METRE),
                    &format!("World {} - give this seed to a friend to race the same world", self.world.code),
                    "",
                    "R / Space - try this world again",
                    "N - brand new world",
                    "S - type a seed",
```

In `draw_title`, replace the `lines` array:

```rust
        let lines = [
            "Go right as far as you can - the obby never ends.",
            "Gravity changes direction when the countdown hits zero.",
            "Land too hard and you take fall damage - lose it all and you die.",
            "Don't go AFK for 30 seconds... the AFK monsters are watching.",
            "Grab coins and spend them on power-ups like SPIDER - each lasts one round.",
            "",
            "Move: A/D (or W/S when gravity is sideways)    Jump: Space (again in mid-air to double jump)",
            "",
            "",
            &format!("Press SPACE or click to start      B - shop ({} coins)", self.coins),
        ];
```

with:

```rust
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
```

- [ ] **Step 7: Run unit tests and builds**

Run: `cargo test && cargo build --release && ./build_web.sh`
Expected: `9 passed; 0 failed`; both builds finish with no errors and no new warnings.

- [ ] **Step 8: Run the end-to-end check and look at the screenshots**

Run: `/tmp/gravity-fs-test/run.sh seed-check.mjs`, then view the four PNGs in `/tmp/gravity-fs-test/`.
Expected:
- `seed-1-typed.png`: "ENTER A SEED" box showing exactly `BANANA` (no leading `S`, no `WAD`).
- `seed-2-hud-banana.png`: the top-right box reads `world BANANA  deaths 0`.
- `seed-3-back-to-title.png`: the title screen, including the line `S - type a seed to play the same world as a friend` and `Don't go AFK for 60 seconds...`.
- `seed-4-hud-random.png`: `world` followed by 5 characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ`.
- Page errors: only the pre-existing `register_plugin is not defined`.

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "Add a seed box (S) and show the world's seed code to share with friends"
```

---

## After the tasks

Run the fullscreen regression check from the earlier itch.io fix (`/tmp/gravity-fs-test/run.sh`, expects `all checks passed`) to confirm nothing in the page broke. Deploying (itch.io via butler, GitHub Pages, Netlify, Vercel) is a separate, user-approved step.
