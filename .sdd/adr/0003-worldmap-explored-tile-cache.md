# 0003: worldmap remembers explored terrain in tiles

Status: accepted
Context: The minimap reads live terrain around the player and keeps nothing. A fullscreen map must show far more than the LOD radius, and scanning everything live every frame is wasted load: generated terrain rarely changes.
Options: live view only (map = LOD radius, no memory); a persistent explored-tile cache fed by a budgeted scan.
Decision: cache (user's reasoning, 2026-11-22). Tiles of 64x64 columns, 4 bytes a column (map colour id, brightness code, surface height i16), one file per tile under `horizon-mod-data/worldmap/` in a folder per world key and dimension, through `Host::data_files()`. An unknown tile is scanned once, nearest to the player first; a known tile is rescanned only while the player is near and it is older than 30 s. Tiles load on demand into a memory LRU and are written when dirty (at most every few seconds, and on close).
Consequences: Disk use grows with exploration (about 16 KB raw per 64x64 block area). The minimap keeps its own live scan for now (follow-up: feed it from the cache). The fullscreen image goes through the one shared `mod_image` slot, so the minimap redraws after the map closes. Needs the per-file size cap from `data_files` (probe in T1).
