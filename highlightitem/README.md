# HighLightItem for Horizon

Highlights inventory items similar to the one hovered over.

A port of [HighLightItem](https://github.com/elmital/HighLightItem) 3.1.0 by elmital to [Horizon](https://github.com/horizon-rust/horizon) 26.3, under the original's licence (GPL-3.0-only, in `LICENSE`; see `NOTICE`).

A WebAssembly mod (`mod.wasm`, no mixins), built with `cargo horizon-mod build --wasm` against Horizon's exact-build SDK `2b77287de099c9316278ba7ac4d83590fdb7ef30`; the `highlightitem-wasm` workflow builds `highlightitem.hmod` and attaches it to a release for a tag `highlightitem-wasm-v*`.

Options > Mods > HighLightItem: on/off, comparator (material; material and amount; material and NBT; material, NBT and amount; name; name and amount; namespace), where it highlights (everywhere, not in creative, player inventory, player inventory without creative, containers only), how the hovered slot is drawn (the game's own, coloured, coloured when not empty), colour and opacity. Controls: keys to toggle and to cycle the comparator, unbound by default.

Not ported: the colour-picker screen, the `/highlightitem` commands, ModMenu, notifications, the key that colours the hovered slot. Needs the player's inventory screen not covered by the recipe book (nothing is drawn then).

## Installing and updating

Download the release into the game's mods folder (the `horizon-mods` folder of the instance or of the launcher's data directory; `--clobber` replaces the old file), then restart the game:

```powershell
gh release download highlightitem-wasm-v0.1.1 -R lospejos/horizon-ports -p "*.hmod" -D "C:\Users\User\AppData\Roaming\horizon\horizon-mods" --clobber
```

For a newer version, change only the tag. `gh release list -R lospejos/horizon-ports` lists the tags. Or download `highlightitem.hmod` from the release page by hand.

To follow the latest CI build instead (branch builds without a release too), use [horizon-mod-updater](https://github.com/lospejos/horizon-mod-updater): `.\update_mod.ps1 -Mod highlightitem`.
