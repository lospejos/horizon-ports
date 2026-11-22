# 0004: worldmap keeps a coarse overview of every tile it has seen

Status: accepted
Context: Zoom steps 1, 2, 4 and 8 blocks a pixel (spec A19) show up to about 6400 tiles at once; memory holds 320 full tiles (ADR 0003, 16 KB each), so zoomed-out views would show only the nearest ones.
Options: only the nearest tiles show when zoomed out; cap zoom at 2; keep a coarse overview of every tile seen.
Decision: overview (user's choice, 2026-11-22). For each tile read from disk or scanned, keep a copy sampled every 2nd column in x and z (32x32 cells, colour id and shade, 2 KB). The map reads full tiles at 1 block a pixel and the overview at 2, 4 and 8 (4 and 8 skip cells). Tiles not yet seen are read from disk in the background, 48 a frame, and dropped after their overview is taken.
Consequences: Opening a zoomed-out view reads the files of all visible tiles once (up to about 100 MB of reads, spread over seconds); overview memory is about 2 KB a tile (about 13 MB at 6400 tiles) and is dropped on a world or dimension change. A tile being scanned shows in the overview when its scan ends.
