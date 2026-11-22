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
- T8: toggles info_coords (default on), info_biome, info_time (clock, 0 ticks = 06:00), info_light (block / sky at the camera block), info_dimension, off by default but coords; lines stack under the map for top corners, above it for bottom ones. Biome and dimension names use the game's translation, else the id made readable. Not done: weather, FPS, speed.
- T8 check (2026-11-22, agent in game): all five lines shown (169 71 161, Forest, 07:28, Light 0 / 15, Overworld); toggles reset afterwards.
- T9 check (2026-11-22): CI green on abaf385f; worldmap.hmod holds icon.png (crop of the map itself) and version 0.1.0. Not tagged: a tag needs the user's word (`worldmap-wasm-v0.1.0`).
- Final audit (2026-11-22): A1-A10 met and checked in CI or in game (A7: LICENSE MIT, NOTICE credits Eternity World Map ideas; ADR 0001 and 0002 followed). Not verified: ru lang in game, settings kept after a restart, icon shown in the launcher.
- Follow-ups (not in this plan): jerky scroll (draw the image offset by camera minus pass centre each frame, needs clipping); weather/FPS info lines; drag-to-move editor; fullscreen map, waypoints, radar (non-goals of step 1).

## Step 2: waypoints (planned, tasks T10-T15 in plan.md)

Goal: waypoints like Xaero's Minimap for Minecraft Java Fabric.

Acceptance criteria:
- A11: The user can create a waypoint (name, x y z, dimension), edit it and delete it; waypoints persist per world across restarts.
- A12: Waypoints show on the minimap as markers; out-of-range ones stick to the map edge; other dimensions are hidden.
- A13: Each waypoint has a custom icon (built-in set) and colour; both changeable after creation.
- A14: The user can teleport to a waypoint from the list or the edit screen. Needs the host API to run a command or move the player and permission to do so (cheats/op); without it the mod shows a message. To check: what the wasm API offers for commands, key binds, text input and file storage.
- T10: waypoints live in `waypoints.rs` (pure). File `horizon-mod-data/worldmap/waypoints-<safe key>-<hash>.txt` via `Host::data_files()`; key is `client_server().save_folder`, else the server address. Text format: header `worldmap-waypoints 1`, then one tab-separated line per waypoint (name, x, y, z, dimension, RRGGBB, icon, visible), `\ \t \n \r` escaped. Loaded when the world key changes; a missing or broken file gives an empty list. Saving is called by the T11 UI.
- T11: key `waypoints` (default N, user's choice) opens the waypoint screen (`ClientHost::mod_screen_open`): arrows browse, name and x/y/z fields, New, Save (greyed while a coordinate is not a whole number), Delete (needs a second press, "Sure?"), Close; Save writes the file at once. A new waypoint gets the player's position, the current dimension and colour FF8800; an edit keeps dimension, colour, icon and visibility. B was the default first but is the game's Emote Wheel. The list screen (T14) replaces the arrows.
- Key plan (user, 2026-11-22): N = create waypoint (T11), M = open the world map (fullscreen map, not yet in the plan; keep M free).
- Test rule (user, 2026-11-22): every task that adds a key checks in the game, Options > Controls (page keybinds), that the default is not bound to another command or mod. T11 check: Worldmap > Waypoints = N; no other binding on N except the debug combo F3+N; M free.
- T11 check (2026-11-22, agent in game): N opened the screen; made Home and Mine, edited Mine's X to 500 (Save greyed while the field was empty), browsed with <, deleted Home with two presses; file `horizon-mod-data/worldmap/waypoints-worldmap-test-<hash>.txt` held only Mine afterwards. Not checked: restart keeps the list, ru texts, a server world.
- T12: visible waypoints of the current dimension draw on the minimap as a 6 px square (black border, waypoint colour), placed like the player marker (relative to the pass centre, so they move between passes). Out of range ones stick to the map edge (4 px inset; round maps pull them in along the line to the centre). The player marker is drawn on top. Icons come in T13.
- T12 check (2026-11-22, agent in game, CI run 45062206186): a waypoint made with N shows as an orange square 4 blocks north of the player; the old "Mine" at X=500 sticks to the right edge; in the Nether no marker is drawn. Not checked: round shape clamp in game, scale 2/4. No key added, so no Controls check.
