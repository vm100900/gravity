# Line boil

## Goal

Make the pencil strokes redraw slightly several times a second, like a hand-drawn cartoon
("line boil"). This replaces the "still lines" decision in `2026-10-01-pencil-style-design.md`.

## Decisions

- Always on (no toggle).
- 8 redraws per second, cycling through 3 drawings: the classic boil loop.
- Every pencil stroke boils: outlines, hatching, scribbles, arcs/circles, arrows, doodles, the
  stickman, HUD and menu cards. Still: the printed graph-paper grid, the paper grain, and text.
- Drawing 0 is exactly today's look.

## Design (`src/sketch.rs`, one call in `src/render.rs`)

- Pure, tested:
  - `boil_drawing(time: f64) -> u64` — `(time × 8) as u64 % 3`.
  - `boil_seed(seed: u64, drawing: u64) -> u64` — `seed ^ drawing × 0x9E37_79B9_7F4A_7C15`
    (wrapping), so drawing 0 leaves the seed unchanged.
- The current drawing lives in a `static BOIL: AtomicU64`, set once per frame by
  `set_boil(drawing)` at the start of `Game::draw` (`set_boil(boil_drawing(get_time()))`).
- The stroke functions that consume a seed — `pencil_line`, `hatch_rect`, `pencil_ellipse`,
  `pencil_arc`, `scribble_rect`, `scribble_circle` — use `boil_seed(seed, current)`. Everything else
  (rects, cross-hatching, circles, arrows, doodles) goes through those, so it boils too. The paper
  grain calls `jitter` directly and is unaffected.

## Testing

- Unit tests: `boil_drawing` gives 0, 1, 2, 0 at 1/8-second steps; `boil_seed(s, 0) == s`;
  drawings 0, 1 and 2 give three different seeds; a stroke's points differ between drawings.
- Browser check: on the shop screen, take ~20 screenshots in quick succession and compare only the
  dark ink of the spider doodle (pixels darker than 150, so faint background flecks don't count):
  exactly 3 distinct versions appear.
- All screens stay strictly grey; existing tests pass.
