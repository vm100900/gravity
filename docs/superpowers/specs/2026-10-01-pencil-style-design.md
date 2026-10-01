# Pencil-on-paper look

## Goal

Make Gravity look hand drawn: graphite pencil on white paper, sketchy lines, handwritten text.

## Decisions

- Pencil on white paper.
- Strictly black and white: every colour is a grey (`r = g = b`); only alpha varies.
- Handwritten text everywhere, in Patrick Hand (SIL Open Font License 1.1), bundled into the game.
- Still lines: sketchy, but each shape is drawn identically every frame (no "line boil").
- Graph-paper background: the existing 100-unit grid becomes faint pencil lines.

## Palette (`src/sketch.rs`)

| Name | Grey | Used for |
|------|------|----------|
| `PAPER` | 0.95 | background, cards, eye whites, head fill |
| `INK` | 0.13 | outlines, stickman, main text |
| `GRAPHITE` | 0.32 | floor/ceiling and bump hatching, monsters |
| `SHADE` | 0.58 | platform hatching, secondary text, dust |
| `FAINT` | 0.84 | graph-paper grid, background flecks |

## What players see

- **Paper:** `PAPER` background with faint grain (a small tiling noise texture made at startup),
  anchored to the world so it scrolls with the level. The graph grid is drawn in `FAINT`.
- **Gravity flecks** (today's drifting "stars"): short `FAINT` pencil dashes, still drifting with
  gravity in screen space.
- **Distance markers and BEST line:** pencil lines in `SHADE` with handwritten labels; BEST is a
  dashed `INK` line whose alpha pulses.
- **Floor and ceiling:** `GRAPHITE` cross-hatching plus a double `INK` stroke on the inner face.
- **Platforms (both orientations):** sketchy double-stroked `INK` outline with slight overshoot at
  the corners, plus `SHADE` hatching at 45°.
- **Bumps:** `INK` outline plus denser `GRAPHITE` cross-hatching, so they read as obstacles.
- **Coins:** pencil ellipse (squashed to spin, as now), inner ellipse, two short shine ticks.
- **Spider webs:** thin `SHADE` strands; the spider in `INK`.
- **Stickman:** `INK` strokes; head filled with `PAPER` and outlined. Hurt: lines thicken and the
  figure shakes while `hurt_flash > 0`. Low health (< 30): a constant small shake replaces the
  red flicker. (Only the figure's position shakes, as a damage signal; its pencil strokes keep
  their shape, so this doesn't contradict "still lines".)
- **AFK monsters:** ghost silhouette filled with dense `GRAPHITE` cross-hatching and an `INK`
  outline; `PAPER` eyes with `INK` pupils that track you and angry brows; handwritten "AFK".
- **Popups:** handwritten; damage numbers bigger and doubled 1 px apart so they look pressed hard.
- **Dust:** small `SHADE` pencil specks.
- **HUD:**
  - Health: handwritten "HEALTH", pencil box, `INK` hatching filling it up to the current health.
  - Top-right box: a paper card (`PAPER` fill, pencil outline) with distance, best and the
    `world CODE  deaths N` line (the box keeps widening for long seed codes).
  - Flip countdown: paper card, handwritten number, pencil arrow and direction. Under 3 s the
    number shakes and the screen edge gravity will turn towards fills with `GRAPHITE` hatching
    whose strength pulses (replaces the red glow).
  - Gravity indicator: handwritten "GRAVITY" and a pencil arrow.
  - Coins: pencil coin icon and count; owned power-ups by name (with the spider recharge timer).
  - AFK banners: paper card with handwritten text.
  - Gravity flip: a brief `SHADE` smudge over the screen (replaces the white flash, which is
    invisible on paper) plus the existing shake and the big handwritten "GRAVITY: X".
  - Controls hint: handwritten, `SHADE`.
- **Screens:** the world is washed with `PAPER` at 85% opacity; content sits on paper cards with
  sketchy outlines.
  - Title: big handwritten "GRAVITY" with today's wobble.
  - Shop: three sketched cards with pencil doodles instead of coloured borders — a spider, a
    magnet, a bone. An owned card is lightly hatched and says READY; text you can't afford yet is
    `SHADE`.
  - Death: big handwritten "YOU DIED" with a scribbled underline.
  - Seed box: pencil-outlined box with a blinking pencil cursor.

## Code structure

- **`src/sketch.rs` (new).** Palette constants, and pencil drawing built on pure, testable geometry:
  - `jitter(seed: u64, i: u32) -> f32` in `[-1, 1]`, deterministic.
  - `seed_of(rect: Rect) -> u64` — from the rect's coordinates, so a world shape always wobbles the
    same way (and the same for everyone on that seed).
  - `hatch_lines(rect: Rect, spacing: f32, angle: f32) -> Vec<(Vec2, Vec2)>` — parallel segments
    clipped to the rect.
  - `stroke_points(a: Vec2, b: Vec2, seed: u64) -> Vec<Vec2>` — a wobbly polyline with overshoot.
  - Drawing: `pencil_line`, `pencil_rect`, `hatch_rect`, `cross_hatch_rect`, `pencil_circle`,
    `pencil_ellipse`, `pencil_arrow`, and the shop doodles `doodle_spider`, `doodle_magnet`,
    `doodle_bone`.
- **`src/render.rs` (new).** Every drawing method moves out of `main.rs` (`draw`,
  `draw_background`, `draw_world`, `draw_web`, `draw_monster`, `draw_stickman`, `draw_hud`,
  `draw_title`, `draw_shop`, `draw_seed_entry`, `draw_overlay`) together with the text helpers
  (`FONT_SIZES`, `warm_font_cache`, `font_size`, `draw_label`, `text_centered`) and `draw_arrow`.
  `main.rs` keeps the game logic.
- **Font.** `assets/PatrickHand-Regular.ttf` and its `assets/OFL.txt`, from
  `github.com/google/fonts/tree/main/ofl/patrickhand`. Loaded once with
  `load_ttf_font_from_bytes(include_bytes!(...))` and kept in a global. `draw_label`,
  `text_centered`, every `measure_text`, and the title's `draw_text_ex` use it. The warm-up caches
  each `FONT_SIZES` entry at `(size × dpi_scale()).ceil()` — the size macroquad actually looks up —
  fixing the mismatch on high-density screens. The game download grows by about 210 KB.

## Out of scope

Line boil or other animation of strokes, any colour, sound, gameplay changes, and layout changes
beyond what the new font's metrics require.

## Testing

- Unit tests for `sketch.rs`: hatch segments lie inside their rect; the number of segments follows
  the spacing; `jitter` is deterministic and within `[-1, 1]`; `seed_of` is stable.
- The existing 9 tests, the fullscreen regression check and the seed check still pass.
- The new font's glyph texture stays at or below 4096 × 4096 at 1× and 2× pixel density.
- Screenshots of the web build (headless Chromium) at 1× and 2×: title, gameplay including an
  urgent countdown, shop, death screen and seed box — reviewed and tuned, a few shown to the user.
- Desktop and web builds succeed; the `GRAVITY_SHOT` screenshot hook still works.
- A rough frame-time check in the headless browser (software rendering) as a performance sanity
  check.
