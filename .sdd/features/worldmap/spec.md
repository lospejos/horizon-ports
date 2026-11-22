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
- A9: The map position is custom: corner plus offsets X and Y (so it can move below the item popup at the top right); applies without a restart. A drag-to-move editor is a later feature.
- A10: Optional info lines under the map, like Xaero's Minimap: coordinates, biome, time of day, light level, dimension; each is a toggle. Only what the host API can read.

Constraints: wasm only, no mixins (ADR 0002). SDK pin 2b77287de099c9316278ba7ac4d83590fdb7ef30. No local cargo: verify in CI. No commit, push or tag without the user's word.
- Block names come from client_far_surface (TerrainColumn: block, height, water), since no client block read exists; far terrain (LOD Distance) must be on, else columns stay empty. Map colour: block_state_id(name) then block_state_map_color.
- Map is 128 px square at 1 block per pixel, top right, rebuilt as one pass of 4096 columns per tick and uploaded when a pass ends (MOD_IMAGE_SIZE is 1024); water and shading come in T4, settings in T5.
- In game T3 check (2026-11-22): terrain shows and follows the player, but the image moves in jumps (refresh ~5 per second, no offset between refreshes); lakes show as ocean-floor grey until T4.
- T4 check (2026-11-22): user sees sea (depth shades) and slopes in game.
- Settings (T5): enabled, corner (4), size 64-256 px step 16, scale 1\/2\/4 blocks per cell, shape square\/round; grid stays 128x128 cells. Scale and shape apply from the next pass (about 8 ticks), others at once. Round shape clears corner pixels in the buffer.
- T5 check (2026-11-22, agent in game via horizon MCP, test world worldmap-test): corner, size, scale 4, shape round and enabled off each changed the live map; settings reset to defaults afterwards. Settings page: Options > Mods > Worldmap > Config.
- T6: marker is a white square with a red tip in the look direction, placed at (camera - pass centre) so it moves between passes; coordinates (x y z, floored) sit under the map for top corners and above it for bottom corners. T8 turns the coordinates into one of the toggled info lines.
- T6 check (2026-11-22, agent in game): marker with look tip and coordinates visible under the map.
- T7: offset_x and offset_y (0-640 and 0-360 GUI px, default 4) count inward from the chosen corner; the map is clamped to stay on screen.
- T7 check (2026-11-22, agent in game): offsets 64 and 60 moved the map and its text; reset to 4 afterwards.
