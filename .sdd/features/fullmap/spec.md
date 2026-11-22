# fullmap: spec (worldmap step 3, branch worldmap-wasm)

Goal: a fullscreen map on `M` that shows the terrain the player has explored, with pan, zoom, waypoints and teleport, like Xaero's World Map.

Non-goals: caves/layers, entity radar, map sharing, import of other mods' maps, feeding the minimap from the cache, 3D markers.

Acceptance criteria:
- A15: Key `M` (default; rebindable) opens and closes the map screen; M, Esc and a Close button close it. Controls check: no other binding on M (spec rule).
- A16: The map draws every explored tile in view, not only the minimap area; unknown area is dark.
- A17: Explorer scan: at most 2048 columns per tick; an unknown tile is scanned once (nearest first); a known tile is rescanned only near the player and when older than 30 s (ADR 0003).
- A18: Explored tiles persist per world and dimension and survive a restart; broken or missing files give empty tiles.
- A19: Drag with the mouse pans, wheel zooms (1, 2, 4, 8 blocks per pixel), a button or key centres on the player.
- A20: Waypoints of the dimension are drawn with icon and colour; a click opens the edit screen; a right click makes a waypoint at that X Z (Y = stored surface height + 1).
- A21: The cursor shows X Z and the surface Y of the stored column; biome only where the host can read it.
- A22: A click on a point or waypoint offers teleport (`/tp @s`, as T15), same dimension only.

Constraints: wasm only (ADR 0002); data per ADR 0003; no local cargo, verify in CI; commit yes, push on the task's CI check, no tag without the user's word. The map image is at most 1024 px a side (`MOD_IMAGE_SIZE`).

T1 probe (SDK 2b77287 source/docs, not run in game):
- A18: `data_files` cap is `DATA_FILE_MAX` = 64 MiB per file (config root 4 MiB); a 16 KB tile is far under it. `list` pages at most `CONFIG_LIST_MAX` = 1024 entries; `write` is atomic.
- A16: a mod screen draws in `screen_render_post` with `native_sprite`, which draws the `mod_image` slot (1024x1024 max, one image shared by all mods; the minimap's own image is replaced while the map is open). Not drawn in game yet: T5 confirms live.
- A21: `client_biome` answers only for a loaded selected column (None otherwise, never loads chunks); `client_far_surface` returns height/colour, no biome. So the biome shows only inside loaded chunks, else the line is omitted.
- A17 detail (T3): the explorer works one 64x64 tile at a time (2 ticks at 2048 columns), looks `RADIUS` 7 tiles around the player's tile, "near" is 2 tiles, memory holds 256 tiles; the store is dropped when the world or dimension changes (disk in T4); a column the host cannot read keeps its stored value. The minimap scan stays separate (its own 4096 columns a tick).
