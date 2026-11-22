# Veinminer for Horizon

Mine one ore and the whole vein goes with it.

A port of [Veinminer](https://github.com/MiraculixxT/Veinminer) 2.x by MiraculixxT to [Horizon](https://github.com/horizon-rust/horizon) 26.3, under the original's licence (AGPL-3.0-only, in `LICENSE`; see `NOTICE`).

A WebAssembly mod (`mod.wasm`, no mixins), built with `cargo horizon-mod build --wasm` against Horizon's exact-build SDK `2b77287de099c9316278ba7ac4d83590fdb7ef30`; the `veinminer-wasm` workflow builds `veinminer.hmod` and attaches it to a release for a tag `veinminer-wasm-v*`.

Breaking an ore with a pickaxe (survival only) breaks the connected ores of the same group, as the original's default "Ores" group does (`#c:ores`: the vanilla ores and ancient debris). The vein is the original's layered flood fill: blocks within the search radius of the previous ones, nearest layer first, up to the chain limit. The origin block breaks as usual; the others drop what the held tool would drop (Fortune and Silk Touch count) and vanish, and the pickaxe wears one point per extra block.

Options > Mods > Veinminer (server side too: the settings of the world's host): must sneak (off), needs the correct tool (on), merge item drops at the mined block (off), wear the tool (on), chain limit (100), search radius (1 to 5, default 1), cooldown in ticks (20).

Not ported: the `/veinminer` commands and editing groups (the only group is "Ores"), mining speed and hunger changes, the flat, tunnel and stairs shapes, the client addon (hotkey, preview), permissions, the enchantment, the delay between blocks, the update checker. Mined veins give no experience orbs (the drops are loot-table drops).

## Installing and updating

Download the release into the game's mods folder (the `horizon-mods` folder of the instance or of the launcher's data directory; `--clobber` replaces the old file), then restart the game:

```powershell
gh release download veinminer-wasm-v0.1.1 -R lospejos/horizon-ports -p "*.hmod" -D "C:\Users\User\AppData\Roaming\horizon\horizon-mods" --clobber
```

For a newer version, change only the tag. `gh release list -R lospejos/horizon-ports` lists the tags. Or download `veinminer.hmod` from the release page by hand.
