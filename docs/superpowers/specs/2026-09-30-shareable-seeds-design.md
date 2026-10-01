# Shareable world seeds

## Goal

Let two friends play the exact same world: every world has a short seed code you can read off the
screen, and you can type a code (or any word) to play that world. Leaving it blank gives a random
world.

## Player experience

- **See it.** The top-right box shows the current world's code (`world K7Q2X  deaths 3`) while
  playing, on the title screen, and on the death screen. The death screen also says
  `World K7Q2X - give this seed to a friend to race the same world`.
- **Set it.** On the title screen or the death screen, press **S** to open the seed box:
  - letters and digits are typed in (shown upper case, at most 12 characters);
  - **Backspace** deletes;
  - **Enter** starts a run in that world — with the box left blank, a new random world;
  - **Esc** goes back to the screen you came from.
- **Random codes** are 5 characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ` (no 0/O or 1/I/L), so
  they are easy to read out. Typed codes may use any letter or digit, so `BANANA` is a world too.
- **Same code, same course:** the same platforms, coins, and the same order of gravity flips.
  When flips happen still depends on how far you get, as today.
- Unchanged: **R** retries the same world, **N** makes a new random world (its code is shown),
  **Space** on the title plays the world already shown.

## Design

### `src/seed.rs` (new, pure functions)

- `normalize(text: &str) -> String` — upper-cases, keeps only ASCII letters and digits, truncates
  to `MAX_LEN = 12`.
- `random_code(bits: u64) -> String` — `CODE_LEN = 5` characters from the alphabet above.
- `world_seed(code: &str) -> u64` — FNV-1a (64-bit) over the code's bytes, then the splitmix64
  finalizer. Integer-only, so the web and desktop builds agree. Pinned by a golden-value test:
  changing it would silently break codes people have already shared. Callers always pass a code
  that is already normalized (typed input goes through `normalize`; `random_code` output already is).

### World

`World` gains `code: String`; `World::new` takes the code and derives its seed with
`seed::world_seed`. The startup world and `N` both use `seed::random_code(self.rng.next_u64())`.

### Gravity flips

`Game` gains `flip_rng: Rng`, reset from the world seed (XOR a fixed salt) in `restart_run()`.
`pick_next()` draws from `flip_rng` instead of the game-wide `rng`, so every run of a world sees
the same flip order.

### Seed entry

- `State::EnterSeed`, plus `seed_input: String` and `seed_return: State` (mirrors the shop's
  `shop_return`).
- Opening the box clears `seed_input` and drains macroquad's character queue, so the `S` that
  opened it is not typed in.
- Typing reads `get_char_pressed()`; only characters that survive `normalize` are added.
- The debug-build coin cheat (`C`) must not fire while typing a seed.
- `draw_hud` returns early for `EnterSeed`, like it does for the title and the shop.

### Text changes

- Title: add `S - type a seed to play the same world as a friend`. While editing these lines, the
  AFK line says 30 seconds but the monsters come after 60 (`AFK_TIME`); make the text say 60.
- Death screen: add the `World ...` share line and `S - type a seed`.

## Out of scope

Share links / URL parameters (they would not reach the game inside itch.io's frame), copying to
the clipboard, best distance per seed, and the randomness of AFK monsters, dust and background
stars (cosmetic).

## Testing

- Unit tests (`cargo test`): `normalize`, `random_code` (length, alphabet only), `world_seed`
  (deterministic, distinct codes differ, golden value).
- Same code → identical solids and coins for the first chunks; different codes → different
  layouts; same code → same first flips after `restart_run()`.
- Browser check with the existing headless-Chromium harness: press S, type `banana`, Enter, and
  confirm the HUD shows `BANANA` (screenshot) and that the opening `S` was not typed.
- `cargo build --release --target wasm32-unknown-unknown` succeeds.
