# Worldmap for Horizon

A minimap of the terrain around the player, drawn over the game's HUD.

An original mod for [Horizon](https://github.com/horizon-rust/horizon) 26.3, written from scratch under the MIT licence (`LICENSE`; see `NOTICE` for the mods that inspired it).

A WebAssembly mod (`mod.wasm`, no mixins), built with `cargo horizon-mod build --wasm` against Horizon's exact-build SDK `2b77287de099c9316278ba7ac4d83590fdb7ef30`; the `worldmap-wasm` workflow builds `worldmap.hmod` and attaches it to a release for a tag `worldmap-wasm-v*`.

The map shows the terrain colours (the game's map colours) with water shaded by depth and slopes lit or dark, a white marker at the player with a red tip where the player looks, and optional lines under it. It reads the far terrain, so **LOD Distance must be on**, or the map stays empty.

Options > Mods > Worldmap > Config: show the map, corner, offsets X and Y from that corner (to move it below another HUD element), size (64 to 256 pixels), blocks per cell (1, 2 or 4), shape (square or round), and the lines under the map: coordinates, biome, time of day, light level (block / sky) and dimension.

Waypoints: press N (rebindable in Options > Controls) for the list. Each waypoint has a name, coordinates, colour (palette or RRGGBB) and icon; it shows on the minimap (stuck to the edge when out of range, hidden in other dimensions), and is saved per world. The list can hide, sort (age, name, distance), edit, delete and teleport (`/tp`, same dimension, needs permission).

Fullscreen map: press M (rebindable in Options > Controls). It draws every area you have explored in this world and dimension, not only what the minimap shows; unknown area is dark. Drag to pan, scroll to zoom (1, 2, 4 or 8 blocks a pixel), Centre returns to you. Waypoints show on it: click one to edit it, right click to start a new one at that spot. The top left shows X, Z, the surface Y and, where the chunk is loaded, the biome under the cursor. A click marks a point; Teleport then sends `/tp @s` there (same dimension, needs permission, lands one block above the stored surface). M, Esc or Close shuts the map.

The explored terrain is a cache: while you play, the mod scans the ground around you into 64 x 64 block tiles and saves them (every few seconds and when you leave the world) under the mod's data folder, per world and dimension, so the map survives a restart. A tile file is 16 KB, so about 4 MB per 1000 x 1000 blocks explored and about 400 MB per 10000 x 10000. Delete the `tiles` folder in the mod's data folder to reset it; a broken file is ignored and scanned again.

Not done yet: radar, cave mode, sharing maps. The minimap image moves in small jumps while you walk, as it is rebuilt a few times a second.

## Installing and updating

Download the release into the game's mods folder (the `horizon-mods` folder of the instance or of the launcher's data directory; `--clobber` replaces the old file), then restart the game:

```powershell
gh release download worldmap-wasm-v0.3.1 -R lospejos/horizon-ports -p "*.hmod" -D "C:\Users\User\AppData\Roaming\horizon\horizon-mods" --clobber
```

For a newer version, change only the tag. `gh release list -R lospejos/horizon-ports` lists the tags. Or download `worldmap.hmod` from the release page by hand.
