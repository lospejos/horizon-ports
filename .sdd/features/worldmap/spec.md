# worldmap: spec (this feature = step 1, the minimap)

Goal: MIT wasm client mod `worldmap` for Horizon 26.3 that draws a live minimap of the terrain around the player.

Non-goals (later features, same mod): fullscreen map, waypoints, radar, cave mode, far LOD, persistent cache, dimension tabs, 3D markers.

Acceptance criteria:
- A1: CI builds `worldmap.hmod`; the game loads it with no refusal.
- A2: Minimap shows terrain colours (map colours) around the player and updates while the player moves.
- A3: Water and height differences are visible (shading).
- A4: Settings: enabled, corner, size, blocks per pixel, shape (square/round); changes apply without a restart.
- A5: Player marker and coordinates are shown.
- A6: Sampling is budgeted per tick (at most 4096 columns); user sees no stutter.
- A7: LICENSE is MIT; NOTICE credits Eternity World Map ideas; no GPL code (ADR 0001).
- A8: README has install and update lines; package icon present.

Constraints: wasm only, no mixins (ADR 0002). SDK pin 2b77287de099c9316278ba7ac4d83590fdb7ef30. No local cargo: verify in CI. No commit, push or tag without the user's word.
